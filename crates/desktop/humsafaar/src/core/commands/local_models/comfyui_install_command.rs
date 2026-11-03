use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use crate::core::local_models::comfyui_settings::ComfyUiSettings;
use comfyui_manager::manager::{ComfyUiManager, EngineStatus};
use comfyui_manager::models::STABLE_DIFFUSION_1_5;
use log::{error, info};
use serde_derive::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct ComfyUiInstallSuccessResponse {}

impl SerializeMarker for ComfyUiInstallSuccessResponse {}

/// Install the local engine and the starter model, then start it.
/// Returns at once; follow progress with `comfyui_get_status_command`.
#[tauri::command]
pub async fn comfyui_install_command(
  settings: State<'_, ComfyUiSettings>,
  manager: State<'_, ComfyUiManager>,
) -> ResponseOrErrorMessage<ComfyUiInstallSuccessResponse> {
  if settings.is_external() {
    return Err("The local engine is managed outside the app.".into());
  }
  if matches!(manager.status(), EngineStatus::Installing { .. }) {
    return Ok(ComfyUiInstallSuccessResponse {}.into());
  }

  info!("comfyui_install_command: starting");
  let manager = manager.inner().clone();
  tauri::async_runtime::spawn(async move {
    if let Err(err) = manager.install(&[STABLE_DIFFUSION_1_5]).await {
      error!("Local engine install failed: {}", err);
      return;
    }
    if let Err(err) = manager.start().await {
      error!("Local engine did not start after installing: {}", err);
    }
  });

  Ok(ComfyUiInstallSuccessResponse {}.into())
}
