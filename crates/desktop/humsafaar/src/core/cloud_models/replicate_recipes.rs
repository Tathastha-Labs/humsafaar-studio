//! What to send Replicate for each model. Field names and allowed values come from each model's
//! published input schema on replicate.com. Each model takes text or a start picture in one endpoint.

use humsafaar_router::api::router_aspect_ratio::RouterAspectRatio;
use humsafaar_router::api::router_resolution::RouterResolution;
use serde_json::{json, Map, Value};

use crate::core::cloud_models::fal_video::VideoInputs;
use crate::core::cloud_models::fal_mesh::{MeshInputs, HUNYUAN_3D_3};
use crate::core::commands::generate::generate_video::request::TauriVideoModel;

pub const MINIMAX_MUSIC_MODEL_ID: &str = "minimax_music_replicate";

const KLING_DURATIONS: [u16; 2] = [5, 10];
const VEO_DURATIONS: [u16; 3] = [4, 6, 8];
const SEEDANCE_MIN_SECONDS: u16 = 4;
const SEEDANCE_MAX_SECONDS: u16 = 12;

#[derive(Debug, PartialEq)]
pub struct ReplicateJob {
  /// `owner/name` of an official model.
  pub model: &'static str,
  pub input: Value,
}

pub fn is_video_supported(model: TauriVideoModel) -> bool {
  matches!(
    model,
    TauriVideoModel::Kling2p5TurboPro | TauriVideoModel::Kling2p6Pro | TauriVideoModel::Veo3p1Fast | TauriVideoModel::Seedance1p5Pro
  )
}

pub fn replicate_video_job(model: TauriVideoModel, inputs: &VideoInputs) -> Result<ReplicateJob, String> {
  let mut input = Map::new();
  input.insert("prompt".into(), json!(inputs.prompt));
  let picture = inputs.start_image.as_ref().map(|image| json!(image));

  let model_name = match model {
    TauriVideoModel::Kling2p5TurboPro => {
      put(&mut input, "start_image", picture);
      put_text(&mut input, "negative_prompt", inputs.negative_prompt);
      put(&mut input, "duration", inputs.duration_seconds.map(|s| json!(nearest(s, &KLING_DURATIONS))));
      put(&mut input, "aspect_ratio", aspect(inputs.aspect_ratio, &["16:9", "9:16", "1:1"]));
      "kwaivgi/kling-v2.5-turbo-pro"
    }
    TauriVideoModel::Kling2p6Pro => {
      put(&mut input, "start_image", picture);
      put_text(&mut input, "negative_prompt", inputs.negative_prompt);
      put(&mut input, "duration", inputs.duration_seconds.map(|s| json!(nearest(s, &KLING_DURATIONS))));
      put(&mut input, "aspect_ratio", aspect(inputs.aspect_ratio, &["16:9", "9:16", "1:1"]));
      put(&mut input, "generate_audio", inputs.generate_audio.map(Value::Bool));
      "kwaivgi/kling-v2.6"
    }
    TauriVideoModel::Veo3p1Fast => {
      put(&mut input, "image", picture);
      put_text(&mut input, "negative_prompt", inputs.negative_prompt);
      put(&mut input, "duration", inputs.duration_seconds.map(|s| json!(nearest(s, &VEO_DURATIONS))));
      put(&mut input, "aspect_ratio", aspect(inputs.aspect_ratio, &["16:9", "9:16"]));
      put(&mut input, "resolution", resolution(inputs.resolution, &["720p", "1080p"]));
      put(&mut input, "generate_audio", inputs.generate_audio.map(Value::Bool));
      "google/veo-3.1-fast"
    }
    TauriVideoModel::Seedance1p5Pro => {
      put(&mut input, "image", picture);
      put(&mut input, "duration", inputs.duration_seconds.map(|s| json!(s.clamp(SEEDANCE_MIN_SECONDS, SEEDANCE_MAX_SECONDS))));
      put(&mut input, "aspect_ratio", aspect(inputs.aspect_ratio, &["16:9", "4:3", "1:1", "3:4", "9:16", "21:9"]));
      put(&mut input, "resolution", resolution(inputs.resolution, &["480p", "720p", "1080p"]));
      put(&mut input, "generate_audio", inputs.generate_audio.map(Value::Bool));
      "bytedance/seedance-1.5-pro"
    }
    other => return Err(format!("{:?} is not available on Replicate in this app yet.", other)),
  };
  Ok(ReplicateJob { model: model_name, input: Value::Object(input) })
}

/// Replicate carries Hunyuan 3D 3.1 (the page's Hunyuan 3.0 entry is its closest match there).
pub fn is_mesh_supported(model_id: &str) -> bool {
  model_id == HUNYUAN_3D_3
}

pub fn replicate_mesh_job(model_id: &str, inputs: &MeshInputs) -> Result<ReplicateJob, String> {
  if !is_mesh_supported(model_id) {
    return Err(format!("{} is not available on Replicate in this app yet.", model_id));
  }
  let prompt = inputs.prompt.map(str::trim).filter(|p| !p.is_empty());
  let mut input = Map::new();
  match (&inputs.image, prompt) {
    (Some(image), _) => { input.insert("image".into(), json!(image)); }
    (None, Some(prompt)) => { input.insert("prompt".into(), json!(prompt)); }
    (None, None) => return Err("Add a picture or describe the object to build.".to_string()),
  }
  // This model offers textured ("Normal") or plain ("Geometry") only.
  let generate_type = if inputs.output_type == Some("geometry") { "Geometry" } else { "Normal" };
  input.insert("generate_type".into(), json!(generate_type));
  if let Some(count) = inputs.face_count {
    input.insert("face_count".into(), json!(count));
  }
  if let Some(pbr) = inputs.enable_pbr {
    input.insert("enable_pbr".into(), json!(pbr));
  }
  Ok(ReplicateJob { model: "tencent/hunyuan-3d-3.1", input: Value::Object(input) })
}

/// MiniMax Music 2.5 sings the lyrics in the style described. It needs lyrics, so there is no
/// instrumental mode.
pub fn replicate_music_job(lyrics: &str, style: &str) -> Result<ReplicateJob, String> {
  let lyrics = lyrics.trim();
  if lyrics.is_empty() {
    return Err("This model sings, so it needs lyrics. Write some in the prompt box.".to_string());
  }
  let mut input = Map::new();
  input.insert("lyrics".into(), json!(lyrics));
  put_text(&mut input, "prompt", Some(style));
  Ok(ReplicateJob { model: "minimax/music-2.5", input: Value::Object(input) })
}

fn put(input: &mut Map<String, Value>, key: &str, value: Option<Value>) {
  if let Some(value) = value {
    input.insert(key.to_string(), value);
  }
}

fn put_text(input: &mut Map<String, Value>, key: &str, value: Option<&str>) {
  if let Some(text) = value.map(str::trim).filter(|t| !t.is_empty()) {
    input.insert(key.to_string(), json!(text));
  }
}

fn nearest(seconds: u16, allowed: &[u16]) -> u16 {
  allowed.iter().copied().min_by_key(|a| a.abs_diff(seconds)).unwrap_or(seconds)
}

fn aspect(ratio: Option<RouterAspectRatio>, allowed: &[&str]) -> Option<Value> {
  use RouterAspectRatio::*;
  let text = match ratio? {
    Square | SquareHd => "1:1",
    WideSixteenByNine | Wide => "16:9",
    WideTwentyOneByNine => "21:9",
    WideFourByThree => "4:3",
    TallNineBySixteen | Tall => "9:16",
    TallThreeByFour => "3:4",
    _ => return None,
  };
  allowed.contains(&text).then(|| json!(text))
}

fn resolution(resolution: Option<RouterResolution>, allowed: &[&str]) -> Option<Value> {
  let text = match resolution? {
    RouterResolution::FourEightyP => "480p",
    RouterResolution::SevenTwentyP => "720p",
    RouterResolution::TenEightyP => "1080p",
    _ => return None,
  };
  allowed.contains(&text).then(|| json!(text))
}

#[cfg(test)]
mod tests {
  use super::*;

  mod video {
    use super::*;

    #[test]
    fn each_model_names_its_picture_field() {
      let with = |model| replicate_video_job(model, &inputs(Some("https://x/a.png"))).unwrap();
      assert_eq!(with(TauriVideoModel::Kling2p5TurboPro).input["start_image"], "https://x/a.png");
      assert_eq!(with(TauriVideoModel::Kling2p6Pro).input["start_image"], "https://x/a.png");
      assert_eq!(with(TauriVideoModel::Veo3p1Fast).input["image"], "https://x/a.png");
      assert_eq!(with(TauriVideoModel::Seedance1p5Pro).input["image"], "https://x/a.png");
    }

    #[test]
    fn no_picture_means_no_picture_field() {
      let job = replicate_video_job(TauriVideoModel::Veo3p1Fast, &inputs(None)).unwrap();
      assert_eq!(job.model, "google/veo-3.1-fast");
      assert!(job.input.get("image").is_none());
    }

    #[test]
    fn lengths_are_whole_numbers_snapped_to_what_the_model_allows() {
      let mut request = inputs(None);
      request.duration_seconds = Some(7);
      assert_eq!(replicate_video_job(TauriVideoModel::Kling2p5TurboPro, &request).unwrap().input["duration"], 5);
      assert_eq!(replicate_video_job(TauriVideoModel::Veo3p1Fast, &request).unwrap().input["duration"], 6);
      assert_eq!(replicate_video_job(TauriVideoModel::Seedance1p5Pro, &request).unwrap().input["duration"], 7);
    }

    #[test]
    fn options_a_model_lacks_are_not_sent() {
      let mut request = inputs(None);
      request.generate_audio = Some(true);
      request.resolution = Some(RouterResolution::TenEightyP);
      let kling = replicate_video_job(TauriVideoModel::Kling2p5TurboPro, &request).unwrap();
      assert!(kling.input.get("generate_audio").is_none());
      assert!(kling.input.get("resolution").is_none());
      assert_eq!(replicate_video_job(TauriVideoModel::Veo3p1Fast, &request).unwrap().input["resolution"], "1080p");
    }

    #[test]
    fn other_models_are_refused() {
      assert!(!is_video_supported(TauriVideoModel::Sora2));
      assert!(replicate_video_job(TauriVideoModel::Sora2, &inputs(None)).is_err());
    }
  }

  mod mesh {
    use super::*;

    #[test]
    fn a_picture_or_a_prompt_builds_the_input() {
      let image = replicate_mesh_job(HUNYUAN_3D_3, &MeshInputs { image: Some("https://x/a.png".into()), ..empty() }).unwrap();
      assert_eq!(image.model, "tencent/hunyuan-3d-3.1");
      assert_eq!(image.input["image"], "https://x/a.png");
      let text = replicate_mesh_job(HUNYUAN_3D_3, &MeshInputs { prompt: Some("a chair"), ..empty() }).unwrap();
      assert_eq!(text.input["prompt"], "a chair");
    }

    #[test]
    fn low_poly_falls_back_to_the_textured_type_and_geometry_is_kept() {
      let low = replicate_mesh_job(HUNYUAN_3D_3, &MeshInputs { prompt: Some("a"), output_type: Some("low_poly"), ..empty() }).unwrap();
      assert_eq!(low.input["generate_type"], "Normal");
      let geometry = replicate_mesh_job(HUNYUAN_3D_3, &MeshInputs { prompt: Some("a"), output_type: Some("geometry"), ..empty() }).unwrap();
      assert_eq!(geometry.input["generate_type"], "Geometry");
    }

    #[test]
    fn nothing_to_build_from_is_refused() {
      assert!(replicate_mesh_job(HUNYUAN_3D_3, &empty()).is_err());
      assert!(replicate_mesh_job("hunyuan_3d_2_0", &empty()).is_err());
    }

    fn empty() -> MeshInputs<'static> {
      MeshInputs { prompt: None, image: None, output_type: None, polygon_type: None, face_count: None, enable_pbr: None, enable_texture: None }
    }
  }

  mod music {
    use super::*;

    #[test]
    fn lyrics_and_style_go_to_their_fields() {
      let job = replicate_music_job("la la la", "synth pop").unwrap();
      assert_eq!(job.model, "minimax/music-2.5");
      assert_eq!(job.input["lyrics"], "la la la");
      assert_eq!(job.input["prompt"], "synth pop");
    }

    #[test]
    fn no_lyrics_is_refused_with_a_reason() {
      assert!(replicate_music_job("  ", "synth pop").unwrap_err().contains("lyrics"));
    }

    #[test]
    fn a_blank_style_is_not_sent() {
      assert!(replicate_music_job("la", "  ").unwrap().input.get("prompt").is_none());
    }
  }

  fn inputs(start_image: Option<&str>) -> VideoInputs<'static> {
    VideoInputs {
      prompt: "a lighthouse at dusk",
      negative_prompt: None,
      start_image: start_image.map(str::to_string),
      aspect_ratio: None,
      resolution: None,
      duration_seconds: None,
      generate_audio: None,
    }
  }
}
