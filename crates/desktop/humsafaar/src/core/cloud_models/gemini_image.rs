//! Pictures from Google's Gemini image models (the "Nano Banana" family) with the user's own key.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::{json, Value};

const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

/// What one Gemini picture is made from.
pub struct GeminiImageInputs<'a> {
  pub prompt: &'a str,
  /// Source pictures as `data:` URIs, for an edit or a reference.
  pub sources: &'a [String],
  /// "16:9", "1:1" and so on; Gemini picks when absent.
  pub aspect_ratio: Option<&'a str>,
}

/// One picture: the bytes and the type Gemini said they are.
pub struct GeminiPicture {
  pub bytes: Vec<u8>,
  pub mime: String,
}

pub async fn generate_picture(api_key: &str, model: &str, inputs: &GeminiImageInputs<'_>) -> Result<GeminiPicture, String> {
  let response = reqwest::Client::new()
    .post(format!("{}/{}:generateContent", BASE_URL, model))
    .header("x-goog-api-key", api_key)
    .json(&request_body(inputs))
    .send()
    .await
    .map_err(|err| format!("Could not reach Gemini: {}", err))?;
  let status = response.status();
  let reply: Value = response.json().await.map_err(|err| format!("Gemini sent something unreadable: {}", err))?;
  if !status.is_success() {
    return Err(friendly_error(status.as_u16(), reply["error"]["message"].as_str().unwrap_or("no reason given")));
  }
  picture_from_reply(&reply)
}

fn request_body(inputs: &GeminiImageInputs) -> Value {
  let mut parts = vec![json!({ "text": inputs.prompt })];
  for source in inputs.sources {
    if let Some((mime, data)) = split_data_uri(source) {
      parts.push(json!({ "inlineData": { "mimeType": mime, "data": data } }));
    }
  }
  let mut config = json!({ "responseModalities": ["IMAGE"] });
  if let Some(ratio) = inputs.aspect_ratio {
    config["imageConfig"] = json!({ "aspectRatio": ratio });
  }
  json!({ "contents": [{ "parts": parts }], "generationConfig": config })
}

fn picture_from_reply(reply: &Value) -> Result<GeminiPicture, String> {
  let parts = reply["candidates"][0]["content"]["parts"].as_array();
  let inline = parts.and_then(|parts| parts.iter().find_map(|part| part.get("inlineData")));
  match inline {
    Some(data) => {
      let bytes = STANDARD.decode(data["data"].as_str().unwrap_or_default()).map_err(|err| format!("Gemini's picture was unreadable: {}", err))?;
      Ok(GeminiPicture { bytes, mime: data["mimeType"].as_str().unwrap_or("image/png").to_string() })
    }
    // A refusal comes back as words instead of a picture.
    None => {
      let said = parts.and_then(|parts| parts.iter().find_map(|part| part["text"].as_str())).unwrap_or("no picture came back");
      Err(format!("Gemini did not make a picture: {}", said.chars().take(200).collect::<String>()))
    }
  }
}

fn friendly_error(status: u16, message: &str) -> String {
  match status {
    401 | 403 => "Gemini did not accept your API key. Check it in Settings, then API Keys.".to_string(),
    402 | 429 => "Your Gemini account is out of credit or over its limit. Check billing at ai.studio/projects.".to_string(),
    _ => format!("Gemini refused ({}): {}", status, message.chars().take(200).collect::<String>()),
  }
}

fn split_data_uri(uri: &str) -> Option<(&str, &str)> {
  let rest = uri.strip_prefix("data:")?;
  let (meta, data) = rest.split_once(',')?;
  Some((meta.strip_suffix(";base64")?, data))
}

pub fn extension_for(mime: &str) -> &'static str {
  match mime {
    "image/jpeg" => "jpg",
    "image/webp" => "webp",
    _ => "png",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_prompt_and_source_pictures_go_in_one_message() {
    let sources = vec!["data:image/png;base64,QUJD".to_string()];
    let body = request_body(&GeminiImageInputs { prompt: "a fox", sources: &sources, aspect_ratio: Some("16:9") });
    let parts = body["contents"][0]["parts"].as_array().unwrap();
    assert_eq!(parts[0]["text"], "a fox");
    assert_eq!(parts[1]["inlineData"]["mimeType"], "image/png");
    assert_eq!(parts[1]["inlineData"]["data"], "QUJD");
    assert_eq!(body["generationConfig"]["imageConfig"]["aspectRatio"], "16:9");
  }

  #[test]
  fn the_picture_is_read_from_the_reply() {
    let reply = json!({"candidates":[{"content":{"parts":[{"text":"here"},{"inlineData":{"mimeType":"image/jpeg","data":"QUJD"}}]}}]});
    let picture = picture_from_reply(&reply).unwrap();
    assert_eq!(picture.bytes, b"ABC");
    assert_eq!(picture.mime, "image/jpeg");
  }

  #[test]
  fn a_words_only_reply_is_an_error_that_quotes_it() {
    let reply = json!({"candidates":[{"content":{"parts":[{"text":"I cannot draw that"}]}}]});
    assert!(picture_from_reply(&reply).err().unwrap().contains("I cannot draw that"));
  }
}
