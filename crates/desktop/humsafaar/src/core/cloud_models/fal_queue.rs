use fal_client::requests::core_api::fal_request::FalRequest;
use serde_json::Value;

use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::events::generation_events::common::GenerationModel;
use crate::core::providers::credentials::payload::provider_credential_payload::ProviderCredentialPayload;
use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;
use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_type::TaskType;

/// The Fal key from Settings, or the error that sends the user there.
pub fn load_fal_api_key(cache: &ProviderCredentialLoadingCache) -> Result<String, GenerateError> {
  match cache.get_credentials(ProviderCredentialKey::FalApiKey) {
    Ok(Some(ProviderCredentialPayload::ApiKey(data))) if !data.as_str().trim().is_empty() => {
      Ok(data.as_str().trim().to_string())
    }
    _ => Err(GenerateError::needs_fal_api_key()),
  }
}

/// Put one job on Fal's queue and describe it so the shared poller can finish it.
/// Fal's own polling and result handling already exist; this only starts the job.
pub async fn enqueue_on_fal(
  endpoint: &str,
  input: Value,
  api_key: &str,
  task_type: TaskType,
  model_id: &str,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let queued = FalRequest::<Value, Value>::new(endpoint, input)
    .with_api_key(api_key)
    .queue_request()
    .await
    .map_err(|err| GenerateError::AnyhowError(anyhow::anyhow!("{}", friendly_fal_error(&err.to_string()))))?;

  Ok(TaskEnqueueSuccess {
    task_type,
    model: Some(GenerationModel::Unknown(model_id.to_string())),
    provider: GenerationProvider::Fal,
    provider_job_id: Some(queued.request_id),
    maybe_queue_status_url: Some(queued.status_url),
    maybe_queue_response_url: Some(queued.response_url),
    maybe_prompt_token: None,
  })
}

/// Fal's errors are raw response bodies; the common ones deserve a plain sentence.
pub fn friendly_fal_error(raw: &str) -> String {
  let lower = raw.to_lowercase();
  if lower.contains("unauthorized") || lower.contains("invalid key") || lower.contains("authentication") {
    "Fal did not accept your API key. Check it in Settings, then API Keys.".to_string()
  } else if lower.contains("exhausted balance") || lower.contains("insufficient") || lower.contains("balance") {
    "Your Fal account is out of credit. Add funds on fal.ai and try again.".to_string()
  } else if lower.contains("locked") {
    "Your Fal account is locked. Check your account on fal.ai.".to_string()
  } else {
    format!("Fal could not start this job: {}", raw.chars().take(300).collect::<String>())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_rejected_key_points_to_settings() {
    assert!(friendly_fal_error(r#"{"detail":"Unauthorized"}"#).contains("API Keys"));
  }

  #[test]
  fn an_empty_balance_says_so() {
    assert!(friendly_fal_error("User is locked. Reason: Exhausted balance.").contains("out of credit"));
  }

  #[test]
  fn other_errors_are_passed_on_trimmed() {
    let message = friendly_fal_error(&"x".repeat(1000));
    assert!(message.starts_with("Fal could not start"));
    assert!(message.len() < 400);
  }
}
