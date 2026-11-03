use log::error;
use serde_json::Value;
use tauri::{AppHandle, Manager};
use tokens::tokens::media_files::MediaFileToken;
use enums::common::generation_provider::GenerationProvider;

use crate::core::cloud_models::local_picture::LocalPicture;
use crate::core::cloud_models::replicate_queue::load_api_key;
use crate::core::cloud_models::world_labs::enqueue_world;
use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::commands::generate::enqueued_job::record_enqueued_job;
use crate::core::commands::generate::omni::dispatch::local_error;
use crate::core::commands::generate::omni::{OmniRequest, OmniResult};
use crate::core::local_media::local_media_store::LocalMediaStore;
use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;

/// A 3D world on World Labs with the user's own key. It starts from the first picture the user
/// attached, or from the prompt alone.
#[tauri::command]
pub async fn generate_splat_command(request: OmniRequest, app: AppHandle) -> OmniResult {
  if !matches!(request.provider, None | Some(GenerationProvider::WorldLabs)) {
    return Err(local_error("Pick World Labs for this model first."));
  }
  let cache = app.state::<ProviderCredentialLoadingCache>();
  let local_media = app.state::<LocalMediaStore>();
  match handle_world(&request, &cache, &local_media).await {
    Ok(success) => record_enqueued_job(&request, success, &app).await,
    Err(err) => {
      error!("World generation failed: {:?}", err);
      Err(local_error(format!("{:?}", err)))
    }
  }
}

async fn handle_world(
  request: &OmniRequest,
  credentials: &ProviderCredentialLoadingCache,
  local_media: &LocalMediaStore,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let model = request.model().ok_or_else(GenerateError::no_model_specified)?;
  let api_key = load_api_key(credentials, ProviderCredentialKey::WorldLabsApiKey, "World Labs")?;
  let prompt = request.fields.get("prompt").and_then(Value::as_str);
  let picture = first_picture(request, local_media)?;
  enqueue_world(&api_key, model, prompt, picture.as_ref()).await
}

fn first_picture(request: &OmniRequest, local_media: &LocalMediaStore) -> Result<Option<LocalPicture>, GenerateError> {
  let token = request.fields.get("reference_image_media_tokens")
    .and_then(Value::as_array)
    .and_then(|tokens| tokens.first())
    .and_then(Value::as_str)
    .map(MediaFileToken::new_from_str);
  match token {
    Some(token) => LocalPicture::read(&token, local_media).map(Some),
    None => Ok(None),
  }
}
