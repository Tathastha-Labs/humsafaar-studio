use serde_derive::{Deserialize, Serialize};
use crate::core::providers::credentials::provider_credential_type::ProviderCredentialType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCredentialKey {

  // ========== API KEYS==========

  FalApiKey,
  ReplicateApiKey,
  OpenAiApiKey,
  AnthropicApiKey,
  HuggingFaceApiKey,
  GeminiApiKey,
  GmiCloudApiKey,
  GrokApiKey,
  GroqApiKey,
  WorldLabsApiKey,

  // ========== WEB LOGINS ==========

  GrokWebLogin,
  HiggsfieldWebLogin,
  MidjourneyLogin,
  RunwayWebLogin,
}


impl ProviderCredentialKey {
  pub fn get_type(&self) -> ProviderCredentialType {
    match self {
      // Api keys
      Self::FalApiKey => ProviderCredentialType::ApiKey,
      Self::ReplicateApiKey => ProviderCredentialType::ApiKey,
      Self::OpenAiApiKey => ProviderCredentialType::ApiKey,
      Self::AnthropicApiKey => ProviderCredentialType::ApiKey,
      Self::HuggingFaceApiKey => ProviderCredentialType::ApiKey,
      Self::GeminiApiKey => ProviderCredentialType::ApiKey,
      Self::GmiCloudApiKey => ProviderCredentialType::ApiKey,
      Self::GrokApiKey => ProviderCredentialType::ApiKey,
      Self::GroqApiKey => ProviderCredentialType::ApiKey,
      Self::WorldLabsApiKey => ProviderCredentialType::ApiKey,
      // Web logins
      Self::GrokWebLogin => ProviderCredentialType::WebLogin,
      Self::HiggsfieldWebLogin => ProviderCredentialType::WebLogin,
      Self::MidjourneyLogin => ProviderCredentialType::WebLogin,
      Self::RunwayWebLogin => ProviderCredentialType::WebLogin,
    }
  }

  pub fn get_filename(&self) -> &'static str {
    match self {
      // Api keys
      ProviderCredentialKey::FalApiKey => "fal.api_key.txt",
      ProviderCredentialKey::ReplicateApiKey => "replicate.api_key.txt",
      ProviderCredentialKey::OpenAiApiKey => "openai.api_key.txt",
      ProviderCredentialKey::AnthropicApiKey => "anthropic.api_key.txt",
      ProviderCredentialKey::HuggingFaceApiKey => "huggingface.api_key.txt",
      ProviderCredentialKey::GeminiApiKey => "gemini.api_key.txt",
      ProviderCredentialKey::GmiCloudApiKey => "gmi_cloud.api_key.txt",
      ProviderCredentialKey::GrokApiKey => "grok.api_key.txt",
      ProviderCredentialKey::GroqApiKey => "groq.api_key.txt",
      ProviderCredentialKey::WorldLabsApiKey => "world_labs.api_key.txt",
      // Web logins
      ProviderCredentialKey::GrokWebLogin => "grok.web_login.toml",
      ProviderCredentialKey::HiggsfieldWebLogin => "higgsfield.web_login.toml",
      ProviderCredentialKey::MidjourneyLogin => "midjourney.web_login.toml",
      ProviderCredentialKey::RunwayWebLogin => "runway.web_login.toml",
    }
  }
}


