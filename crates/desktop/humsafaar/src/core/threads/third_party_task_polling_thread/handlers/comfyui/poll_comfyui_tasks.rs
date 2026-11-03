use crate::core::local_models::comfyui_engine::running_base_url;
use crate::core::local_models::comfyui_settings::ComfyUiSettings;
use crate::core::state::app_env_configs::app_env_configs::AppEnvConfigs;
use crate::core::state::data_dir::app_data_root::AppDataRoot;
use crate::core::state::data_dir::trait_data_subdir::DataSubdir;
use crate::core::state::task_database::TaskDatabase;
use crate::core::threads::third_party_task_polling_thread::handlers::complete_task_locally::complete_task_locally;
use crate::core::threads::third_party_task_polling_thread::handlers::fal::handle_fal_failure::handle_fal_failure;
use comfyui_client::client::ComfyUiClient;
use comfyui_client::error::ComfyUiError;
use comfyui_client::history::JobState;
use comfyui_manager::manager::{ComfyUiManager, EngineStatus};
use enums::tauri::tasks::task_media_file_class::TaskMediaFileClass;
use enums::tauri::tasks::task_type::TaskType;
use log::{error, info, warn};
use sqlite_tasks::queries::task::Task;
use std::path::PathBuf;
use tauri::AppHandle;
use uuid_utils::uuid::generate_random_uuid;

/// Check each pending local ComfyUI job once; finish the ones that are done.
pub async fn poll_comfyui_tasks(
  app_handle: &AppHandle,
  app_env_configs: &AppEnvConfigs,
  app_data_root: &AppDataRoot,
  task_database: &TaskDatabase,
  settings: &ComfyUiSettings,
  manager: &ComfyUiManager,
  tasks: &[&Task],
) {
  let Some(base_url) = running_base_url(settings, manager) else {
    // Jobs live only in the engine's memory, so none survive it stopping.
    if !matches!(manager.status(), EngineStatus::Starting | EngineStatus::Installing { .. }) {
      for task in tasks {
        handle_fal_failure(app_handle, task_database, task, "The local engine stopped before this finished.").await;
      }
    }
    return;
  };
  let client = ComfyUiClient::new(&base_url);

  for task in tasks {
    let Some(prompt_id) = task.provider_job_id.as_deref() else {
      handle_fal_failure(app_handle, task_database, task, "Local job has no id").await;
      continue;
    };

    match client.job_state(prompt_id).await {
      Ok(JobState::Pending) => {
        // A job that is in neither the queue nor the history is gone: the engine restarted.
        // History is checked again so a job that just finished is not mistaken for a lost one.
        let missing = matches!(client.queue_contains(prompt_id).await, Ok(false))
          && matches!(client.job_state(prompt_id).await, Ok(JobState::Pending));
        if missing {
          handle_fal_failure(app_handle, task_database, task, "The local engine lost this job because it restarted.").await;
        }
      }
      Ok(JobState::Failed(message)) => {
        handle_fal_failure(app_handle, task_database, task, &message).await;
      }
      Ok(JobState::Done(images)) if images.is_empty() => {
        handle_fal_failure(app_handle, task_database, task, "ComfyUI finished without any images").await;
      }
      Ok(JobState::Done(images)) => {
        info!("[ComfyUiPolling] Task {} finished with {} image(s)", task.id.as_str(), images.len());
        let result = fetch_and_complete(
          app_handle, app_env_configs, app_data_root, task_database, &client, task, &images,
        ).await;
        if let Err(err) = result {
          error!("[ComfyUiPolling] Could not complete task {}: {:?}", task.id.as_str(), err);
          handle_fal_failure(app_handle, task_database, task, "Could not save the generated image").await;
        }
      }
      // The server being down is not the job failing; ComfyUI may be restarted.
      Err(ComfyUiError::NotReachable(url)) => {
        warn!("[ComfyUiPolling] ComfyUI not reachable at {}; will retry", url);
        return;
      }
      Err(err) => {
        warn!("[ComfyUiPolling] Error checking task {}: {:?}", task.id.as_str(), err);
      }
    }
  }
}

async fn fetch_and_complete(
  app_handle: &AppHandle,
  app_env_configs: &AppEnvConfigs,
  app_data_root: &AppDataRoot,
  task_database: &TaskDatabase,
  client: &ComfyUiClient,
  task: &Task,
  images: &[comfyui_client::history::OutputImage],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let media_class = match task.task_type {
    TaskType::VideoGeneration => TaskMediaFileClass::Video,
    TaskType::AudioGeneration => TaskMediaFileClass::Audio,
    _ => TaskMediaFileClass::Image,
  };

  let mut files: Vec<PathBuf> = Vec::with_capacity(images.len());
  for image in images {
    let bytes = client.download_image(image).await?;
    let extension = image.filename.rsplit('.').next().unwrap_or("png");
    let path = app_data_root.temp_dir().path().join(format!("comfyui_{}.{}", generate_random_uuid(), extension));
    std::fs::write(&path, bytes)?;
    files.push(path);
  }

  complete_task_locally(
    app_handle,
    app_env_configs,
    app_data_root,
    task_database,
    task,
    files,
    media_class,
  ).await
}
