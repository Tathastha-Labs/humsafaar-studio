use serde_json::{Map, Value};
use tauri::{AppHandle, Manager};
use tokens::tokens::media_files::MediaFileToken;
use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_type::TaskType;
use log::{error, info};

use crate::core::cloud_models::fal_mesh::{fal_mesh_job, is_supported, MeshInputs};
use crate::core::cloud_models::local_picture::LocalPicture;
use crate::core::cloud_models::replicate_queue::{enqueue_on_replicate, load_api_key, picture_for_replicate};
use crate::core::cloud_models::replicate_recipes::{is_mesh_supported as replicate_mesh_supported, replicate_mesh_job};
use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::cloud_models::fal_queue::{enqueue_on_fal, load_fal_api_key};
use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::commands::generate::enqueued_job::record_enqueued_job;
use crate::core::commands::generate::omni::dispatch::local_error;
use crate::core::commands::generate::omni::{OmniRequest, OmniResult};
use crate::core::local_media::local_media_store::LocalMediaStore;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;

#[tauri::command]
pub async fn generate_mesh_command(request: OmniRequest, app: AppHandle) -> OmniResult {
  if matches!(request.provider, Some(GenerationProvider::Fal | GenerationProvider::Replicate)) {
    return generate_cloud_mesh(request, &app).await;
  }
  Err(local_error("Pick a provider for this model first."))
}

/// A 3D object on a cloud provider with the user's own key.
async fn generate_cloud_mesh(request: OmniRequest, app: &AppHandle) -> OmniResult {
  let cache = app.state::<ProviderCredentialLoadingCache>();
  let local_media = app.state::<LocalMediaStore>();
  match handle_cloud_mesh(&request, &cache, &local_media).await {
    Ok(success) => record_enqueued_job(&request, success, app).await,
    Err(err) => {
      error!("3D generation failed: {:?}", err);
      Err(local_error(format!("{:?}", err)))
    }
  }
}

async fn handle_cloud_mesh(
  request: &OmniRequest,
  credentials: &ProviderCredentialLoadingCache,
  local_media: &LocalMediaStore,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let fields = &request.fields;
  let model = fields.get("model").and_then(Value::as_str).ok_or_else(GenerateError::no_model_specified)?;
  let provider = request.provider.unwrap_or(GenerationProvider::Fal);
  let supported = match provider {
    GenerationProvider::Replicate => replicate_mesh_supported(model),
    _ => is_supported(model),
  };
  if !supported {
    return Err(GenerateError::NotYetImplemented(format!("{} is not available on {:?} yet", model, provider)));
  }

  let picture = first_picture(fields, local_media)?;
  let text = |key: &str| fields.get(key).and_then(Value::as_str);
  let mut inputs = MeshInputs {
    prompt: text("prompt"),
    image: None,
    output_type: text("mesh_output_type"),
    polygon_type: text("polygon_type"),
    face_count: fields.get("face_count").and_then(Value::as_u64).map(|n| n as u32),
    enable_pbr: fields.get("enable_pbr").and_then(Value::as_bool),
    enable_texture: fields.get("enable_texture").and_then(Value::as_bool),
  };

  match provider {
    GenerationProvider::Replicate => {
      let api_key = load_api_key(credentials, ProviderCredentialKey::ReplicateApiKey, "Replicate")?;
      if let Some(picture) = &picture {
        inputs.image = Some(picture_for_replicate(&api_key, picture).await);
      }
      let job = replicate_mesh_job(model, &inputs).map_err(|message| GenerateError::AnyhowError(anyhow::anyhow!(message)))?;
      info!("Queueing Replicate 3D: {} ({})", job.model, model);
      enqueue_on_replicate(job.model, job.input, &api_key, TaskType::ObjectGeneration, model).await
    }
    _ => {
      inputs.image = picture.as_ref().map(LocalPicture::data_uri);
      let job = fal_mesh_job(model, &inputs).map_err(|message| GenerateError::AnyhowError(anyhow::anyhow!(message)))?;
      let api_key = load_fal_api_key(credentials)?;
      info!("Queueing Fal 3D: {} ({})", job.endpoint, model);
      enqueue_on_fal(job.endpoint, job.input, &api_key, TaskType::ObjectGeneration, model).await
    }
  }
}

/// The first picture the user attached, read from this computer.
fn first_picture(fields: &Map<String, Value>, local_media: &LocalMediaStore) -> Result<Option<LocalPicture>, GenerateError> {
  let token = fields.get("reference_image_media_tokens")
    .and_then(Value::as_array)
    .and_then(|tokens| tokens.first())
    .and_then(Value::as_str)
    .map(MediaFileToken::new_from_str);
  match token {
    Some(token) => LocalPicture::read(&token, local_media).map(Some),
    None => Ok(None),
  }
}
