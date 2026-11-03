use enums::tauri::tasks::task_type::TaskType;
use log::info;

use crate::core::cloud_models::fal_video::VideoInputs;
use crate::core::cloud_models::local_picture::LocalPicture;
use crate::core::cloud_models::replicate_queue::{enqueue_on_replicate, load_api_key, picture_for_replicate};
use crate::core::cloud_models::replicate_recipes::{is_video_supported, replicate_video_job};
use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::commands::generate::generate_video::fal::handle_fal_video::video_model_id;
use crate::core::commands::generate::generate_video::request::TauriGenerateVideoRequest;
use crate::core::local_media::local_media_store::LocalMediaStore;
use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;

/// Video on Replicate with the user's own key, from a prompt alone or from a start picture.
pub async fn handle_replicate_video(
  request: &TauriGenerateVideoRequest,
  credentials: &ProviderCredentialLoadingCache,
  local_media: &LocalMediaStore,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let model = request.model.ok_or_else(GenerateError::no_model_specified)?;
  if !is_video_supported(model) {
    return Err(GenerateError::NotYetImplemented(format!("{:?} is not available on Replicate yet", model)));
  }

  let prompt = request.prompt.as_deref().map(str::trim).filter(|p| !p.is_empty())
    .ok_or_else(|| GenerateError::AnyhowError(anyhow::anyhow!("A prompt is required")))?;

  let api_key = load_api_key(credentials, ProviderCredentialKey::ReplicateApiKey, "Replicate")?;
  let start_image = match &request.start_frame_image_media_token {
    Some(token) => Some(picture_for_replicate(&api_key, &LocalPicture::read(token, local_media)?).await),
    None => None,
  };

  let inputs = VideoInputs {
    prompt,
    negative_prompt: request.negative_prompt.as_deref(),
    start_image,
    aspect_ratio: request.aspect_ratio,
    resolution: request.resolution,
    duration_seconds: request.duration_seconds,
    generate_audio: request.generate_audio,
  };
  let job = replicate_video_job(model, &inputs).map_err(|message| GenerateError::AnyhowError(anyhow::anyhow!(message)))?;

  info!("Queueing Replicate video: {} ({:?}), from a picture={}", job.model, model, inputs.start_image.is_some());
  enqueue_on_replicate(job.model, job.input, &api_key, TaskType::VideoGeneration, video_model_id(model)).await
}
