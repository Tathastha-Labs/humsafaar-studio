use crate::core::cloud_models::groq::{load_key, speak, VOICES};
use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::local_media::local_media_store::LocalMediaStore;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;
use crate::core::commands::voiceover::voiceover_generate_command::VoiceoverGenerateSuccessResponse;
use serde_derive::Deserialize;
use tauri::State;
use uuid_utils::uuid::generate_random_uuid;

#[derive(Deserialize)]
pub struct GroqVoiceoverRequest {
  pub text: String,
  /// One of Groq's English voices; Hannah when left out.
  pub voice: Option<String>,
}

/// Speak the script with the user's Groq key and save it in the local media store.
#[tauri::command]
pub async fn groq_voiceover_generate_command(
  request: GroqVoiceoverRequest,
  credentials: State<'_, ProviderCredentialLoadingCache>,
  store: State<'_, LocalMediaStore>,
) -> ResponseOrErrorMessage<VoiceoverGenerateSuccessResponse> {
  let key = load_key(&credentials)?;
  let text = request.text.trim();
  if text.is_empty() {
    return Err("Write something to say first.".into());
  }
  let voice = request.voice.as_deref().unwrap_or("hannah");
  if !VOICES.contains(&voice) {
    return Err(format!("Groq has no voice called {}. Pick one of: {}.", voice, VOICES.join(", ")).into());
  }

  let wav = speak(&key, text, voice).await?;
  let scratch = std::env::temp_dir().join(format!("groq-voiceover-{}.wav", generate_random_uuid()));
  std::fs::write(&scratch, &wav).map_err(|err| format!("Could not save the voiceover: {}", err))?;
  let saved = store.put_file(&scratch);
  let _ = std::fs::remove_file(&scratch);
  let token = saved.map_err(|err| format!("Could not save the voiceover: {}", err))?;
  let path = store.path_for_token(&token).ok_or("The saved voiceover went missing.")?;

  // A WAV header is 44 bytes, then 16-bit samples; Groq's speech comes back at 24 kHz mono.
  let seconds = wav.len().saturating_sub(44) as f32 / (24_000.0 * 2.0);
  Ok(VoiceoverGenerateSuccessResponse {
    media_token: token.to_string(),
    url: LocalMediaStore::asset_url_for_path(&path),
    seconds,
  }.into())
}
