//! Replicate's HTTP API: start a prediction, check on it, and upload an input file.
//! Official models are started by name, with no version. Shapes follow Replicate's HTTP reference.

use reqwest::multipart::{Form, Part};
use serde_json::{json, Value};

const API_BASE: &str = "https://api.replicate.com/v1";

/// Replicate recommends inline `data:` pictures only up to this size; larger ones are uploaded.
pub const MAX_INLINE_BYTES: usize = 256 * 1024;

#[derive(Debug, PartialEq)]
pub struct Created {
  pub id: String,
  /// Where to ask how the prediction is going.
  pub get_url: String,
}

#[derive(Debug, PartialEq)]
pub enum PredictionState {
  Running,
  /// Every file the model returned.
  Done(Vec<String>),
  Failed(String),
}

pub async fn create_prediction(api_key: &str, model: &str, input: Value) -> Result<Created, String> {
  let response = reqwest::Client::new()
    .post(format!("{}/models/{}/predictions", API_BASE, model))
    .bearer_auth(api_key)
    .json(&json!({ "input": input }))
    .send().await
    .map_err(|err| format!("Could not reach Replicate: {}", err))?;
  let status = response.status();
  let body: Value = response.json().await.unwrap_or(Value::Null);
  if !status.is_success() {
    return Err(friendly_error(status.as_u16(), &body));
  }
  parse_created(&body)
}

pub async fn get_prediction(api_key: &str, get_url: &str) -> Result<PredictionState, String> {
  let response = reqwest::Client::new()
    .get(get_url)
    .bearer_auth(api_key)
    .send().await
    .map_err(|err| format!("Could not reach Replicate: {}", err))?;
  let status = response.status();
  let body: Value = response.json().await.unwrap_or(Value::Null);
  if !status.is_success() {
    return Err(friendly_error(status.as_u16(), &body));
  }
  Ok(parse_state(&body))
}

/// Upload a picture so a model can read it by URL. Returns that URL.
pub async fn upload_file(api_key: &str, bytes: Vec<u8>, file_name: &str, mime: &str) -> Result<String, String> {
  let part = Part::bytes(bytes).file_name(file_name.to_string()).mime_str(mime).map_err(|err| err.to_string())?;
  let response = reqwest::Client::new()
    .post(format!("{}/files", API_BASE))
    .bearer_auth(api_key)
    .multipart(Form::new().part("content", part))
    .send().await
    .map_err(|err| format!("Could not reach Replicate: {}", err))?;
  let status = response.status();
  let body: Value = response.json().await.unwrap_or(Value::Null);
  if !status.is_success() {
    return Err(friendly_error(status.as_u16(), &body));
  }
  body["urls"]["get"].as_str().map(str::to_string)
    .ok_or_else(|| "Replicate did not say where it put the picture.".to_string())
}

pub fn parse_created(body: &Value) -> Result<Created, String> {
  let id = body["id"].as_str().ok_or("Replicate did not return a prediction id.")?;
  let get_url = body["urls"]["get"].as_str().ok_or("Replicate did not return a status URL.")?;
  Ok(Created { id: id.to_string(), get_url: get_url.to_string() })
}

pub fn parse_state(body: &Value) -> PredictionState {
  match body["status"].as_str().unwrap_or("") {
    "succeeded" => {
      let files = output_files(&body["output"]);
      if files.is_empty() {
        PredictionState::Failed("Replicate finished without returning a file.".to_string())
      } else {
        PredictionState::Done(files)
      }
    }
    "failed" => PredictionState::Failed(body["error"].as_str().unwrap_or("Replicate could not make this.").to_string()),
    "canceled" => PredictionState::Failed("This was canceled on Replicate.".to_string()),
    _ => PredictionState::Running,
  }
}

/// Models return one URL, a list of URLs, or an object holding several. Collect every URL.
fn output_files(output: &Value) -> Vec<String> {
  match output {
    Value::String(url) if url.starts_with("http") => vec![url.clone()],
    Value::Array(items) => items.iter().flat_map(output_files).collect(),
    Value::Object(fields) => fields.values().flat_map(output_files).collect(),
    _ => Vec::new(),
  }
}

/// Replicate's errors are `{"title", "detail", "status"}`; the common ones deserve a plain sentence.
pub fn friendly_error(status: u16, body: &Value) -> String {
  let detail = body["detail"].as_str().unwrap_or("");
  match status {
    401 => "Replicate did not accept your API key. Check it in Settings, then API Keys.".to_string(),
    402 => "Your Replicate account is out of credit. Add billing on replicate.com and try again.".to_string(),
    404 => "Replicate does not have that model right now.".to_string(),
    422 => format!("Replicate rejected the settings: {}", detail),
    429 => "Replicate is rate limiting this account. Wait a moment and try again.".to_string(),
    _ if detail.is_empty() => format!("Replicate returned an error ({}).", status),
    _ => format!("Replicate returned an error: {}", detail.chars().take(300).collect::<String>()),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  mod created {
    use super::*;

    #[test]
    fn reads_the_id_and_status_url() {
      let body = json!({ "id": "abc", "status": "starting", "urls": { "get": "https://api.replicate.com/v1/predictions/abc", "cancel": "x" } });
      assert_eq!(parse_created(&body), Ok(Created { id: "abc".into(), get_url: "https://api.replicate.com/v1/predictions/abc".into() }));
    }

    #[test]
    fn a_reply_without_them_is_an_error() {
      assert!(parse_created(&json!({ "status": "starting" })).is_err());
    }
  }

  mod state {
    use super::*;

    #[test]
    fn starting_and_processing_are_still_running() {
      assert_eq!(parse_state(&json!({ "status": "starting" })), PredictionState::Running);
      assert_eq!(parse_state(&json!({ "status": "processing" })), PredictionState::Running);
    }

    #[test]
    fn one_file_as_text() {
      let state = parse_state(&json!({ "status": "succeeded", "output": "https://replicate.delivery/a.mp4" }));
      assert_eq!(state, PredictionState::Done(vec!["https://replicate.delivery/a.mp4".into()]));
    }

    #[test]
    fn a_list_of_files() {
      let state = parse_state(&json!({ "status": "succeeded", "output": ["https://x/a.png", "https://x/b.png"] }));
      assert_eq!(state, PredictionState::Done(vec!["https://x/a.png".into(), "https://x/b.png".into()]));
    }

    #[test]
    fn an_object_of_files_keeps_every_url_and_skips_the_rest() {
      let state = parse_state(&json!({ "status": "succeeded", "output": { "model_file": "https://x/m.glb", "note": "done", "n": 3 } }));
      assert_eq!(state, PredictionState::Done(vec!["https://x/m.glb".into()]));
    }

    #[test]
    fn success_without_a_file_is_a_failure() {
      assert!(matches!(parse_state(&json!({ "status": "succeeded", "output": null })), PredictionState::Failed(_)));
    }

    #[test]
    fn failure_carries_replicates_reason() {
      let state = parse_state(&json!({ "status": "failed", "error": "NSFW content detected" }));
      assert_eq!(state, PredictionState::Failed("NSFW content detected".into()));
    }

    #[test]
    fn canceled_is_a_failure() {
      assert!(matches!(parse_state(&json!({ "status": "canceled" })), PredictionState::Failed(_)));
    }
  }

  mod errors {
    use super::*;

    #[test]
    fn common_statuses_get_plain_sentences() {
      assert!(friendly_error(401, &Value::Null).contains("API Keys"));
      assert!(friendly_error(402, &Value::Null).contains("out of credit"));
      assert!(friendly_error(429, &Value::Null).contains("rate limiting"));
    }

    #[test]
    fn validation_errors_show_the_detail() {
      assert!(friendly_error(422, &json!({ "detail": "duration must be 5 or 10" })).contains("duration must be 5 or 10"));
    }

    #[test]
    fn unknown_statuses_still_say_something() {
      assert!(friendly_error(500, &Value::Null).contains("500"));
    }
  }
}
