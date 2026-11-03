use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use comfyui_manager::manager::ComfyUiManager;
use comfyui_manager::models::catalog_model;
use log::info;
use serde_derive::{Deserialize, Serialize};
use tauri::State;

#[derive(Deserialize)]
pub struct ComfyUiDownloadModelRequest {
  /// REQUIRED.
  /// A catalog model id, eg. "stable_diffusion_xl".
  pub model_id: String,
}

#[derive(Serialize)]
pub struct ComfyUiDownloadModelSuccessResponse {}

impl SerializeMarker for ComfyUiDownloadModelSuccessResponse {}

/// Start downloading a model. Returns at once; follow it with `comfyui_get_status_command`.
#[tauri::command]
pub async fn comfyui_download_model_command(
  request: ComfyUiDownloadModelRequest,
  manager: State<'_, ComfyUiManager>,
) -> ResponseOrErrorMessage<ComfyUiDownloadModelSuccessResponse> {
  let model = catalog_model(&request.model_id).ok_or("Unknown model")?;

  info!("comfyui_download_model_command called for {}", model.id);
  manager.download_model(*model).map_err(|err| err.to_string())?;

  Ok(ComfyUiDownloadModelSuccessResponse {}.into())
}
