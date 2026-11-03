//! Music on Fal with the user's own key: ACE-Step, the same open model that runs locally, but on
//! Fal's GPUs and for longer songs. Field names come from Fal's schema for `fal-ai/ace-step`.

use serde_json::json;

use crate::core::cloud_models::fal_job::FalJob;
use crate::core::local_models::local_audio_model::tags_and_lyrics;

pub const FAL_ACE_STEP_MODEL_ID: &str = "ace_step_fal";

const ENDPOINT: &str = "fal-ai/ace-step";
const SONG_SECONDS: u32 = 60;

pub fn fal_audio_job(prompt: &str, style: &str, instrumental: bool, seed: u64) -> Result<FalJob, String> {
  if prompt.trim().is_empty() && style.trim().is_empty() {
    return Err("Describe the music you want, or write some lyrics".to_string());
  }
  let (tags, lyrics) = tags_and_lyrics(prompt, style, instrumental);
  Ok(FalJob {
    endpoint: ENDPOINT,
    input: json!({ "tags": tags, "lyrics": lyrics, "duration": SONG_SECONDS, "seed": seed }),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn lyrics_and_style_go_to_their_own_fields() {
    let job = fal_audio_job("la la la", "synth pop", false, 7).unwrap();
    assert_eq!(job.endpoint, "fal-ai/ace-step");
    assert_eq!(job.input["tags"], "synth pop");
    assert_eq!(job.input["lyrics"], "la la la");
    assert_eq!(job.input["duration"], 60);
    assert_eq!(job.input["seed"], 7);
  }

  #[test]
  fn an_instrumental_says_so_in_the_lyrics() {
    let job = fal_audio_job("calm piano", "", true, 1).unwrap();
    assert_eq!(job.input["tags"], "calm piano");
    assert_eq!(job.input["lyrics"], "[instrumental]");
  }

  #[test]
  fn an_empty_request_is_refused() {
    assert!(fal_audio_job("  ", "", false, 1).is_err());
  }
}
