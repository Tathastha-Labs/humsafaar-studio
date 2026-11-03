use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use crate::core::local_models::comfyui_settings::ComfyUiSettings;
use comfyui_manager::manager::ComfyUiManager;
use log::info;
use serde_derive::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct ComfyUiStopSuccessResponse {}

impl SerializeMarker for ComfyUiStopSuccessResponse {}

#[tauri::command]
pub async fn comfyui_stop_command(
  settings: State<'_, ComfyUiSettings>,
  manager: State<'_, ComfyUiManager>,
) -> ResponseOrErrorMessage<ComfyUiStopSuccessResponse> {
  if settings.is_external() {
    return Err("The local engine is managed outside the app.".into());
  }

  info!("comfyui_stop_command called");
  manager.stop().await;
  Ok(ComfyUiStopSuccessResponse {}.into())
}
