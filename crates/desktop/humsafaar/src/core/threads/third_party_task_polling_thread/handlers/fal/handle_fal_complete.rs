use crate::core::state::app_env_configs::app_env_configs::AppEnvConfigs;
use crate::core::state::data_dir::app_data_root::AppDataRoot;
use crate::core::state::data_dir::trait_data_subdir::DataSubdir;
use crate::core::state::task_database::TaskDatabase;
use crate::core::threads::third_party_task_polling_thread::handlers::complete_task_locally::complete_task_locally;
use crate::core::utils::auto_download::auto_download_task_urls;
use enums::tauri::tasks::task_media_file_class::TaskMediaFileClass;
use fal_client::polling::poll_job_response::poll_job_response::PollJobResponse;
use fal_client::polling::poll_job_response::success_case_extractors::PollResponseExtractedContents;
use log::{error, info, warn};
use sqlite_tasks::queries::task::Task;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tauri::AppHandle;
use uuid_utils::uuid::generate_random_uuid;

/// A Fal job finished: download what it made and save it on this computer.
pub async fn handle_fal_complete(
  app_handle: &AppHandle,
  app_env_configs: &AppEnvConfigs,
  app_data_root: &AppDataRoot,
  task_database: &TaskDatabase,
  task: &Task,
  job_response: PollJobResponse,
) {
  info!("[FalComplete] Handling completed task {}", task.id.as_str());

  let result = handle_fal_complete_local(app_handle, app_env_configs, app_data_root, task_database, task, job_response).await;

  if let Err(err) = result {
    error!("[FalComplete] Failed to handle task {}: {:?}", task.id.as_str(), err);
  }
}

async fn handle_fal_complete_local(
  app_handle: &AppHandle,
  app_env_configs: &AppEnvConfigs,
  app_data_root: &AppDataRoot,
  task_database: &TaskDatabase,
  task: &Task,
  job_response: PollJobResponse,
) -> Result<(), Box<dyn std::error::Error>> {

  let (urls, media_class) = collect_media_urls(task, &job_response.extracted_contents)?;

  if urls.is_empty() {
    warn!("[FalComplete] Task {} completed but no downloadable media found in response", task.id.as_str());
    return Ok(());
  }

  let download_urls = urls.iter().map(|url| reqwest::Url::parse(url)).collect::<Result<Vec<_>, _>>()?;
  auto_download_task_urls(app_handle, task, &download_urls).await?;

  let mut downloaded = Vec::with_capacity(urls.len());
  for (i, url) in urls.iter().enumerate() {
    downloaded.push(download_file(url, app_data_root, i).await?);
  }

  complete_task_locally(
    app_handle,
    app_env_configs,
    app_data_root,
    task_database,
    task,
    downloaded,
    media_class,
  ).await.map_err(|err| -> Box<dyn std::error::Error> { err })
}

// ── Helpers ──

/// Collect downloadable media URLs from the extracted response contents.
fn collect_media_urls(
  task: &Task,
  extracted: &Option<PollResponseExtractedContents>,
) -> Result<(Vec<String>, TaskMediaFileClass), Box<dyn std::error::Error>> {
  let extracted = match extracted {
    Some(e) => e,
    None => return Ok((vec![], TaskMediaFileClass::Image)),
  };

  // Images (batch)
  if let Some(images) = &extracted.images {
    let urls: Vec<String> = images.iter()
      .filter_map(|img| img.url.clone())
      .collect();
    if !urls.is_empty() {
      return Ok((urls, TaskMediaFileClass::Image));
    }
  }

  // Single image (e.g. background removal)
  if let Some(image) = &extracted.image {
    if let Some(url) = &image.url {
      return Ok((vec![url.clone()], TaskMediaFileClass::Image));
    }
  }

  // Video
  if let Some(video) = &extracted.video {
    if let Some(url) = &video.url {
      return Ok((vec![url.clone()], TaskMediaFileClass::Video));
    }
  }

  // Audio
  if let Some(audio) = &extracted.audio {
    if let Some(url) = &audio.url {
      return Ok((vec![url.clone()], TaskMediaFileClass::Audio));
    }
  }

  // 3D model (GLB)
  if let Some(glb) = &extracted.model_glb {
    if let Some(url) = &glb.url {
      return Ok((vec![url.clone()], TaskMediaFileClass::Dimensional));
    }
  }

  Ok((vec![], TaskMediaFileClass::Image))
}

async fn download_file(
  url: &str,
  app_data_root: &AppDataRoot,
  index: usize,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
  let response = reqwest::get(url).await?;
  let bytes = response.bytes().await?;

  let extension = url_utils::download_extension::extract_download_extension_from_url::extract_download_extension_from_url_str(url)
    .map(|ext| ext.as_extension_without_period())
    .unwrap_or("bin");

  let tempdir = app_data_root.temp_dir().path();
  let filename = format!("fal_{}_{}.{}", generate_random_uuid(), index, extension);
  let download_path = tempdir.join(filename);

  let mut file = File::create(&download_path)?;
  file.write_all(&bytes)?;

  Ok(download_path)
}

const MAX_UPLOAD_RETRIES: u32 = 5;
const INITIAL_RETRY_DELAY_SECS: u64 = 10;
