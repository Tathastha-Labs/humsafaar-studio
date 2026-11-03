use enums::common::generation_provider::GenerationProvider;
use enums::tauri::ux::tauri_command_caller::TauriCommandCaller;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// What a generate page sends: a few desktop fields, and every other field passed through as the
/// page wrote it, so model ids and options can change without touching this type.
#[derive(Clone, Deserialize, Serialize)]
pub struct OmniRequest {
  pub provider: Option<GenerationProvider>,
  pub frontend_caller: Option<TauriCommandCaller>,
  pub frontend_subscriber_id: Option<String>,
  pub frontend_subscriber_payload: Option<String>,
  #[serde(flatten)]
  pub fields: Map<String, Value>,
}

impl OmniRequest {
  pub fn model(&self) -> Option<&str> {
    self.fields.get("model").and_then(Value::as_str)
  }
}
