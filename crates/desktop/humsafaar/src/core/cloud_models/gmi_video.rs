use gmicloud_client::creds::gmicloud_api_key::GmiCloudApiKey;
use gmicloud_client::requests::poll_request_queue::poll_request_queue::poll_gmicloud_request;

use crate::core::cloud_models::replicate_api::PredictionState;

/// Ask GMI Cloud how a video job is going.
pub async fn check_gmi_video(api_key: &str, request_id: &str) -> Result<PredictionState, String> {
  let polled = poll_gmicloud_request(&GmiCloudApiKey::new(api_key.to_string()), request_id)
    .await
    .map_err(|err| err.to_string())?;
  if polled.is_success() {
    return Ok(match polled.video_url() {
      Some(url) => PredictionState::Done(vec![url.to_string()]),
      None => PredictionState::Failed("GMI Cloud finished but returned no video".to_string()),
    });
  }
  Ok(if polled.is_failed() { PredictionState::Failed("GMI Cloud could not make this video".to_string()) } else { PredictionState::Running })
}
