use reqwest::multipart::{Form, Part};
use serde_json::{json, Value};

use crate::core::providers::credentials::payload::provider_credential_payload::ProviderCredentialPayload;
use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;

const BASE_URL: &str = "https://api.groq.com/openai/v1";

/// Groq refuses uploads over 25 MB, so say so before sending a long recording.
pub const MAX_UPLOAD_BYTES: usize = 25 * 1024 * 1024;

pub const SPEECH_MODEL: &str = "canopylabs/orpheus-v1-english";
pub const TRANSCRIBE_MODEL: &str = "whisper-large-v3-turbo";

/// Voices of the English speech model, as listed in Groq's docs.
pub const VOICES: [&str; 6] = ["autumn", "diana", "hannah", "austin", "daniel", "troy"];

pub fn load_key(cache: &ProviderCredentialLoadingCache) -> Result<String, String> {
  match cache.get_credentials(ProviderCredentialKey::GroqApiKey) {
    Ok(Some(ProviderCredentialPayload::ApiKey(data))) if !data.as_str().trim().is_empty() => Ok(data.as_str().trim().to_string()),
    _ => Err("Add your Groq API key in Settings, then API Keys, to use Groq.".to_string()),
  }
}

/// Turn a recording into text.
pub async fn transcribe(api_key: &str, file_name: String, audio: Vec<u8>) -> Result<String, String> {
  if audio.len() > MAX_UPLOAD_BYTES {
    return Err("That recording is over 25 MB, which Groq will not take. Try a shorter clip.".to_string());
  }
  let form = Form::new()
    .part("file", Part::bytes(audio).file_name(file_name))
    .text("model", TRANSCRIBE_MODEL)
    .text("response_format", "json");
  let response = reqwest::Client::new()
    .post(format!("{}/audio/transcriptions", BASE_URL))
    .bearer_auth(api_key)
    .multipart(form)
    .send().await
    .map_err(|err| format!("Could not reach Groq: {}", err))?;
  let status = response.status();
  let reply: Value = response.json().await.map_err(|err| format!("Groq sent something unreadable: {}", err))?;
  if !status.is_success() {
    return Err(error_message(&reply, status.as_u16()));
  }
  reply["text"].as_str().map(|t| t.trim().to_string()).ok_or_else(|| "Groq returned no text".to_string())
}

/// Speak text. Returns a WAV file.
pub async fn speak(api_key: &str, text: &str, voice: &str) -> Result<Vec<u8>, String> {
  let response = reqwest::Client::new()
    .post(format!("{}/audio/speech", BASE_URL))
    .bearer_auth(api_key)
    .json(&json!({ "model": SPEECH_MODEL, "input": text, "voice": voice, "response_format": "wav" }))
    .send().await
    .map_err(|err| format!("Could not reach Groq: {}", err))?;
  let status = response.status();
  if !status.is_success() {
    let reply: Value = response.json().await.unwrap_or(Value::Null);
    return Err(error_message(&reply, status.as_u16()));
  }
  response.bytes().await.map(|b| b.to_vec()).map_err(|err| format!("Could not read Groq's audio: {}", err))
}

// The speech model asks the account owner to accept terms once; that message says exactly where.
fn error_message(reply: &Value, status: u16) -> String {
  format!("Groq refused ({}): {}", status, reply["error"]["message"].as_str().unwrap_or("no reason given"))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_refusal_carries_groqs_own_words() {
    let reply = json!({"error":{"message":"accept the terms"}});
    assert_eq!(error_message(&reply, 400), "Groq refused (400): accept the terms");
    assert_eq!(error_message(&Value::Null, 500), "Groq refused (500): no reason given");
  }

  #[tokio::test]
  async fn a_huge_recording_is_refused_before_upload() {
    let err = transcribe("k", "a.wav".into(), vec![0; MAX_UPLOAD_BYTES + 1]).await.unwrap_err();
    assert!(err.contains("25 MB"));
  }

  // Costs a fraction of a cent: GROQ_API_KEY=... GROQ_TEST_WAV=/path/to/clip.wav cargo test -p humsafaar live_transcribe -- --ignored
  #[tokio::test]
  #[ignore]
  async fn live_transcribe() {
    let key = std::env::var("GROQ_API_KEY").unwrap();
    let audio = std::fs::read(std::env::var("GROQ_TEST_WAV").unwrap()).unwrap();
    let text = transcribe(&key, "clip.wav".into(), audio).await.unwrap();
    assert!(text.to_lowercase().contains("fisherman"), "{text}");
  }
}
