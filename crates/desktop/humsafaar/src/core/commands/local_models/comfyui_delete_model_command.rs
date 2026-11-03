use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use comfyui_manager::manager::ComfyUiManager;
use comfyui_manager::models::catalog_model;
use log::info;
use serde_derive::{Deserialize, Serialize};
use tauri::State;

#[derive(Deserialize)]
pub struct ComfyUiDeleteModelRequest {
  /// REQUIRED.
  /// A catalog model id, eg. "stable_diffusion_xl".
  pub model_id: String,
}

#[derive(Serialize)]
pub struct ComfyUiDeleteModelSuccessResponse {}

impl SerializeMarker for ComfyUiDeleteModelSuccessResponse {}

/// Delete a downloaded model to free its disk space.
#[tauri::command]
pub async fn comfyui_delete_model_command(
  request: ComfyUiDeleteModelRequest,
  manager: State<'_, ComfyUiManager>,
) -> ResponseOrErrorMessage<ComfyUiDeleteModelSuccessResponse> {
  let model = catalog_model(&request.model_id).ok_or("Unknown model")?;

  info!("comfyui_delete_model_command called for {}", model.id);
  manager.delete_model(model).map_err(|err| err.to_string())?;

  Ok(ComfyUiDeleteModelSuccessResponse {}.into())
}
