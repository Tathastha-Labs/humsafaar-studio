use enums::tauri::tasks::task_type::TaskType;
use log::info;

use crate::core::cloud_models::fal_queue::{enqueue_on_fal, load_fal_api_key};
use crate::core::cloud_models::local_picture::LocalPicture;
use crate::core::cloud_models::fal_video::{fal_video_job, is_supported, VideoInputs};
use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::commands::generate::generate_video::request::{TauriGenerateVideoRequest, TauriVideoModel};
use crate::core::local_media::local_media_store::LocalMediaStore;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;

/// Video on Fal with the user's own key, from a prompt alone or from a start picture.
pub async fn handle_fal_video(
  request: &TauriGenerateVideoRequest,
  credentials: &ProviderCredentialLoadingCache,
  local_media: &LocalMediaStore,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let model = request.model.ok_or_else(GenerateError::no_model_specified)?;
  if !is_supported(model) {
    return Err(GenerateError::NotYetImplemented(format!("{:?} is not available on Fal yet", model)));
  }

  let prompt = request.prompt.as_deref().map(str::trim).filter(|p| !p.is_empty())
    .ok_or_else(|| GenerateError::AnyhowError(anyhow::anyhow!("A prompt is required")))?;

  let api_key = load_fal_api_key(credentials)?;
  let start_image = start_image_data_uri(request, local_media)?;

  let inputs = VideoInputs {
    prompt,
    negative_prompt: request.negative_prompt.as_deref(),
    start_image,
    aspect_ratio: request.aspect_ratio,
    resolution: request.resolution,
    duration_seconds: request.duration_seconds,
    generate_audio: request.generate_audio,
  };
  let job = fal_video_job(model, &inputs).map_err(|message| GenerateError::AnyhowError(anyhow::anyhow!(message)))?;

  info!("Queueing Fal video: {} ({:?}), from a picture={}", job.endpoint, model, inputs.start_image.is_some());
  enqueue_on_fal(job.endpoint, job.input, &api_key, TaskType::VideoGeneration, video_model_id(model)).await
}

/// The start picture as an inline `data:` URI, which Fal accepts, so nothing has to be uploaded first.
fn start_image_data_uri(
  request: &TauriGenerateVideoRequest,
  local_media: &LocalMediaStore,
) -> Result<Option<String>, GenerateError> {
  match &request.start_frame_image_media_token {
    Some(token) => Ok(Some(LocalPicture::read(token, local_media)?.data_uri())),
    None => Ok(None),
  }
}

pub fn video_model_id(model: TauriVideoModel) -> &'static str {
  match model {
    TauriVideoModel::Kling2p5TurboPro => "kling_2p5_turbo_pro",
    TauriVideoModel::Kling2p6Pro => "kling_2p6_pro",
    TauriVideoModel::Veo3p1Fast => "veo_3p1_fast",
    TauriVideoModel::Seedance1p5Pro => "seedance_1p5_pro",
    _ => "unknown",
  }
}
