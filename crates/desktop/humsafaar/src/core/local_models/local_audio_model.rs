use comfyui_client::workflows::AceStepParams;
use comfyui_manager::models::{catalog_model, CatalogModel};

/// The model id the audio page sends.
pub const ACE_STEP_MODEL_ID: &str = "ace_step_1p5_turbo";

const DEFAULT_BPM: u32 = 120;
const MIN_BPM: u32 = 40;
const MAX_BPM: u32 = 220;
const DEFAULT_KEY: &str = "C major";

/// Used when the user gives no style and wants vocals, since the model needs some direction.
const FALLBACK_STYLE: &str = "pop";
const INSTRUMENTAL_LYRICS: &str = "[instrumental]";

/// The length of every song. Fixed so a song takes a predictable time and memory.
const SONG_SECONDS: f32 = 30.0;

/// The music models that run on the local engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalAudioModel {
  AceStep15Turbo,
}

impl LocalAudioModel {
  pub fn from_model_id(id: &str) -> Option<Self> {
    match id {
      ACE_STEP_MODEL_ID => Some(Self::AceStep15Turbo),
      _ => None,
    }
  }

  /// The downloadable model this one needs.
  pub fn catalog(&self) -> &'static CatalogModel {
    catalog_model(ACE_STEP_MODEL_ID).expect("every local audio model is in the catalog")
  }

  pub fn display_name(&self) -> &'static str {
    self.catalog().display_name
  }

  /// `prompt` is the lyrics (or a description for an instrumental), `style` the genre and mood.
  pub fn params(
    &self,
    prompt: &str,
    style: &str,
    instrumental: bool,
    bpm: Option<u32>,
    musical_key: Option<&str>,
    seed: u64,
  ) -> AceStepParams {
    let (tags, lyrics) = tags_and_lyrics(prompt, style, instrumental);
    let mut params = AceStepParams::new(&tags, &lyrics, seed);
    params.checkpoint = self.catalog().primary_file().file_name.to_string();
    params.seconds = SONG_SECONDS;
    params.bpm = bpm.unwrap_or(DEFAULT_BPM).clamp(MIN_BPM, MAX_BPM);
    params.key_scale = musical_key.map(key_scale).unwrap_or_else(|| DEFAULT_KEY.to_string());
    params
  }
}

/// How the page's boxes map onto the model's two text inputs.
/// Instrumental: the lyrics say so, and the style (or else the prompt) describes the music.
/// With vocals: the prompt is the lyrics and the style is the genre.
pub(crate) fn tags_and_lyrics(prompt: &str, style: &str, instrumental: bool) -> (String, String) {
  let (prompt, style) = (prompt.trim(), style.trim());
  if instrumental {
    let tags = if !style.is_empty() { style } else if !prompt.is_empty() { prompt } else { FALLBACK_STYLE };
    (tags.to_string(), INSTRUMENTAL_LYRICS.to_string())
  } else {
    let tags = if style.is_empty() { FALLBACK_STYLE } else { style };
    (tags.to_string(), prompt.to_string())
  }
}

/// "d_minor" becomes "D minor". "auto" and anything unrecognised become the default key.
fn key_scale(musical_key: &str) -> String {
  let mut parts = musical_key.split('_');
  let (Some(note), Some(mode), None) = (parts.next(), parts.next(), parts.next()) else {
    return DEFAULT_KEY.to_string();
  };
  let valid_note = note.len() == 1 && note.chars().all(|c| ('a'..='g').contains(&c.to_ascii_lowercase()));
  if valid_note && (mode == "major" || mode == "minor") {
    format!("{} {}", note.to_ascii_uppercase(), mode)
  } else {
    DEFAULT_KEY.to_string()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const MODEL: LocalAudioModel = LocalAudioModel::AceStep15Turbo;

  mod models {
    use super::*;

    #[test]
    fn only_the_known_id_maps() {
      assert_eq!(LocalAudioModel::from_model_id("ace_step_1p5_turbo"), Some(MODEL));
      assert_eq!(LocalAudioModel::from_model_id("suno_music"), None);
    }

    #[test]
    fn it_loads_the_catalog_checkpoint() {
      let params = MODEL.params("la la", "folk", false, None, None, 1);
      assert_eq!(params.checkpoint, "ace_step_1.5_turbo_aio.safetensors");
      assert_eq!(MODEL.display_name(), "ACE-Step 1.5 Music");
    }
  }

  mod text_inputs {
    use super::*;

    #[test]
    fn vocals_use_the_prompt_as_lyrics_and_the_style_as_tags() {
      assert_eq!(tags_and_lyrics("[Verse]\nla la", "folk, calm", false), ("folk, calm".to_string(), "[Verse]\nla la".to_string()));
    }

    #[test]
    fn vocals_without_a_style_still_get_a_genre() {
      assert_eq!(tags_and_lyrics("la la", "  ", false).0, "pop");
    }

    #[test]
    fn instrumentals_say_so_in_the_lyrics() {
      assert_eq!(tags_and_lyrics("ignored lyrics", "ambient pads", true), ("ambient pads".to_string(), "[instrumental]".to_string()));
    }

    #[test]
    fn an_instrumental_without_a_style_uses_the_prompt_as_the_description() {
      assert_eq!(tags_and_lyrics("a calm piano piece", "", true).0, "a calm piano piece");
      assert_eq!(tags_and_lyrics("", "", true).0, "pop");
    }
  }

  mod key_and_tempo {
    use super::*;

    #[test]
    fn keys_are_converted_to_the_models_wording() {
      assert_eq!(key_scale("d_minor"), "D minor");
      assert_eq!(key_scale("c_major"), "C major");
      assert_eq!(key_scale("b_minor"), "B minor");
    }

    #[test]
    fn auto_and_odd_keys_fall_back_to_the_default() {
      for odd in ["auto", "", "h_major", "c_sharp_minor", "c_lydian", "cc_major"] {
        assert_eq!(key_scale(odd), "C major", "{odd}");
      }
    }

    #[test]
    fn tempo_is_kept_in_a_musical_range() {
      assert_eq!(MODEL.params("x", "y", false, Some(90), None, 1).bpm, 90);
      assert_eq!(MODEL.params("x", "y", false, Some(5), None, 1).bpm, 40);
      assert_eq!(MODEL.params("x", "y", false, Some(900), None, 1).bpm, 220);
      assert_eq!(MODEL.params("x", "y", false, None, None, 1).bpm, 120);
    }

    #[test]
    fn every_song_is_the_same_length() {
      assert_eq!(MODEL.params("x", "y", false, None, None, 1).seconds, 30.0);
    }
  }
}
