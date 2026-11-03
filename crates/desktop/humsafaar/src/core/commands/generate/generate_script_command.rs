use serde_derive::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::State;

use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use crate::core::providers::credentials::payload::provider_credential_payload::ProviderCredentialPayload;
use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;

const OPENAI_URL: &str = "https://api.openai.com/v1";
const GEMINI_URL: &str = "https://generativelanguage.googleapis.com/v1beta/openai";
const GROQ_URL: &str = "https://api.groq.com/openai/v1";
const DEFAULT_MODEL: &str = "gpt-4o-mini";
const GROQ_DEFAULT_MODEL: &str = "openai/gpt-oss-20b";
const GEMINI_DEFAULT_MODEL: &str = "gemini-3.1-flash-lite";

#[derive(Deserialize)]
pub struct GenerateScriptRequest {
  pub prompt: String,
  pub system_prompt: Option<String>,
}

#[derive(Serialize)]
pub struct GenerateScriptResponse {
  pub text: String,
}

impl SerializeMarker for GenerateScriptResponse {}

/// Writes text with any OpenAI-compatible chat server: OpenAI itself by default, or a server of
/// your own (Ollama, llama.cpp, Colibri) by setting `HUMSAFAAR_LLM_URL`, e.g. `http://localhost:11434/v1`.
/// `HUMSAFAAR_LLM_MODEL` names the model. With no URL set, a saved Gemini key is used, then Groq, then OpenAI.
#[tauri::command]
pub async fn generate_script_command(
  request: GenerateScriptRequest,
  credentials: State<'_, ProviderCredentialLoadingCache>,
) -> ResponseOrErrorMessage<GenerateScriptResponse> {
  let url = std::env::var("HUMSAFAAR_LLM_URL").ok().filter(|v| !v.trim().is_empty());
  let model = std::env::var("HUMSAFAAR_LLM_MODEL").ok().filter(|v| !v.trim().is_empty());
  let saved = |key| match credentials.get_credentials(key) {
    Ok(Some(ProviderCredentialPayload::ApiKey(data))) if !data.as_str().trim().is_empty() => Some(data.as_str().trim().to_string()),
    _ => None,
  };
  let gemini_key = saved(ProviderCredentialKey::GeminiApiKey);
  let groq_key = saved(ProviderCredentialKey::GroqApiKey);
  let openai_key = saved(ProviderCredentialKey::OpenAiApiKey);

  // A server named by URL wins; otherwise a saved Gemini key, then Groq, then OpenAI. Each key goes only to its own host.
  let (base, default_model, key) = match url.as_deref() {
    Some(u) if u.contains("googleapis.com") => (u.to_string(), GEMINI_DEFAULT_MODEL, gemini_key),
    Some(u) if u.contains("groq.com") => (u.to_string(), GROQ_DEFAULT_MODEL, groq_key),
    Some(u) => (u.to_string(), DEFAULT_MODEL, openai_key),
    None if gemini_key.is_some() => (GEMINI_URL.to_string(), GEMINI_DEFAULT_MODEL, gemini_key),
    None if groq_key.is_some() => (GROQ_URL.to_string(), GROQ_DEFAULT_MODEL, groq_key),
    None => (OPENAI_URL.to_string(), DEFAULT_MODEL, openai_key),
  };

  // Our own server may need no key, but the hosted ones always do.
  if key.is_none() && url.as_deref().map_or(true, |u| u.contains("groq.com") || u.contains("googleapis.com")) {
    return Err("Add a Gemini, Groq or OpenAI API key in Settings, then API Keys, or set HUMSAFAAR_LLM_URL to a server of your own.".into());
  }

  let body = chat_body(model.as_deref().unwrap_or(default_model), &request);
  let endpoint = format!("{}/chat/completions", base.trim().trim_end_matches('/'));
  let mut call = reqwest::Client::new().post(endpoint).json(&body);
  if let Some(key) = key {
    call = call.bearer_auth(key);
  }

  let response = call.send().await.map_err(|err| format!("Could not reach the writing model: {}", err))?;
  let status = response.status();
  let reply: Value = response.json().await.map_err(|err| format!("The writing model sent something unreadable: {}", err))?;
  if !status.is_success() {
    return Err(format!("The writing model refused ({}): {}", status, reply["error"]["message"].as_str().unwrap_or("no reason given")).into());
  }

  let text = reply_text(&reply).ok_or("The writing model returned no text")?;
  Ok(GenerateScriptResponse { text }.into())
}

fn chat_body(model: &str, request: &GenerateScriptRequest) -> Value {
  let mut messages = Vec::new();
  if let Some(system) = request.system_prompt.as_deref().filter(|s| !s.trim().is_empty()) {
    messages.push(json!({ "role": "system", "content": system }));
  }
  messages.push(json!({ "role": "user", "content": request.prompt }));
  json!({ "model": model, "messages": messages })
}

fn reply_text(reply: &Value) -> Option<String> {
  reply["choices"][0]["message"]["content"].as_str().map(str::to_string).filter(|t| !t.trim().is_empty())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_blank_system_prompt_is_left_out() {
    let body = chat_body("m", &GenerateScriptRequest { prompt: "hi".into(), system_prompt: Some("  ".into()) });
    assert_eq!(body["messages"].as_array().unwrap().len(), 1);
    let body = chat_body("m", &GenerateScriptRequest { prompt: "hi".into(), system_prompt: Some("be brief".into()) });
    assert_eq!(body["messages"][0]["role"], "system");
  }

  #[test]
  fn the_reply_text_is_read_from_the_first_choice() {
    assert_eq!(reply_text(&json!({"choices":[{"message":{"content":"INT. ROOM"}}]})).as_deref(), Some("INT. ROOM"));
    assert_eq!(reply_text(&json!({"choices":[]})), None);
  }
}
