use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use crate::core::local_models::comfyui_settings::ComfyUiSettings;
use comfyui_manager::manager::ComfyUiManager;
use log::{error, info};
use serde_derive::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct ComfyUiStartSuccessResponse {}

impl SerializeMarker for ComfyUiStartSuccessResponse {}

/// Start the engine. Returns at once; follow progress with `comfyui_get_status_command`.
#[tauri::command]
pub async fn comfyui_start_command(
  settings: State<'_, ComfyUiSettings>,
  manager: State<'_, ComfyUiManager>,
) -> ResponseOrErrorMessage<ComfyUiStartSuccessResponse> {
  if settings.is_external() {
    return Err("The local engine is managed outside the app.".into());
  }

  info!("comfyui_start_command called");
  let manager = manager.inner().clone();
  tauri::async_runtime::spawn(async move {
    if let Err(err) = manager.start().await {
      error!("Local engine did not start: {}", err);
    }
  });

  Ok(ComfyUiStartSuccessResponse {}.into())
}
