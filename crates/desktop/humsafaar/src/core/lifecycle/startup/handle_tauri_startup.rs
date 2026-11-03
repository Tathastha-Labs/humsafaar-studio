use crate::core::lifecycle::startup::tasks::bootstrap_task_database::bootstrap_task_database;
use crate::core::lifecycle::startup::tasks::initially_size_and_position_windows::initially_size_and_position_windows;
use crate::core::lifecycle::startup::tasks::load_provider_priority_state::load_provider_priority_state;
use crate::core::lifecycle::startup::tasks::set_app_log_level::set_app_log_level;
use crate::core::lifecycle::startup::tasks::spawn_main_window_thread::spawn_main_window_thread;
use crate::core::state::app_env_configs::app_env_configs::AppEnvConfigs;
use crate::core::state::humsafaar_platform_info::HumsafaarPlatformInfo;
use crate::core::state::humsafaar_usage_tracker::humsafaar_usage_tracker::HumsafaarUsageTracker;
use crate::core::state::data_dir::app_data_root::AppDataRoot;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;
use crate::core::threads::third_party_task_polling_thread::third_party_task_polling_thread::third_party_task_polling_thread;
use errors::AnyhowResult;
use crate::core::local_models::comfyui_settings::ComfyUiSettings;
use crate::core::threads::comfyui_idle_stop_thread::comfyui_idle_stop_thread;
use comfyui_manager::manager::ComfyUiManager;
use tauri::{AppHandle, Manager};

pub async fn handle_tauri_startup(
  app: AppHandle,
  root: AppDataRoot,
  app_env_configs: AppEnvConfigs,
  humsafaar_platform_info: HumsafaarPlatformInfo,
  humsafaar_usage_tracker: HumsafaarUsageTracker,
  credential_cache: ProviderCredentialLoadingCache,
) -> AnyhowResult<()> {

  set_app_log_level(
    &app,
    &root,
  )?;

  let task_database =
      bootstrap_task_database(&app, &root).await?;

  load_provider_priority_state(
    &app,
    &root,
  )?;

  spawn_main_window_thread(
    &app,
    &root,
  )?;

  tauri::async_runtime::spawn(third_party_task_polling_thread(
    app.clone(),
    app_env_configs.clone(),
    root.clone(),
    task_database.clone(),
    credential_cache,
  ));

  tauri::async_runtime::spawn(comfyui_idle_stop_thread(
    app.state::<ComfyUiManager>().inner().clone(),
    app.state::<ComfyUiSettings>().inner().clone(),
    task_database.clone(),
  ));

  initially_size_and_position_windows(&app, &root);

  Ok(())
}
