use thiserror::Error;

#[derive(Debug, Error)]
pub enum ComfyUiError {
  #[error("ComfyUI is not reachable at {0}")]
  NotReachable(String),

  #[error("ComfyUI request failed: {0}")]
  Http(#[from] reqwest::Error),

  #[error("ComfyUI rejected the workflow ({status}): {body}")]
  Rejected { status: u16, body: String },

  #[error("ComfyUI returned an unexpected response: {0}")]
  BadResponse(String),

  #[error("ComfyUI job failed: {0}")]
  JobFailed(String),

  #[error("timed out waiting for ComfyUI job {0}")]
  Timeout(String),
}
