use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_type::TaskType;
use worldlabs_api_client::api::api_types::operation_id::OperationId;
use worldlabs_api_client::api::api_types::world_labs_model::WorldLabsModel;
use worldlabs_api_client::api::requests::generate_world::generate_world::{generate_world, GenerateWorldArgs};
use worldlabs_api_client::api::requests::generate_world::http_request::{ContentReference, WorldPrompt};
use worldlabs_api_client::api::requests::get_operation::get_operation::{get_operation, GetOperationArgs};
use worldlabs_api_client::api::requests::prepare_upload::prepare_upload::{prepare_upload, MediaAssetKind, PrepareUploadArgs};
use worldlabs_api_client::api::requests::upload_to_signed_url::upload_to_signed_url::{upload_to_signed_url, UploadToSignedUrlArgs};
use worldlabs_api_client::credentials::world_labs_api_creds::WorldLabsApiCreds;

use crate::core::cloud_models::local_picture::LocalPicture;
use crate::core::cloud_models::replicate_api::PredictionState;
use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::events::generation_events::common::GenerationModel;

/// Start a world on World Labs from a picture, a prompt, or both. The job is followed by its
/// operation id, which the shared poller asks World Labs about.
pub async fn enqueue_world(
  api_key: &str,
  model_id: &str,
  prompt: Option<&str>,
  picture: Option<&LocalPicture>,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let model = model_for_id(model_id)
    .ok_or_else(|| GenerateError::NotYetImplemented(format!("{} is not a World Labs model", model_id)))?;
  let creds = WorldLabsApiCreds::new(api_key.to_string());
  let text_prompt = prompt.map(str::trim).filter(|text| !text.is_empty()).map(str::to_string);

  let world_prompt = match picture {
    Some(picture) => WorldPrompt::Image {
      image_prompt: ContentReference::MediaAsset { media_asset_id: upload_picture(&creds, picture).await? },
      text_prompt,
      is_pano: None,
      disable_recaption: None,
    },
    None if text_prompt.is_some() => WorldPrompt::Text { text_prompt, disable_recaption: None },
    None => return Err(GenerateError::AnyhowError(anyhow::anyhow!("Add a picture or a description to make a world."))),
  };

  let started = generate_world(GenerateWorldArgs {
    creds: &creds,
    world_prompt,
    display_name: None,
    model,
    seed: None,
    tags: None,
    permission: None,
    request_timeout: None,
  }).await.map_err(to_generate_error)?;

  Ok(TaskEnqueueSuccess {
    task_type: TaskType::GaussianGeneration,
    model: Some(GenerationModel::Unknown(model_id.to_string())),
    provider: GenerationProvider::WorldLabs,
    provider_job_id: Some(started.operation_id.as_str().to_string()),
    maybe_queue_status_url: None,
    maybe_queue_response_url: None,
    maybe_prompt_token: None,
  })
}

/// Ask World Labs how a job is going. A finished world gives back its splat file.
pub async fn check_operation(api_key: &str, operation_id: &str) -> Result<PredictionState, String> {
  let creds = WorldLabsApiCreds::new(api_key.to_string());
  let operation_id = OperationId(operation_id.to_string());
  let operation = get_operation(GetOperationArgs { creds: &creds, operation_id: &operation_id, request_timeout: None })
    .await
    .map_err(|err| err.to_string())?;
  if let Some(error) = operation.error {
    return Ok(PredictionState::Failed(error.message.unwrap_or_else(|| "World Labs could not make this world".to_string())));
  }
  if !operation.done {
    return Ok(PredictionState::Running);
  }
  let splats = operation.world.and_then(|world| world.assets).and_then(|assets| assets.splats);
  match splats.and_then(|splats| splats.spz_url_full_res.or(splats.spz_url_500k).or(splats.spz_url_100k)) {
    Some(url) => Ok(PredictionState::Done(vec![url])),
    None => Ok(PredictionState::Failed("World Labs finished but returned no world file".to_string())),
  }
}

async fn upload_picture(creds: &WorldLabsApiCreds, picture: &LocalPicture) -> Result<String, GenerateError> {
  let prepared = prepare_upload(PrepareUploadArgs {
    creds,
    file_name: &picture.file_name,
    kind: MediaAssetKind::Image,
    request_timeout: None,
  }).await.map_err(to_generate_error)?;

  upload_to_signed_url(UploadToSignedUrlArgs {
    upload_url: &prepared.upload_url,
    file_bytes: picture.bytes.clone(),
    required_headers: &prepared.required_headers,
    content_type: picture.mime,
    request_timeout: None,
  }).await.map_err(to_generate_error)?;

  Ok(prepared.media_asset_id.as_str().to_string())
}

/// The World Labs model for an id the model list uses.
pub fn model_for_id(id: &str) -> Option<WorldLabsModel> {
  match id {
    "marble_1p0_draft" => Some(WorldLabsModel::Marble1p0Draft),
    "marble_1p0" => Some(WorldLabsModel::Marble1p0),
    "marble_1p1" => Some(WorldLabsModel::Marble1p1),
    "marble_1p1_plus" => Some(WorldLabsModel::Marble1p1Plus),
    _ => None,
  }
}

fn to_generate_error(err: worldlabs_api_client::error::world_labs_error::WorldLabsError) -> GenerateError {
  if err.is_403_forbidden() {
    return GenerateError::AnyhowError(anyhow::anyhow!("World Labs refused the key. Check it in Settings, then API Keys."));
  }
  GenerateError::AnyhowError(anyhow::anyhow!("World Labs: {}", err))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn every_listed_model_maps_to_an_api_name() {
    for (id, api_name) in [
      ("marble_1p0_draft", "marble-1.0-draft"),
      ("marble_1p0", "marble-1.0"),
      ("marble_1p1", "marble-1.1"),
      ("marble_1p1_plus", "marble-1.1-plus"),
    ] {
      assert_eq!(model_for_id(id).unwrap().get_model_api_name_str(), api_name);
    }
  }

  #[test]
  fn retired_and_unknown_models_are_refused() {
    assert!(model_for_id("marble_0p1_mini").is_none());
    assert!(model_for_id("nope").is_none());
  }
}
