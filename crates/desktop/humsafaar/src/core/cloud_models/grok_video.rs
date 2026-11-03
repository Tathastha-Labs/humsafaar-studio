use grok_api_client::api::requests::videos::video_status::video_status::{video_status, VideoStatus, VideoStatusArgs, VideoStatusRequest};
use grok_api_client::creds::grok_api_key::GrokApiKey;

use crate::core::cloud_models::replicate_api::PredictionState;

/// Ask xAI how a video job is going. A finished job gives back a temporary link to the video.
pub async fn check_grok_video(api_key: &str, request_id: &str) -> Result<PredictionState, String> {
  let key = GrokApiKey::new(api_key.to_string());
  let status = video_status(VideoStatusArgs { api_key: &key, request: VideoStatusRequest { request_id: request_id.to_string() } })
    .await
    .map_err(|err| err.to_string())?
    .status;
  Ok(match status {
    VideoStatus::Pending { .. } => PredictionState::Running,
    VideoStatus::Complete { video, .. } => match video.and_then(|video| video.url) {
      Some(url) => PredictionState::Done(vec![url]),
      None => PredictionState::Failed("xAI finished but returned no video".to_string()),
    },
    VideoStatus::Failed { error, .. } => PredictionState::Failed(error.unwrap_or_else(|| "xAI could not make this video".to_string())),
  })
}
