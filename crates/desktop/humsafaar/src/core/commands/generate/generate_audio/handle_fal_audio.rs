use enums::tauri::tasks::task_type::TaskType;
use log::info;
use serde_json::{Map, Value};

use crate::core::cloud_models::fal_audio::{fal_audio_job, FAL_ACE_STEP_MODEL_ID};
use crate::core::cloud_models::fal_queue::{enqueue_on_fal, load_fal_api_key};
use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::local_models::seed::random_seed;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;

/// Music on Fal with the user's own key. `fields` are the audio page's request fields.
pub async fn handle_fal_audio(
  fields: &Map<String, Value>,
  credentials: &ProviderCredentialLoadingCache,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  if fields.get("model").and_then(Value::as_str) != Some(FAL_ACE_STEP_MODEL_ID) {
    return Err(GenerateError::NotYetImplemented("That audio model is not available on Fal yet".to_string()));
  }

  let text = |key: &str| fields.get(key).and_then(Value::as_str).unwrap_or("");
  let instrumental = fields.get("is_instrumental").and_then(Value::as_bool).unwrap_or(false);
  let job = fal_audio_job(text("prompt"), text("style_prompt"), instrumental, random_seed())
    .map_err(|message| GenerateError::AnyhowError(anyhow::anyhow!(message)))?;

  let api_key = load_fal_api_key(credentials)?;
  info!("Queueing Fal music: {}", job.endpoint);
  enqueue_on_fal(job.endpoint, job.input, &api_key, TaskType::AudioGeneration, FAL_ACE_STEP_MODEL_ID).await
}
