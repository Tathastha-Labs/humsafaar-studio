use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::commands::generate::enqueued_job::record_enqueued_job;
use crate::core::commands::generate::generate_audio::handle_comfyui_audio::handle_comfyui_audio;
use crate::core::commands::generate::generate_audio::handle_fal_audio::handle_fal_audio;
use crate::core::commands::generate::generate_audio::handle_replicate_audio::handle_replicate_audio;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;
use crate::core::commands::generate::omni::dispatch::local_error;
use crate::core::commands::generate::omni::{OmniRequest, OmniResult};
use crate::core::local_models::comfyui_settings::ComfyUiSettings;
use comfyui_manager::manager::ComfyUiManager;
use enums::common::generation_provider::GenerationProvider;
use log::error;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub async fn generate_audio_command(request: OmniRequest, app: AppHandle) -> OmniResult {
  match request.provider {
    Some(GenerationProvider::ComfyUi) => return generate_local_audio(request, &app).await,
    Some(GenerationProvider::Fal) => return generate_fal_audio(request, &app).await,
    Some(GenerationProvider::Replicate) => return generate_replicate_audio(request, &app).await,
    _ => {}
  }
  Err(local_error("Pick a provider for this model first."))
}

/// Queue a song on the local engine. The page follows it through the task queue like any other job.
async fn generate_local_audio(request: OmniRequest, app: &AppHandle) -> OmniResult {
  let settings = app.state::<ComfyUiSettings>();
  let manager = app.state::<ComfyUiManager>();

  match handle_comfyui_audio(&request.fields, &settings, &manager).await {
    Ok(success) => record_enqueued_job(&request, success, app).await,
    Err(err) => fail(&request, err, app).await,
  }
}

/// Music on Fal with the user's own key.
async fn generate_fal_audio(request: OmniRequest, app: &AppHandle) -> OmniResult {
  let cache = app.state::<ProviderCredentialLoadingCache>();
  match handle_fal_audio(&request.fields, &cache).await {
    Ok(success) => record_enqueued_job(&request, success, app).await,
    Err(err) => fail(&request, err, app).await,
  }
}

/// Music on Replicate with the user's own key.
async fn generate_replicate_audio(request: OmniRequest, app: &AppHandle) -> OmniResult {
  let cache = app.state::<ProviderCredentialLoadingCache>();
  match handle_replicate_audio(&request.fields, &cache).await {
    Ok(success) => record_enqueued_job(&request, success, app).await,
    Err(err) => fail(&request, err, app).await,
  }
}

async fn fail(_request: &OmniRequest, err: GenerateError, app: &AppHandle) -> OmniResult {
  error!("Audio generation failed: {:?}", err);
  Err(local_error(format!("{:?}", err)))
}
