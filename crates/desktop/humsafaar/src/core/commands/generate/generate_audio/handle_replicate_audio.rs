use enums::tauri::tasks::task_type::TaskType;
use log::info;
use serde_json::{Map, Value};

use crate::core::cloud_models::replicate_queue::{enqueue_on_replicate, load_api_key};
use crate::core::cloud_models::replicate_recipes::{replicate_music_job, MINIMAX_MUSIC_MODEL_ID};
use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;

/// Music on Replicate with the user's own key. `fields` are the audio page's request fields.
pub async fn handle_replicate_audio(
  fields: &Map<String, Value>,
  credentials: &ProviderCredentialLoadingCache,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  if fields.get("model").and_then(Value::as_str) != Some(MINIMAX_MUSIC_MODEL_ID) {
    return Err(GenerateError::NotYetImplemented("That audio model is not available on Replicate yet".to_string()));
  }

  let text = |key: &str| fields.get(key).and_then(Value::as_str).unwrap_or("");
  let job = replicate_music_job(text("prompt"), text("style_prompt"))
    .map_err(|message| GenerateError::AnyhowError(anyhow::anyhow!(message)))?;

  let api_key = load_api_key(credentials, ProviderCredentialKey::ReplicateApiKey, "Replicate")?;
  info!("Queueing Replicate music: {}", job.model);
  enqueue_on_replicate(job.model, job.input, &api_key, TaskType::AudioGeneration, MINIMAX_MUSIC_MODEL_ID).await
}
