use crate::error::ComfyUiError;
use serde_json::Value;

/// One file produced by a workflow's save node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputImage {
  pub filename: String,
  pub subfolder: String,
  /// "output", "temp" or "input".
  pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobState {
  /// Not in the history yet, or still running.
  Pending,
  Done(Vec<OutputImage>),
  Failed(String),
}

/// Interpret the body of `GET /history/{prompt_id}`.
pub fn parse_history(prompt_id: &str, body: &Value) -> Result<JobState, ComfyUiError> {
  let Some(entry) = body.get(prompt_id) else {
    return Ok(JobState::Pending);
  };

  let status = entry.get("status");
  let status_str = status.and_then(|s| s.get("status_str")).and_then(Value::as_str);
  let completed = status.and_then(|s| s.get("completed")).and_then(Value::as_bool);

  if status_str == Some("error") {
    return Ok(JobState::Failed(error_message(status)));
  }

  let outputs = entry.get("outputs").and_then(Value::as_object);
  let mut images = Vec::new();
  for node in outputs.into_iter().flat_map(|o| o.values()) {
    // Pictures and video are listed under "images"; audio saved by SaveAudio under "audio".
    let files = ["images", "audio"].into_iter()
      .filter_map(|key| node.get(key).and_then(Value::as_array))
      .flatten();
    for image in files {
      let filename = image.get("filename").and_then(Value::as_str)
        .ok_or_else(|| ComfyUiError::BadResponse("output image without filename".into()))?;
      images.push(OutputImage {
        filename: filename.to_string(),
        subfolder: image.get("subfolder").and_then(Value::as_str).unwrap_or("").to_string(),
        kind: image.get("type").and_then(Value::as_str).unwrap_or("output").to_string(),
      });
    }
  }

  if completed == Some(true) || status_str == Some("success") {
    return Ok(JobState::Done(images));
  }
  Ok(JobState::Pending)
}

/// Whether `prompt_id` is among the running or pending jobs in a `GET /queue` body.
/// Each entry is `[number, prompt_id, workflow, ...]`.
pub fn queue_has_prompt(body: &Value, prompt_id: &str) -> bool {
  ["queue_running", "queue_pending"].iter()
    .filter_map(|key| body.get(key).and_then(Value::as_array))
    .flatten()
    .filter_map(|entry| entry.as_array())
    .any(|entry| entry.get(1).and_then(Value::as_str) == Some(prompt_id))
}

fn error_message(status: Option<&Value>) -> String {
  status
    .and_then(|s| s.get("messages"))
    .and_then(Value::as_array)
    .into_iter()
    .flatten()
    .filter_map(|m| m.as_array())
    .find(|m| m.first().and_then(Value::as_str) == Some("execution_error"))
    .and_then(|m| m.get(1))
    .and_then(|d| d.get("exception_message"))
    .and_then(Value::as_str)
    .unwrap_or("execution error")
    .trim()
    .to_string()
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn missing_entry_is_pending() {
    assert_eq!(parse_history("abc", &json!({})).unwrap(), JobState::Pending);
  }

  #[test]
  fn completed_entry_lists_images() {
    let body = json!({"abc": {
      "outputs": {"9": {"images": [
        {"filename": "a_00001_.png", "subfolder": "", "type": "output"},
        {"filename": "a_00002_.png", "subfolder": "x", "type": "output"}
      ]}},
      "status": {"status_str": "success", "completed": true, "messages": []}
    }});
    let JobState::Done(images) = parse_history("abc", &body).unwrap() else { panic!("not done") };
    assert_eq!(images.len(), 2);
    assert_eq!(images[1].subfolder, "x");
  }

  #[test]
  fn audio_files_are_listed_as_outputs() {
    let body = json!({"abc": {
      "outputs": {"106": {"audio": [{"filename": "song_00001_.mp3", "subfolder": "audio", "type": "output"}]}},
      "status": {"status_str": "success", "completed": true, "messages": []}
    }});
    let JobState::Done(files) = parse_history("abc", &body).unwrap() else { panic!("not done") };
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].filename, "song_00001_.mp3");
    assert_eq!(files[0].subfolder, "audio");
  }

  #[test]
  fn error_entry_reports_exception_message() {
    let body = json!({"abc": {
      "outputs": {},
      "status": {"status_str": "error", "completed": false, "messages": [
        ["execution_start", {}],
        ["execution_error", {"exception_message": "Out of memory\n"}]
      ]}
    }});
    assert_eq!(parse_history("abc", &body).unwrap(), JobState::Failed("Out of memory".into()));
  }

  #[test]
  fn finds_a_job_in_the_running_or_pending_queue() {
    let body = json!({
      "queue_running": [[0, "run-id", {}, {}, []]],
      "queue_pending": [[1, "wait-id", {}, {}, []]]
    });
    assert!(queue_has_prompt(&body, "run-id"));
    assert!(queue_has_prompt(&body, "wait-id"));
    assert!(!queue_has_prompt(&body, "gone-id"));
  }

  #[test]
  fn an_empty_or_odd_queue_has_no_jobs() {
    assert!(!queue_has_prompt(&json!({"queue_running": [], "queue_pending": []}), "x"));
    assert!(!queue_has_prompt(&json!({}), "x"));
    assert!(!queue_has_prompt(&json!({"queue_running": "nope"}), "x"));
  }

  #[test]
  fn running_entry_is_pending() {
    let body = json!({"abc": {"outputs": {}, "status": {"status_str": "running", "completed": false}}});
    assert_eq!(parse_history("abc", &body).unwrap(), JobState::Pending);
  }
}
