use crate::core::commands::generate::generate_script_command::generate_script_command;
use crate::core::commands::generate::generate_splat_command::generate_splat_command;
use crate::core::commands::generate::generate_audio_command::generate_audio_command;
use crate::core::commands::generate::generate_mesh_command::generate_mesh_command;
pub mod core;
pub mod version;

use tauri::Manager;

use crate::core::commands::app_preferences::get_app_preferences_command::get_app_preferences_command;
use crate::core::commands::app_preferences::update_app_preference_command::update_app_preferences_command;
use crate::core::commands::download::download_directory_reveal_command::download_directory_reveal_command;
use crate::core::commands::download::download_url_command::download_url_command;
use crate::core::commands::download::get_download_path_command::get_download_path_command;
use crate::core::commands::enqueue::image_bg_removal::enqueue_image_bg_removal_command::enqueue_image_bg_removal_command;
use crate::core::commands::generate::generate_image::generate_image_command::generate_image_command;
use crate::core::commands::generate::generate_video::generate_video_command::generate_video_command;
use crate::core::commands::flip_image::flip_image;
use crate::core::commands::get_app_info_command::get_app_info_command;
use crate::core::commands::load_without_cors_command::load_without_cors_command;
use crate::core::commands::local_models::comfyui_delete_model_command::comfyui_delete_model_command;
use crate::core::commands::local_models::comfyui_download_model_command::comfyui_download_model_command;
use crate::core::commands::local_models::comfyui_get_status_command::comfyui_get_status_command;
use crate::core::commands::local_models::comfyui_install_command::comfyui_install_command;
use crate::core::commands::local_models::comfyui_start_command::comfyui_start_command;
use crate::core::commands::local_models::comfyui_stop_command::comfyui_stop_command;
use crate::core::commands::voiceover::groq_voiceover_generate_command::groq_voiceover_generate_command;
use crate::core::commands::voiceover::transcribe_audio_command::transcribe_audio_command;
use crate::core::commands::voiceover::voiceover_generate_command::voiceover_generate_command;
use crate::core::commands::voiceover::voiceover_get_status_command::voiceover_get_status_command;
use crate::core::commands::voiceover::voiceover_install_command::voiceover_install_command;
use crate::core::commands::media_files::local_media_get_url_command::local_media_get_url_command;
use crate::core::commands::media_files::local_media_import_command::local_media_import_command;
use crate::core::commands::media_files::local_media_list_command::local_media_list_command;
use crate::core::commands::media_files::local_media_read_text_command::local_media_read_text_command;
use crate::core::commands::media_files::local_media_replace_command::local_media_replace_command;
use crate::core::commands::media_files::media_file_delete_command::media_file_delete_command;
use crate::core::commands::providers::deprecated::get_provider_order_command::get_provider_order_command;
use crate::core::commands::providers::deprecated::set_provider_order_command::set_provider_order_command;
use crate::core::commands::providers::provider_clear_command::provider_clear_command;
use crate::core::commands::providers::provider_list_command::provider_list_command;
use crate::core::commands::providers::provider_set_api_key_command::provider_set_api_key_command;
use crate::core::commands::task_queue::get_task_queue_command::get_task_queue_command;
use crate::core::commands::task_queue::mark_task_as_dismissed_command::mark_task_as_dismissed_command;
use crate::core::commands::task_queue::tasks_nuke_all_command::tasks_nuke_all_command;
use crate::core::lifecycle::startup::handle_tauri_startup::handle_tauri_startup;
use crate::core::lifecycle::startup::setup_main_window::setup_main_window;
use crate::core::state::app_env_configs::app_env_configs::AppEnvConfigs;
use crate::core::state::app_preferences::app_preferences_manager::load_app_preferences_or_default;
use crate::core::state::humsafaar_platform_info::HumsafaarPlatformInfo;
use crate::core::state::data_dir::app_data_root::AppDataRoot;
use crate::core::state::provider_priority::ProviderPriorityStore;
use crate::core::threads::main_window_thread::main_window_thread::main_window_thread;
use log::error;

use crate::core::state::humsafaar_usage_tracker::humsafaar_usage_tracker::HumsafaarUsageTracker;
use tauri_plugin_dialog;
use tauri_plugin_http;
use tauri_plugin_log::Target;
use tauri_plugin_log::TargetKind;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  // NB: Tauri wants to install the logger itself, so we can't rely on the logger crate
  // until the tauri runtime begins.
  println!("Loading config...");
  let app_data_root = AppDataRoot::create_default().expect("data directory should be created");
  let app_data_root_2 = app_data_root.clone();

  // The local image engine lives in its own folder inside the data directory.
  let comfyui_manager = comfyui_manager::manager::ComfyUiManager::new(
    comfyui_manager::layout::RuntimeLayout::new(app_data_root.path().join("comfyui")),
  );
  let comfyui_manager_for_exit = comfyui_manager.clone();

  println!("Getting platform info...");
  let humsafaar_platform_info = HumsafaarPlatformInfo::get();
  let humsafaar_platform_info_2 = humsafaar_platform_info.clone();

  println!("Platform info: {:?}", humsafaar_platform_info);

  println!("Loading app preferences...");
  let app_preferences = load_app_preferences_or_default(&app_data_root);
  
  let provider_credential_cache = crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache::new(app_data_root.clone());
  let provider_credential_cache_2 = provider_credential_cache.clone();

  // NB: tauri-plugin-http stores the credentials on disk, so we can defer to that for now.
  // println!("Attempting to read existing humsafaar credentials...");
  
  
  // Other state

  let app_env_configs = AppEnvConfigs::load_from_filesystem(&app_data_root)
    .expect("AppEnvConfigs should be loaded from disk");
  
  let app_env_configs_2 = app_env_configs.clone();





  
  let humsafaar_usage_tracker = HumsafaarUsageTracker::new();
  let humsafaar_usage_tracker_2 = humsafaar_usage_tracker.clone();

  println!("Initializing backend runtime...");

  let builder = tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_http::init())
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_upload::init())
    .setup(move |app| {
      // TODO(bt): This is broken on windows
      // log_environment_details();

      //if cfg!(debug_assertions) {
      //  app.handle().plugin(
      //    tauri_plugin_log::Builder::default()
      //      .level(log::LevelFilter::Info)
      //      .build(),
      //  )?;
      //}
      let app = app.handle().clone();
      let handle = app.clone();
      let root = app_data_root_2.clone();
      let env_config = app_env_configs_2.clone();

      tauri::async_runtime::block_on(async move {
        let result = setup_main_window(&app).await;

        let result = handle_tauri_startup(
          handle,
          root,
          env_config,
          humsafaar_platform_info_2,
          humsafaar_usage_tracker_2,
          provider_credential_cache_2,
        ).await;

        if let Err(err) = result {
          error!("Failed to handle Tauri startup: {:?}", err);
          panic!("Failed to handle Tauri startup: {:?}", err);
        }
      });

      Ok(())
    })
    .manage(crate::core::local_media::local_media_store::LocalMediaStore::from_app_data_root(&app_data_root))
    .manage(crate::core::local_models::comfyui_settings::ComfyUiSettings::from_env())
    .manage(comfyui_manager)
    .manage(crate::core::local_models::voiceover_state::VoiceoverState::new())
    .manage(app_data_root)
    .manage(app_env_configs)
    .manage(app_preferences)
    .manage(humsafaar_platform_info)
    .manage(humsafaar_usage_tracker)
    .manage(provider_credential_cache);

  // TODO: Break this out into another module, because RustRover/IntelliJ lags with these macros.
  //  My first attempt at naively doing this didn't work because the macros can't find their codegen'd targets.
  let builder = builder.invoke_handler(tauri::generate_handler![
    comfyui_delete_model_command,
    comfyui_download_model_command,
    comfyui_get_status_command,
    comfyui_install_command,
    comfyui_start_command,
    comfyui_stop_command,
    voiceover_generate_command,
    groq_voiceover_generate_command,
    transcribe_audio_command,
    voiceover_get_status_command,
    voiceover_install_command,
    download_directory_reveal_command,
    download_url_command,
    get_download_path_command,
    enqueue_image_bg_removal_command,
    flip_image,
    generate_image_command,
    generate_video_command,
    generate_script_command,
    generate_splat_command,
    generate_audio_command,
    generate_mesh_command,
    get_app_info_command,
    get_app_preferences_command,
    get_provider_order_command,
    get_task_queue_command,
    provider_clear_command,
    provider_list_command,
    provider_set_api_key_command,
    load_without_cors_command,
    mark_task_as_dismissed_command,
    media_file_delete_command,
    local_media_import_command,
    local_media_list_command,
    local_media_read_text_command,
    local_media_replace_command,
    local_media_get_url_command,
    set_provider_order_command,
    tasks_nuke_all_command,
    update_app_preferences_command,
  ]);

  let app = builder.build(tauri::generate_context!("tauri.conf.json"))
    .expect("error while building tauri application");

  app.run(move |_app_handle, event| {
    // Do not leave the local engine running after the app is gone.
    if let tauri::RunEvent::Exit = event {
      tauri::async_runtime::block_on(comfyui_manager_for_exit.stop());
    }
  });
}
