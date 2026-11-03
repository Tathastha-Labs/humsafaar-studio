use crate::core::commands::response::shorthand::InfallibleResponse;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use crate::core::state::app_env_configs::app_env_configs::AppEnvConfigs;
use crate::core::state::app_preferences::app_preferences_manager::AppPreferencesManager;
use crate::core::state::humsafaar_platform_info::{HumsafaarOs, HumsafaarPlatformInfo};
use crate::core::state::data_dir::app_data_root::AppDataRoot;
use crate::core::state::data_dir::trait_data_subdir::DataSubdir;
use crate::core::state::os_platform::OsPlatform;
use chrono::{DateTime, Utc};
use log::{info, warn};
use serde_derive::Serialize;
use std::path::PathBuf;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct AppInfoResponse {
  pub humsafaar_version: String,

  pub build_timestamp: DateTime<Utc>,

  pub git_commit_id: Option<String>,
  pub git_commit_short_id: Option<String>,
  pub git_commit_timestamp: Option<DateTime<Utc>>,

  pub os_platform: DetectedOs,
  pub os_version: String,

  pub humsafaar_root_directory: PathBuf,
  pub download_directory: PathBuf,
}

impl SerializeMarker for AppInfoResponse {}

#[derive(Copy, Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectedOs {
  Windows,
  #[serde(rename = "macos")]
  MacOS,
  Linux,
  Unknown,
}

#[tauri::command]
pub fn get_app_info_command(
  app_data_root: State<'_, AppDataRoot>,
  app_env_configs: State<'_, AppEnvConfigs>,
  humsafaar_platform_info: State<'_, HumsafaarPlatformInfo>,
  app_prefs: State<'_, AppPreferencesManager>,
) -> InfallibleResponse<AppInfoResponse> {
  info!("get_app_info_command called...");
  
  let os_platform = match humsafaar_platform_info.os_platform {
    HumsafaarOs::Linux => DetectedOs::Linux,
    HumsafaarOs::MacOS => DetectedOs::MacOS,
    HumsafaarOs::Windows => DetectedOs::Windows,
    HumsafaarOs::Unknown => DetectedOs::Unknown,
  };

  let root_directory = app_data_root.path().to_path_buf();

  let download_directory = match app_prefs.get_clone() {
    Ok(app_prefs) => {
      app_prefs.preferred_download_directory
          .download_directory(&app_data_root)
    }
    Err(err) => {
      warn!("Can't get user download directory: {:?}", err);
      app_data_root.downloads_dir().path().to_path_buf()
    }
  };

  AppInfoResponse {
    humsafaar_version: humsafaar_platform_info.humsafaar_version.clone(),
    build_timestamp: humsafaar_platform_info.build_timestamp,
    git_commit_id: humsafaar_platform_info.git_commit_id.clone(),
    git_commit_short_id: humsafaar_platform_info.git_commit_short_id.clone(),
    git_commit_timestamp: humsafaar_platform_info.git_commit_timestamp,
    os_platform,
    os_version: humsafaar_platform_info.os_version.clone(),
    humsafaar_root_directory: root_directory,
    download_directory,
  }.into()
}

