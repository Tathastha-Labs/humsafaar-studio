use humsafaar_router::api::image_list_ref::ImageListRef;
use enums::common::generation::common_aspect_ratio::CommonAspectRatio;
use enums::common::generation::common_resolution::CommonResolution;
use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_type::TaskType;
use grok_api_client::api::requests::images::image_edit::image_edit::{image_edit, ImageEditArgs, ImageEditRequest, ImageSource};
use grok_api_client::api::requests::images::image_generation::image_generation::{image_generation, ImageGenerationArgs, ImageGenerationRequest};
use grok_api_client::api::types::image_types::image_aspect_ratio::ImageAspectRatio;
use grok_api_client::api::types::image_types::image_model::ImageModel;
use grok_api_client::api::types::image_types::image_resolution::ImageResolution;
use grok_api_client::creds::grok_api_key::GrokApiKey;

use crate::core::cloud_models::replicate_queue::load_api_key;
use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::commands::generate::generate_image::providers::humsafaar_router::handle_api_providers::resolve_image_inputs;
use crate::core::commands::generate::generate_image::tauri_generate_image_request::TauriGenerateImageRequest;
use crate::core::commands::generate::generate_image::tauri_image_model::TauriImageModel;
use crate::core::events::generation_events::common::GenerationModel;
use crate::core::local_media::local_media_store::LocalMediaStore;
use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;

/// xAI allows up to three source pictures for an edit.
const MAX_SOURCE_IMAGES: usize = 3;

/// Separates the picture links saved on the task for the poller to download.
pub const RESULT_URL_SEPARATOR: char = '\n';

/// An image on xAI with the user's own key: an edit when pictures are attached, otherwise from the
/// prompt. xAI answers at once, so the picture links are saved on the task and the shared poller
/// downloads them on its next round.
pub async fn handle_grok_image(
  request: &TauriGenerateImageRequest,
  credentials: &ProviderCredentialLoadingCache,
  local_media: &LocalMediaStore,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let model = request.model.ok_or_else(GenerateError::no_model_specified)?;
  if !matches!(model, TauriImageModel::GrokImage) {
    return Err(GenerateError::NotYetImplemented(format!("{:?} is not a Grok model", model)));
  }
  let prompt = request.prompt.as_deref().map(str::trim).filter(|p| !p.is_empty())
    .ok_or_else(|| GenerateError::AnyhowError(anyhow::anyhow!("A prompt is required")))?;

  let api_key = GrokApiKey::new(load_api_key(credentials, ProviderCredentialKey::GrokApiKey, "Grok")?);
  let sources = match resolve_image_inputs(request, local_media).await? {
    Some(ImageListRef::Urls(urls)) => urls,
    _ => Vec::new(),
  };
  let number_images = request.batch_size.map(|n| n.clamp(1, 4));
  let aspect_ratio = request.aspect_ratio.and_then(aspect);
  let resolution = request.resolution.and_then(resolution);

  let result = if sources.is_empty() {
    image_generation(ImageGenerationArgs {
      api_key: &api_key,
      request: ImageGenerationRequest {
        prompt: prompt.to_string(), model: Some(ImageModel::GrokImagineImage), number_images, aspect_ratio,
        resolution, response_format: None, user: None,
      },
    }).await.map(|done| done.images.into_iter().filter_map(|image| image.url).collect::<Vec<String>>())
  } else {
    image_edit(ImageEditArgs {
      api_key: &api_key,
      request: ImageEditRequest {
        prompt: prompt.to_string(),
        source_images: sources.into_iter().take(MAX_SOURCE_IMAGES).map(ImageSource::Url).collect(),
        model: Some(ImageModel::GrokImagineImage), number_images, aspect_ratio,
        resolution, response_format: None, user: None,
      },
    }).await.map(|done| done.images.into_iter().filter_map(|image| image.url).collect::<Vec<String>>())
  };
  let images = result.map_err(|err| GenerateError::AnyhowError(anyhow::anyhow!("Grok: {}", err)))?;

  let urls = images;
  if urls.is_empty() {
    return Err(GenerateError::AnyhowError(anyhow::anyhow!("xAI returned no pictures")));
  }

  Ok(TaskEnqueueSuccess {
    task_type: TaskType::ImageGeneration,
    model: Some(GenerationModel::Unknown("grok_imagine_image".to_string())),
    provider: GenerationProvider::Grok,
    provider_job_id: Some(uuid_utils::uuid::generate_random_uuid()),
    maybe_queue_status_url: None,
    maybe_queue_response_url: Some(urls.join(&RESULT_URL_SEPARATOR.to_string())),
    maybe_prompt_token: None,
  })
}

fn aspect(ratio: CommonAspectRatio) -> Option<ImageAspectRatio> {
  use CommonAspectRatio::*;
  Some(match ratio {
    Square | SquareHd => ImageAspectRatio::Square,
    WideSixteenByNine | Wide => ImageAspectRatio::Landscape16x9,
    TallNineBySixteen | Tall => ImageAspectRatio::Portrait9x16,
    WideFourByThree => ImageAspectRatio::Landscape4x3,
    TallThreeByFour => ImageAspectRatio::Portrait3x4,
    WideThreeByTwo => ImageAspectRatio::Landscape3x2,
    TallTwoByThree => ImageAspectRatio::Portrait2x3,
    Auto => ImageAspectRatio::Auto,
    _ => return None,
  })
}

fn resolution(resolution: CommonResolution) -> Option<ImageResolution> {
  match resolution {
    CommonResolution::OneK => Some(ImageResolution::OneK),
    CommonResolution::TwoK => Some(ImageResolution::TwoK),
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_ratio_grok_lacks_keeps_its_default() {
    assert_eq!(aspect(CommonAspectRatio::TallNineByTwentyOne), None);
    assert_eq!(aspect(CommonAspectRatio::WideSixteenByNine), Some(ImageAspectRatio::Landscape16x9));
  }

  #[test]
  fn only_one_and_two_k_are_offered() {
    assert_eq!(resolution(CommonResolution::FourK), None);
    assert_eq!(resolution(CommonResolution::TwoK), Some(ImageResolution::TwoK));
  }
}
