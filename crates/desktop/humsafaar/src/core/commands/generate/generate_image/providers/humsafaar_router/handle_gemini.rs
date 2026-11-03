use enums::common::generation::common_aspect_ratio::CommonAspectRatio;
use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_type::TaskType;
use humsafaar_router::api::image_list_ref::ImageListRef;

use crate::core::cloud_models::gemini_image::{extension_for, generate_picture, GeminiImageInputs};
use crate::core::cloud_models::replicate_queue::load_api_key;
use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::commands::generate::generate_image::providers::humsafaar_router::handle_api_providers::resolve_image_inputs;
use crate::core::commands::generate::generate_image::providers::humsafaar_router::RESULT_URL_SEPARATOR;
use crate::core::commands::generate::generate_image::tauri_generate_image_request::TauriGenerateImageRequest;
use crate::core::commands::generate::generate_image::tauri_image_model::TauriImageModel;
use crate::core::events::generation_events::common::GenerationModel;
use crate::core::local_media::local_media_store::LocalMediaStore;
use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;

const MAX_BATCH_SIZE: u32 = 4;

/// Scheme of the links saved on the task: Gemini answers with the picture itself, so it is written
/// to a file first and the shared poller picks it up from there.
pub const LOCAL_FILE_SCHEME: &str = "file://";

/// Pictures from Gemini with the user's own key: an edit when pictures are attached, otherwise from
/// the prompt. One request per picture.
pub async fn handle_gemini_image(
  request: &TauriGenerateImageRequest,
  credentials: &ProviderCredentialLoadingCache,
  local_media: &LocalMediaStore,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let model = request.model.ok_or_else(GenerateError::no_model_specified)?;
  let model_name = gemini_model(model)
    .ok_or_else(|| GenerateError::NotYetImplemented(format!("{:?} is not available on Gemini", model)))?;
  let prompt = request.prompt.as_deref().map(str::trim).filter(|p| !p.is_empty())
    .ok_or_else(|| GenerateError::AnyhowError(anyhow::anyhow!("A prompt is required")))?;

  let api_key = load_api_key(credentials, ProviderCredentialKey::GeminiApiKey, "Gemini")?;
  let sources = match resolve_image_inputs(request, local_media).await? {
    Some(ImageListRef::Urls(urls)) => urls,
    _ => Vec::new(),
  };
  let inputs = GeminiImageInputs {
    prompt,
    sources: &sources,
    aspect_ratio: request.aspect_ratio.and_then(aspect),
  };

  let count = request.batch_size.unwrap_or(1).clamp(1, MAX_BATCH_SIZE);
  let mut links = Vec::with_capacity(count as usize);
  for _ in 0..count {
    let picture = generate_picture(&api_key, model_name, &inputs).await
      .map_err(|message| GenerateError::AnyhowError(anyhow::anyhow!(message)))?;
    let path = std::env::temp_dir().join(format!("gemini_{}.{}", uuid_utils::uuid::generate_random_uuid(), extension_for(&picture.mime)));
    std::fs::write(&path, &picture.bytes).map_err(|err| GenerateError::AnyhowError(err.into()))?;
    links.push(format!("{}{}", LOCAL_FILE_SCHEME, path.display()));
  }

  Ok(TaskEnqueueSuccess {
    task_type: TaskType::ImageGeneration,
    model: Some(GenerationModel::Unknown(model_name.to_string())),
    provider: GenerationProvider::Gemini,
    provider_job_id: Some(uuid_utils::uuid::generate_random_uuid()),
    maybe_queue_status_url: None,
    maybe_queue_response_url: Some(links.join(&RESULT_URL_SEPARATOR.to_string())),
    maybe_prompt_token: None,
  })
}

/// Gemini's own names for the models the picture pages call Nano Banana.
fn gemini_model(model: TauriImageModel) -> Option<&'static str> {
  match model {
    TauriImageModel::NanoBanana => Some("gemini-2.5-flash-image"),
    TauriImageModel::NanoBanana2 => Some("gemini-3.1-flash-image"),
    TauriImageModel::NanoBananaPro => Some("gemini-3-pro-image"),
    _ => None,
  }
}

fn aspect(ratio: CommonAspectRatio) -> Option<&'static str> {
  use CommonAspectRatio::*;
  Some(match ratio {
    Square | SquareHd => "1:1",
    Wide | WideSixteenByNine => "16:9",
    Tall | TallNineBySixteen => "9:16",
    WideThreeByTwo => "3:2",
    WideFourByThree => "4:3",
    WideFiveByFour => "5:4",
    WideTwentyOneByNine => "21:9",
    TallTwoByThree => "2:3",
    TallThreeByFour => "3:4",
    TallFourByFive => "4:5",
    _ => return None,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn nano_banana_models_map_to_gemini_names_and_others_do_not() {
    assert_eq!(gemini_model(TauriImageModel::NanoBananaPro), Some("gemini-3-pro-image"));
    assert_eq!(gemini_model(TauriImageModel::Flux1Dev), None);
  }

  #[test]
  fn aspect_ratios_gemini_has_no_name_for_are_left_to_gemini() {
    assert_eq!(aspect(CommonAspectRatio::WideSixteenByNine), Some("16:9"));
    assert_eq!(aspect(CommonAspectRatio::Auto), None);
  }
}
