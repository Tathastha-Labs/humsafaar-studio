use crate::core::cloud_models::replicate_api::{get_prediction, PredictionState};
use crate::core::cloud_models::gmi_video::check_gmi_video;
use crate::core::cloud_models::grok_video::check_grok_video;
use crate::core::cloud_models::world_labs::check_operation;
use crate::core::commands::generate::generate_image::providers::humsafaar_router::{LOCAL_FILE_SCHEME, RESULT_URL_SEPARATOR};
use crate::core::providers::credentials::payload::provider_credential_payload::ProviderCredentialPayload;
use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;
use crate::core::state::app_env_configs::app_env_configs::AppEnvConfigs;
use crate::core::state::data_dir::app_data_root::AppDataRoot;
use crate::core::state::data_dir::trait_data_subdir::DataSubdir;
use crate::core::state::task_database::TaskDatabase;
use crate::core::threads::third_party_task_polling_thread::handlers::complete_task_locally::complete_task_locally;
use crate::core::threads::third_party_task_polling_thread::handlers::fal::handle_fal_failure::handle_fal_failure;
use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_media_file_class::TaskMediaFileClass;
use enums::tauri::tasks::task_type::TaskType;
use log::{error, info, warn};
use sqlite_tasks::queries::task::Task;
use std::path::PathBuf;
use tauri::AppHandle;
use uuid_utils::uuid::generate_random_uuid;

type BoxedError = Box<dyn std::error::Error + Send + Sync>;

/// Check each pending job on a provider that is asked about over plain HTTPS; finish the ones that
/// are done. The job's status URL was saved when it was started.
pub async fn poll_rest_provider_tasks(
  provider: GenerationProvider,
  app_handle: &AppHandle,
  app_env_configs: &AppEnvConfigs,
  app_data_root: &AppDataRoot,
  task_database: &TaskDatabase,
  credential_cache: &ProviderCredentialLoadingCache,
  tasks: &[&Task],
) {
  let Some(api_key) = load_key(provider, credential_cache) else {
    warn!("[RestPolling] No {:?} API key configured, skipping poll", provider);
    return;
  };

  for task in tasks {
    // Replicate gives an address to ask; World Labs is asked by the job's own id.
    let locator = match provider {
      GenerationProvider::WorldLabs | GenerationProvider::Grok | GenerationProvider::GmiCloud | GenerationProvider::Gemini => task.provider_job_id.as_deref(),
      _ => task.queue_status_url.as_deref(),
    };
    let Some(status_url) = locator else {
      handle_fal_failure(app_handle, task_database, task, "This job has no status address to check").await;
      continue;
    };

    match check(provider, &api_key, status_url, task).await {
      Ok(PredictionState::Running) => {}
      Ok(PredictionState::Failed(message)) => {
        handle_fal_failure(app_handle, task_database, task, &message).await;
      }
      Ok(PredictionState::Done(urls)) => {
        info!("[RestPolling] Task {} finished with {} file(s)", task.id.as_str(), urls.len());
        let result = download_and_complete(
          provider, &api_key, app_handle, app_env_configs, app_data_root, task_database, task, &urls,
        ).await;
        if let Err(err) = result {
          error!("[RestPolling] Could not complete task {}: {:?}", task.id.as_str(), err);
          handle_fal_failure(app_handle, task_database, task, "Could not save the result").await;
        }
      }
      // A network blip is not the job failing; ask again next round.
      Err(message) => warn!("[RestPolling] Could not check task {}: {}", task.id.as_str(), message),
    }
  }
}

async fn check(provider: GenerationProvider, api_key: &str, status_url: &str, task: &Task) -> Result<PredictionState, String> {
  // xAI and Gemini pictures are ready when the job starts; their links were saved on the task.
  if matches!(provider, GenerationProvider::Grok | GenerationProvider::Gemini) && task.task_type == TaskType::ImageGeneration {
    let links = task.queue_response_url.as_deref().unwrap_or_default();
    return Ok(PredictionState::Done(links.split(RESULT_URL_SEPARATOR).filter(|l| !l.is_empty()).map(str::to_string).collect()));
  }
  match provider {
    GenerationProvider::Replicate => get_prediction(api_key, status_url).await,
    GenerationProvider::WorldLabs => check_operation(api_key, status_url).await,
    GenerationProvider::Grok => check_grok_video(api_key, status_url).await,
    GenerationProvider::GmiCloud => check_gmi_video(api_key, status_url).await,
    other => Err(format!("{:?} is not polled over HTTPS", other)),
  }
}

async fn download_and_complete(
  provider: GenerationProvider,
  api_key: &str,
  app_handle: &AppHandle,
  app_env_configs: &AppEnvConfigs,
  app_data_root: &AppDataRoot,
  task_database: &TaskDatabase,
  task: &Task,
  urls: &[String],
) -> Result<(), BoxedError> {
  let media_class = media_class_for(task.task_type);
  let urls = wanted_files(task.task_type, urls);

  let mut files: Vec<PathBuf> = Vec::with_capacity(urls.len());
  for url in &urls {
    let bytes = download(provider, api_key, url).await?;
    let path = app_data_root.temp_dir().path().join(format!("cloud_{}.{}", generate_random_uuid(), extension_of(url, task.task_type)));
    std::fs::write(&path, bytes)?;
    files.push(path);
  }

  complete_task_locally(app_handle, app_env_configs, app_data_root, task_database, task, files, media_class).await
}

async fn download(provider: GenerationProvider, api_key: &str, url: &str) -> Result<Vec<u8>, BoxedError> {
  // A picture already written to disk, as Gemini's are.
  if let Some(path) = url.strip_prefix(LOCAL_FILE_SCHEME) {
    let bytes = std::fs::read(path)?;
    let _ = std::fs::remove_file(path);
    return Ok(bytes);
  }
  let mut request = reqwest::Client::new().get(url);
  // Replicate says its output links may need the key; it is only ever sent to Replicate's own hosts.
  if provider == GenerationProvider::Replicate && is_replicate_host(url) {
    request = request.bearer_auth(api_key);
  }
  let response = request.send().await?.error_for_status()?;
  Ok(response.bytes().await?.to_vec())
}

fn load_key(provider: GenerationProvider, cache: &ProviderCredentialLoadingCache) -> Option<String> {
  let key = match provider {
    GenerationProvider::Replicate => ProviderCredentialKey::ReplicateApiKey,
    GenerationProvider::WorldLabs => ProviderCredentialKey::WorldLabsApiKey,
    GenerationProvider::Grok => ProviderCredentialKey::GrokApiKey,
    GenerationProvider::GmiCloud => ProviderCredentialKey::GmiCloudApiKey,
    GenerationProvider::Gemini => ProviderCredentialKey::GeminiApiKey,
    _ => return None,
  };
  match cache.get_credentials(key) {
    Ok(Some(ProviderCredentialPayload::ApiKey(data))) if !data.as_str().trim().is_empty() => Some(data.as_str().trim().to_string()),
    _ => None,
  }
}

fn media_class_for(task_type: TaskType) -> TaskMediaFileClass {
  match task_type {
    TaskType::VideoGeneration => TaskMediaFileClass::Video,
    TaskType::AudioGeneration => TaskMediaFileClass::Audio,
    TaskType::ObjectGeneration | TaskType::GaussianGeneration => TaskMediaFileClass::Dimensional,
    _ => TaskMediaFileClass::Image,
  }
}

/// A 3D job can return a preview video and other extras beside the model; keep only the model.
fn wanted_files(task_type: TaskType, urls: &[String]) -> Vec<String> {
  if task_type != TaskType::ObjectGeneration {
    return urls.to_vec();
  }
  let models: Vec<String> = urls.iter().filter(|url| is_model_file(url)).cloned().collect();
  if models.is_empty() { urls.iter().take(1).cloned().collect() } else { models.into_iter().take(1).collect() }
}

fn is_model_file(url: &str) -> bool {
  matches!(extension_text(url).as_deref(), Some("glb") | Some("gltf"))
}

fn extension_of(url: &str, task_type: TaskType) -> String {
  extension_text(url).unwrap_or_else(|| match task_type {
    TaskType::VideoGeneration => "mp4",
    TaskType::AudioGeneration => "mp3",
    TaskType::ObjectGeneration => "glb",
    TaskType::GaussianGeneration => "spz",
    _ => "png",
  }.to_string())
}

/// The file extension in the path of a URL, ignoring any query string.
fn extension_text(url: &str) -> Option<String> {
  let path = url.split(['?', '#']).next()?;
  let name = path.rsplit('/').next()?;
  let (_, extension) = name.rsplit_once('.')?;
  (!extension.is_empty() && extension.len() <= 5 && extension.chars().all(|c| c.is_ascii_alphanumeric()))
    .then(|| extension.to_ascii_lowercase())
}

fn is_replicate_host(url: &str) -> bool {
  let host = url.split("://").nth(1).and_then(|rest| rest.split('/').next()).unwrap_or("");
  host == "replicate.com" || host.ends_with(".replicate.com") || host == "replicate.delivery" || host.ends_with(".replicate.delivery")
}

#[cfg(test)]
mod tests {
  use super::*;

  mod files {
    use super::*;

    #[test]
    fn the_extension_comes_from_the_path_not_the_query() {
      assert_eq!(extension_text("https://x/out/video.MP4?sig=abc.def").as_deref(), Some("mp4"));
      assert_eq!(extension_text("https://x/out/model.glb").as_deref(), Some("glb"));
    }

    #[test]
    fn no_extension_falls_back_by_kind() {
      assert_eq!(extension_of("https://x/out/abc", TaskType::VideoGeneration), "mp4");
      assert_eq!(extension_of("https://x/out/abc", TaskType::AudioGeneration), "mp3");
      assert_eq!(extension_of("https://x/out/abc", TaskType::ObjectGeneration), "glb");
    }

    #[test]
    fn a_3d_job_keeps_only_the_model() {
      let urls = vec!["https://x/preview.mp4".to_string(), "https://x/m.glb".to_string(), "https://x/other.glb".to_string()];
      assert_eq!(wanted_files(TaskType::ObjectGeneration, &urls), vec!["https://x/m.glb".to_string()]);
    }

    #[test]
    fn a_3d_job_with_no_glb_keeps_the_first_file() {
      let urls = vec!["https://x/a.obj".to_string(), "https://x/b.obj".to_string()];
      assert_eq!(wanted_files(TaskType::ObjectGeneration, &urls), vec!["https://x/a.obj".to_string()]);
    }

    #[test]
    fn other_jobs_keep_everything() {
      let urls = vec!["https://x/a.png".to_string(), "https://x/b.png".to_string()];
      assert_eq!(wanted_files(TaskType::ImageGeneration, &urls), urls);
    }
  }

  mod hosts {
    use super::*;

    #[test]
    fn only_replicate_hosts_get_the_key() {
      assert!(is_replicate_host("https://replicate.delivery/pbxt/abc/out.mp4"));
      assert!(is_replicate_host("https://api.replicate.com/v1/files/abc"));
      assert!(!is_replicate_host("https://evil.example/replicate.com/out.mp4"));
      assert!(!is_replicate_host("https://notreplicate.com/out.mp4"));
    }
  }
}
