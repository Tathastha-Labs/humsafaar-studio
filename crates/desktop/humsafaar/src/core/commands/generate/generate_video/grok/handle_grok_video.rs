use humsafaar_router::api::router_aspect_ratio::RouterAspectRatio;
use humsafaar_router::api::router_resolution::RouterResolution;
use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_type::TaskType;
use grok_api_client::api::requests::videos::video_generation::video_generation::{
  video_generation, VideoGenerationArgs, VideoGenerationRequest, VideoImageSource,
};
use grok_api_client::api::types::video_types::video_aspect_ratio::VideoAspectRatio;
use grok_api_client::api::types::video_types::video_model::VideoModel;
use grok_api_client::api::types::video_types::video_resolution::VideoResolution;
use grok_api_client::creds::grok_api_key::GrokApiKey;

use crate::core::cloud_models::local_picture::LocalPicture;
use crate::core::cloud_models::replicate_queue::load_api_key;
use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::commands::generate::generate_video::request::{TauriGenerateVideoRequest, TauriVideoModel};
use crate::core::events::generation_events::common::GenerationModel;
use crate::core::local_media::local_media_store::LocalMediaStore;
use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;

/// Video on xAI with the user's own key, from a prompt alone or from a start picture.
pub async fn handle_grok_video(
  request: &TauriGenerateVideoRequest,
  credentials: &ProviderCredentialLoadingCache,
  local_media: &LocalMediaStore,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let model = request.model.ok_or_else(GenerateError::no_model_specified)?;
  let (grok_model, model_id) = grok_model(model)
    .ok_or_else(|| GenerateError::NotYetImplemented(format!("{:?} is not a Grok model", model)))?;

  let prompt = request.prompt.as_deref().map(str::trim).filter(|p| !p.is_empty())
    .ok_or_else(|| GenerateError::AnyhowError(anyhow::anyhow!("A prompt is required")))?;

  let api_key = GrokApiKey::new(load_api_key(credentials, ProviderCredentialKey::GrokApiKey, "Grok")?);
  let image = match &request.start_frame_image_media_token {
    Some(token) => Some(VideoImageSource::Url(LocalPicture::read(token, local_media)?.data_uri())),
    None => None,
  };

  let started = video_generation(VideoGenerationArgs {
    api_key: &api_key,
    request: VideoGenerationRequest {
      prompt: prompt.to_string(),
      model: Some(grok_model),
      image,
      reference_images: None,
      aspect_ratio: request.aspect_ratio.and_then(aspect),
      duration: request.duration_seconds.map(|s| u32::from(s).clamp(1, 15)),
      resolution: request.resolution.and_then(resolution),
      user: None,
    },
  }).await.map_err(|err| GenerateError::AnyhowError(anyhow::anyhow!("Grok: {}", err)))?;

  Ok(TaskEnqueueSuccess {
    task_type: TaskType::VideoGeneration,
    model: Some(GenerationModel::Unknown(model_id.to_string())),
    provider: GenerationProvider::Grok,
    provider_job_id: Some(started.request_id),
    maybe_queue_status_url: None,
    maybe_queue_response_url: None,
    maybe_prompt_token: None,
  })
}

fn grok_model(model: TauriVideoModel) -> Option<(VideoModel, &'static str)> {
  match model {
    TauriVideoModel::GrokVideo => Some((VideoModel::GrokImagineVideo, "grok_imagine_video")),
    TauriVideoModel::GrokImagineVideo1p5 => Some((VideoModel::GrokImagineVideo1p5, "grok_imagine_video_1p5")),
    _ => None,
  }
}

fn aspect(ratio: RouterAspectRatio) -> Option<VideoAspectRatio> {
  use RouterAspectRatio::*;
  Some(match ratio {
    Square | SquareHd => VideoAspectRatio::Square,
    WideSixteenByNine | Wide => VideoAspectRatio::Landscape16x9,
    TallNineBySixteen | Tall => VideoAspectRatio::Portrait9x16,
    WideFourByThree => VideoAspectRatio::Landscape4x3,
    TallThreeByFour => VideoAspectRatio::Portrait3x4,
    WideThreeByTwo => VideoAspectRatio::Landscape3x2,
    TallTwoByThree => VideoAspectRatio::Portrait2x3,
    _ => return None,
  })
}

fn resolution(resolution: RouterResolution) -> Option<VideoResolution> {
  Some(match resolution {
    RouterResolution::FourEightyP => VideoResolution::FourEightyP,
    RouterResolution::SevenTwentyP => VideoResolution::SevenTwentyP,
    RouterResolution::TenEightyP => VideoResolution::TenEightyP,
    _ => return None,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn only_grok_models_are_taken() {
    assert_eq!(grok_model(TauriVideoModel::GrokImagineVideo1p5).unwrap().1, "grok_imagine_video_1p5");
    assert!(grok_model(TauriVideoModel::Kling2p6Pro).is_none());
  }

  #[test]
  fn a_ratio_grok_lacks_keeps_its_default() {
    assert_eq!(aspect(RouterAspectRatio::WideTwentyOneByNine), None);
    assert_eq!(aspect(RouterAspectRatio::TallNineBySixteen), Some(VideoAspectRatio::Portrait9x16));
  }
}
