use crate::core::cloud_models::groq::{load_key, transcribe};
use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;
use serde_derive::{Deserialize, Serialize};
use tauri::State;

#[derive(Deserialize)]
pub struct TranscribeAudioRequest {
  /// A recording the user picked on this computer.
  pub file_path: String,
}

#[derive(Serialize)]
pub struct TranscribeAudioResponse {
  pub text: String,
}

impl SerializeMarker for TranscribeAudioResponse {}

/// Turn a recording into text with the user's Groq key.
#[tauri::command]
pub async fn transcribe_audio_command(
  request: TranscribeAudioRequest,
  credentials: State<'_, ProviderCredentialLoadingCache>,
) -> ResponseOrErrorMessage<TranscribeAudioResponse> {
  let key = load_key(&credentials)?;
  let path = std::path::Path::new(&request.file_path);
  let audio = std::fs::read(path).map_err(|err| format!("Could not read that file: {}", err))?;
  let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("audio.wav").to_string();
  let text = transcribe(&key, name, audio).await?;
  Ok(TranscribeAudioResponse { text }.into())
}
