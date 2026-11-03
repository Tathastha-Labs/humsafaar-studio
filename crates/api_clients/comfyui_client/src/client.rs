use crate::error::ComfyUiError;
use crate::history::{parse_history, queue_has_prompt, JobState, OutputImage};
use serde_json::{json, Value};
use std::time::Duration;

const POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone)]
pub struct ComfyUiClient {
  base_url: String,
  http: reqwest::Client,
}

impl ComfyUiClient {
  /// `base_url` like `http://127.0.0.1:8188`.
  pub fn new(base_url: &str) -> Self {
    Self {
      base_url: base_url.trim_end_matches('/').to_string(),
      http: reqwest::Client::new(),
    }
  }

  pub fn base_url(&self) -> &str {
    &self.base_url
  }

  pub async fn is_healthy(&self) -> bool {
    match self.http.get(self.url("/system_stats")).timeout(Duration::from_secs(3)).send().await {
      Ok(response) => response.status().is_success(),
      Err(_) => false,
    }
  }

  /// Names in ComfyUI's `models/checkpoints`, as ComfyUI sees them.
  pub async fn list_checkpoints(&self) -> Result<Vec<String>, ComfyUiError> {
    let info: Value = self.get_json("/object_info/CheckpointLoaderSimple").await?;
    let names = info["CheckpointLoaderSimple"]["input"]["required"]["ckpt_name"][0]
      .as_array()
      .ok_or_else(|| ComfyUiError::BadResponse("no checkpoint list in object_info".into()))?;
    Ok(names.iter().filter_map(|n| n.as_str().map(str::to_string)).collect())
  }

  /// Queue a workflow and return its prompt id.
  pub async fn queue_prompt(&self, workflow: &Value) -> Result<String, ComfyUiError> {
    let response = self.http.post(self.url("/prompt"))
      .json(&json!({ "prompt": workflow }))
      .send().await
      .map_err(|e| self.map_connect_error(e))?;

    let status = response.status();
    if !status.is_success() {
      let body = response.text().await.unwrap_or_default();
      return Err(ComfyUiError::Rejected { status: status.as_u16(), body });
    }

    let body: Value = response.json().await?;
    body.get("prompt_id").and_then(Value::as_str).map(str::to_string)
      .ok_or_else(|| ComfyUiError::BadResponse("no prompt_id in /prompt response".into()))
  }

  /// One look at a job's state, without waiting.
  pub async fn job_state(&self, prompt_id: &str) -> Result<JobState, ComfyUiError> {
    let body: Value = self.get_json(&format!("/history/{}", prompt_id)).await?;
    parse_history(prompt_id, &body)
  }

  /// Whether a job is waiting or running. A job that is in neither the queue nor the
  /// history is gone (for example the engine restarted, which empties both).
  pub async fn queue_contains(&self, prompt_id: &str) -> Result<bool, ComfyUiError> {
    let body: Value = self.get_json("/queue").await?;
    Ok(queue_has_prompt(&body, prompt_id))
  }

  /// Poll the history until the job finishes, fails or the timeout passes.
  pub async fn wait_for_outputs(
    &self,
    prompt_id: &str,
    timeout: Duration,
  ) -> Result<Vec<OutputImage>, ComfyUiError> {
    let started = tokio::time::Instant::now();
    loop {
      match self.job_state(prompt_id).await? {
        JobState::Done(images) => return Ok(images),
        JobState::Failed(message) => return Err(ComfyUiError::JobFailed(message)),
        JobState::Pending => {}
      }
      if started.elapsed() >= timeout {
        return Err(ComfyUiError::Timeout(prompt_id.to_string()));
      }
      tokio::time::sleep(POLL_INTERVAL).await;
    }
  }

  /// Upload an image to the engine's input folder and return the name to load it by.
  pub async fn upload_image(&self, bytes: Vec<u8>, file_name: &str) -> Result<String, ComfyUiError> {
    let part = reqwest::multipart::Part::bytes(bytes)
      .file_name(file_name.to_string())
      .mime_str("image/png")?;
    let form = reqwest::multipart::Form::new()
      .part("image", part)
      .text("overwrite", "true");

    let response = self.http.post(self.url("/upload/image")).multipart(form).send().await
      .map_err(|e| self.map_connect_error(e))?;
    let status = response.status();
    if !status.is_success() {
      let body = response.text().await.unwrap_or_default();
      return Err(ComfyUiError::Rejected { status: status.as_u16(), body });
    }

    let body: Value = response.json().await?;
    body.get("name").and_then(Value::as_str).map(str::to_string)
      .ok_or_else(|| ComfyUiError::BadResponse("no name in /upload/image response".into()))
  }

  pub async fn download_image(&self, image: &OutputImage) -> Result<Vec<u8>, ComfyUiError> {
    let response = self.http.get(self.url("/view"))
      .query(&[
        ("filename", image.filename.as_str()),
        ("subfolder", image.subfolder.as_str()),
        ("type", image.kind.as_str()),
      ])
      .send().await?
      .error_for_status()?;
    Ok(response.bytes().await?.to_vec())
  }

  async fn get_json(&self, path: &str) -> Result<Value, ComfyUiError> {
    let response = self.http.get(self.url(path)).send().await
      .map_err(|e| self.map_connect_error(e))?
      .error_for_status()?;
    Ok(response.json().await?)
  }

  fn url(&self, path: &str) -> String {
    format!("{}{}", self.base_url, path)
  }

  fn map_connect_error(&self, err: reqwest::Error) -> ComfyUiError {
    if err.is_connect() {
      ComfyUiError::NotReachable(self.base_url.clone())
    } else {
      ComfyUiError::Http(err)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn trims_trailing_slash() {
    assert_eq!(ComfyUiClient::new("http://127.0.0.1:8188/").base_url(), "http://127.0.0.1:8188");
  }

  #[tokio::test]
  async fn unreachable_server_is_reported_not_healthy() {
    // Port 9 (discard) is not a ComfyUI server.
    assert!(!ComfyUiClient::new("http://127.0.0.1:9").is_healthy().await);
  }

  /// Needs a running ComfyUI with an SD 1.5 checkpoint:
  /// `COMFYUI_URL=http://127.0.0.1:8188 cargo test -p comfyui_client -- --ignored`
  #[tokio::test]
  #[ignore]
  async fn generates_an_image_against_a_real_server() {
    use crate::workflows::{text_to_image, TextToImageParams};

    let url = std::env::var("COMFYUI_URL").unwrap_or_else(|_| "http://127.0.0.1:8188".into());
    let client = ComfyUiClient::new(&url);
    assert!(client.is_healthy().await, "ComfyUI is not running at {}", url);

    let checkpoints = client.list_checkpoints().await.unwrap();
    let checkpoint = checkpoints.first().expect("no checkpoints installed");

    let mut params = TextToImageParams::new(checkpoint, "a watercolor fox in a forest", 7);
    params.steps = 8;
    let id = client.queue_prompt(&text_to_image(&params)).await.unwrap();
    let images = client.wait_for_outputs(&id, Duration::from_secs(300)).await.unwrap();
    assert_eq!(images.len(), 1);

    let bytes = client.download_image(&images[0]).await.unwrap();
    assert_eq!(&bytes[1..4], b"PNG");
  }
}
