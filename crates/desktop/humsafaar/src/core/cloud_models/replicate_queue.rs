use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_type::TaskType;
use serde_json::Value;

use crate::core::cloud_models::local_picture::LocalPicture;
use crate::core::cloud_models::replicate_api::{create_prediction, upload_file, MAX_INLINE_BYTES};
use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::events::generation_events::common::GenerationModel;
use crate::core::providers::credentials::payload::provider_credential_payload::ProviderCredentialPayload;
use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;

/// A key from Settings, or the error that sends the user there.
pub fn load_api_key(
  cache: &ProviderCredentialLoadingCache,
  key: ProviderCredentialKey,
  provider_name: &str,
) -> Result<String, GenerateError> {
  match cache.get_credentials(key) {
    Ok(Some(ProviderCredentialPayload::ApiKey(data))) if !data.as_str().trim().is_empty() => {
      Ok(data.as_str().trim().to_string())
    }
    _ => Err(GenerateError::AnyhowError(anyhow::anyhow!(
      "Add your {} API key in Settings, then API Keys, to use {}.", provider_name, provider_name
    ))),
  }
}

/// The picture as something a Replicate model can read: inline when small, otherwise uploaded.
/// If the upload fails the picture is sent inline anyway, which Replicate still accepts.
pub async fn picture_for_replicate(api_key: &str, picture: &LocalPicture) -> String {
  if picture.bytes.len() <= MAX_INLINE_BYTES {
    return picture.data_uri();
  }
  match upload_file(api_key, picture.bytes.clone(), &picture.file_name, picture.mime).await {
    Ok(url) => url,
    Err(_) => picture.data_uri(),
  }
}

/// Start one prediction on Replicate and describe it so the shared poller can finish it.
pub async fn enqueue_on_replicate(
  model: &str,
  input: Value,
  api_key: &str,
  task_type: TaskType,
  model_id: &str,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let created = create_prediction(api_key, model, input).await
    .map_err(|message| GenerateError::AnyhowError(anyhow::anyhow!(message)))?;

  Ok(TaskEnqueueSuccess {
    task_type,
    model: Some(GenerationModel::Unknown(model_id.to_string())),
    provider: GenerationProvider::Replicate,
    provider_job_id: Some(created.id),
    maybe_queue_status_url: Some(created.get_url),
    maybe_queue_response_url: None,
    maybe_prompt_token: None,
  })
}
