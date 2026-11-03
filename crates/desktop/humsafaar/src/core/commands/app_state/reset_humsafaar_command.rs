use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use crate::core::state::app_preferences::app_preferences_manager::AppPreferencesManager;
use crate::core::state::app_preferences::preferred_download_directory::PreferredDownloadDirectory;
use crate::core::state::data_dir::app_data_root::AppDataRoot;
use anyhow::anyhow;
use errors::AnyhowResult;
use log::{error, info};
use serde_derive::{Deserialize, Serialize};
use tauri::{AppHandle, State};

#[derive(Serialize)]
pub struct ResetHumsafaarCommandResponse {
  pub success: bool,
}

impl SerializeMarker for ResetHumsafaarCommandResponse {}

#[tauri::command]
pub async fn reset_humsafaar_command(
  app: AppHandle,
) -> ResponseOrErrorMessage<ResetHumsafaarCommandResponse> {
  info!("reset_humsafaar_command called");

  // TODO: Reset local vs. production

  Ok(ResetHumsafaarCommandResponse {
    success: true,
  }.into())
}

