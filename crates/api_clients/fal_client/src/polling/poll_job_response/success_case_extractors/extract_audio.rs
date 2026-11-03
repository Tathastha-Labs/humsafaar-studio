use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct PollResponseAudioData {
  pub url: Option<String>,
  pub content_type: Option<String>,
  pub file_name: Option<String>,
  pub file_size: Option<u64>,
}

pub fn extract_audio(value: &Value) -> Option<PollResponseAudioData> {
  let audio = value.get("audio")?;
  serde_json::from_value(audio.clone()).ok()
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn reads_the_audio_file() {
    let value = json!({ "audio": { "url": "https://cdn/x.mp3", "content_type": "audio/mpeg" }, "seed": 1 });
    let audio = extract_audio(&value).unwrap();
    assert_eq!(audio.url.as_deref(), Some("https://cdn/x.mp3"));
  }

  #[test]
  fn a_response_without_audio_gives_none() {
    assert!(extract_audio(&json!({ "video": { "url": "u" } })).is_none());
  }
}
