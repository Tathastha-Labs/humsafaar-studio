use crate::core::local_media::local_media_store::LocalMediaStore;
use crate::core::state::app_env_configs::app_env_configs::AppEnvConfigs;
use crate::core::state::data_dir::app_data_root::AppDataRoot;
use crate::core::state::task_database::TaskDatabase;
use crate::core::threads::third_party_task_polling_thread::events::notify_frontend_of_completion::{
  notify_frontend_of_completion, CompletionData,
};
use crate::core::utils::auto_download::clear_auto_download_checkpoint;
use enums::tauri::tasks::task_media_file_class::TaskMediaFileClass;
use log::{info, warn};
use sqlite_tasks::queries::task::Task;
use sqlite_tasks::queries::update_successful_task_status_with_metadata::{
  update_successful_task_status_with_metadata, UpdateSuccessfulTaskArgs,
};
use std::path::PathBuf;
use tauri::AppHandle;
use tokens::tokens::media_files::MediaFileToken;

/// Move finished files into the local media store, mark the task complete and tell the frontend.
/// `files` are temporary files on disk, in result order; they are removed once stored.
pub async fn complete_task_locally(
  app_handle: &AppHandle,
  app_env_configs: &AppEnvConfigs,
  app_data_root: &AppDataRoot,
  task_database: &TaskDatabase,
  task: &Task,
  files: Vec<PathBuf>,
  media_class: TaskMediaFileClass,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  if files.is_empty() {
    return Err("no result files to store".into());
  }

  let store = LocalMediaStore::from_app_data_root(app_data_root);
  let mut local_batch: Vec<(MediaFileToken, reqwest::Url)> = Vec::with_capacity(files.len());

  for file in &files {
    let token = store.put_file(file)?;
    if let Err(err) = std::fs::remove_file(file) {
      warn!("[LocalComplete] Could not remove temp file {:?}: {:?}", file, err);
    }
    let stored_path = store.path_for_token(&token).ok_or("stored media file is missing")?;
    let asset_url = reqwest::Url::parse(&LocalMediaStore::asset_url_for_path(&stored_path))?;
    local_batch.push((token, asset_url));
  }

  let (primary_token, primary_url) = local_batch[0].clone();

  let updated = update_successful_task_status_with_metadata(UpdateSuccessfulTaskArgs {
    db: task_database.get_connection(),
    task_id: &task.id,
    maybe_batch_token: None,
    maybe_primary_media_file_token: Some(&primary_token),
    maybe_primary_media_file_class: Some(media_class),
    maybe_primary_media_file_cdn_url: Some(primary_url.as_str()),
    maybe_primary_media_file_thumbnail_url_template: None,
  }).await?;

  if updated {
    clear_auto_download_checkpoint(app_handle, task);
    let completion = CompletionData {
      primary_media_file_token: primary_token,
      maybe_cdn_url: Some(primary_url),
      maybe_thumbnail_url_template: None,
      maybe_batch_token: None,
      media_class,
      local_batch,
    };
    notify_frontend_of_completion(
      app_handle,
      task,
      &completion,
    ).await;
  }

  info!("[LocalComplete] Task {} saved locally", task.id.as_str());
  Ok(())
}
