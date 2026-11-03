use enums::common::generation_provider::GenerationProvider;
use log::info;

use crate::core::commands::enqueue::generate_error::{GenerateError, MissingCredentialsReason};
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::commands::generate::generate_image::providers::humsafaar_router::handle_gemini::handle_gemini_image;
use crate::core::commands::generate::generate_image::providers::humsafaar_router::handle_grok::handle_grok_image;
use crate::core::commands::generate::generate_image::providers::humsafaar_router::handle_api_providers::handle_api_key_provider;
use crate::core::commands::generate::generate_image::tauri_generate_image_request::TauriGenerateImageRequest;
use crate::core::local_media::local_media_store::LocalMediaStore;
use crate::core::providers::credentials::payload::provider_credential_payload::ProviderCredentialPayload;
use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;

/// Dispatch an image generation request to a cloud provider with the user's own API key.
pub async fn handle_router(
  request: &TauriGenerateImageRequest,
  provider: GenerationProvider,
  credential_cache: &ProviderCredentialLoadingCache,
  local_media: &LocalMediaStore,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  if provider == GenerationProvider::Grok {
    return handle_grok_image(request, credential_cache, local_media).await;
  }

  if provider == GenerationProvider::Gemini {
    return handle_gemini_image(request, credential_cache, local_media).await;
  }

  let credential_key = map_provider_to_credential_key(provider)?;

  info!("handle_router: provider={:?}, credential_key={:?}", provider, credential_key);

  let payload = credential_cache.get_credentials(credential_key)
    .map_err(|err| {
      GenerateError::AnyhowError(anyhow::anyhow!("Failed to load credentials: {:?}", err))
    })?
    .ok_or_else(|| map_provider_to_missing_credentials_error(provider))?;

  match payload {
    ProviderCredentialPayload::ApiKey(api_key_data) => {
      handle_api_key_provider(request, provider, api_key_data.as_str(), local_media).await
    }
    ProviderCredentialPayload::WebLogin(_) => Err(GenerateError::NotYetImplemented(
      format!("Provider {:?} does not use a web login here", provider),
    )),
  }
}

// ── Helpers ──

fn map_provider_to_credential_key(
  provider: GenerationProvider,
) -> Result<ProviderCredentialKey, GenerateError> {
  match provider {
    GenerationProvider::Fal => Ok(ProviderCredentialKey::FalApiKey),
    _ => Err(GenerateError::NotYetImplemented(
      format!("Provider {:?} does not have a mapped credential key yet", provider),
    )),
  }
}

fn map_provider_to_missing_credentials_error(provider: GenerationProvider) -> GenerateError {
  match provider {
    GenerationProvider::Fal => GenerateError::MissingCredentials(MissingCredentialsReason::NeedsFalApiKey),
    _ => GenerateError::NoProviderAvailable,
  }
}
