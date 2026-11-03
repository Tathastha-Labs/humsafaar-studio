//! Video on Fal with the user's own key: which Fal endpoint each model uses and what it is sent.
//! Field names and allowed values come from Fal's published schemas for each endpoint.

use humsafaar_router::api::router_aspect_ratio::RouterAspectRatio;
use humsafaar_router::api::router_resolution::RouterResolution;
use serde_json::{json, Map, Value};

use crate::core::cloud_models::fal_job::FalJob;
use crate::core::commands::generate::generate_video::request::TauriVideoModel;

const KLING_DURATIONS: [u16; 2] = [5, 10];
const VEO_DURATIONS: [u16; 3] = [4, 6, 8];
const SEEDANCE_MIN_SECONDS: u16 = 4;
const SEEDANCE_MAX_SECONDS: u16 = 12;

/// What the user asked for, before it is shaped for a model.
pub struct VideoInputs<'a> {
  pub prompt: &'a str,
  pub negative_prompt: Option<&'a str>,
  /// A `data:` URI of the start picture. Without one, the model makes video from text.
  pub start_image: Option<String>,
  pub aspect_ratio: Option<RouterAspectRatio>,
  pub resolution: Option<RouterResolution>,
  pub duration_seconds: Option<u16>,
  pub generate_audio: Option<bool>,
}

/// Models that can run on Fal, by the ids the video page uses.
pub fn is_supported(model: TauriVideoModel) -> bool {
  matches!(
    model,
    TauriVideoModel::Kling2p5TurboPro
      | TauriVideoModel::Kling2p6Pro
      | TauriVideoModel::Veo3p1Fast
      | TauriVideoModel::Seedance1p5Pro
  )
}

pub fn fal_video_job(model: TauriVideoModel, inputs: &VideoInputs) -> Result<FalJob, String> {
  let from_picture = inputs.start_image.is_some();
  let mut input = Map::new();
  input.insert("prompt".into(), json!(inputs.prompt));
  if let Some(image) = &inputs.start_image {
    input.insert("image_url".into(), json!(image));
  }

  let endpoint = match model {
    TauriVideoModel::Kling2p5TurboPro => {
      put_text(&mut input, "negative_prompt", inputs.negative_prompt);
      put(&mut input, "duration", inputs.duration_seconds.map(|s| json!(nearest(s, &KLING_DURATIONS).to_string())));
      if !from_picture {
        put(&mut input, "aspect_ratio", aspect(inputs.aspect_ratio, &["16:9", "9:16", "1:1"]));
      }
      pick(from_picture,
        "fal-ai/kling-video/v2.5-turbo/pro/image-to-video",
        "fal-ai/kling-video/v2.5-turbo/pro/text-to-video")
    }
    TauriVideoModel::Kling2p6Pro => {
      put_text(&mut input, "negative_prompt", inputs.negative_prompt);
      put(&mut input, "duration", inputs.duration_seconds.map(|s| json!(nearest(s, &KLING_DURATIONS).to_string())));
      put(&mut input, "generate_audio", inputs.generate_audio.map(Value::Bool));
      if !from_picture {
        put(&mut input, "aspect_ratio", aspect(inputs.aspect_ratio, &["16:9", "9:16", "1:1"]));
      }
      pick(from_picture,
        "fal-ai/kling-video/v2.6/pro/image-to-video",
        "fal-ai/kling-video/v2.6/pro/text-to-video")
    }
    TauriVideoModel::Veo3p1Fast => {
      put_text(&mut input, "negative_prompt", inputs.negative_prompt.filter(|_| !from_picture));
      put(&mut input, "duration", inputs.duration_seconds.map(|s| json!(format!("{}s", nearest(s, &VEO_DURATIONS)))));
      put(&mut input, "aspect_ratio", aspect(inputs.aspect_ratio, &["16:9", "9:16"]));
      put(&mut input, "resolution", resolution(inputs.resolution, &["720p", "1080p"]));
      put(&mut input, "generate_audio", inputs.generate_audio.map(Value::Bool));
      pick(from_picture, "fal-ai/veo3.1/fast/image-to-video", "fal-ai/veo3.1/fast")
    }
    TauriVideoModel::Seedance1p5Pro => {
      let seconds = inputs.duration_seconds.map(|s| s.clamp(SEEDANCE_MIN_SECONDS, SEEDANCE_MAX_SECONDS));
      put(&mut input, "duration", seconds.map(|s| json!(s.to_string())));
      put(&mut input, "aspect_ratio", aspect(inputs.aspect_ratio, &["21:9", "16:9", "4:3", "1:1", "3:4", "9:16"]));
      put(&mut input, "resolution", resolution(inputs.resolution, &["480p", "720p", "1080p"]));
      put(&mut input, "generate_audio", inputs.generate_audio.map(Value::Bool));
      pick(from_picture,
        "fal-ai/bytedance/seedance/v1.5/pro/image-to-video",
        "fal-ai/bytedance/seedance/v1.5/pro/text-to-video")
    }
    other => return Err(format!("{:?} is not available on Fal in this app yet.", other)),
  };

  Ok(FalJob { endpoint, input: Value::Object(input) })
}

fn pick(from_picture: bool, with_picture: &'static str, from_text: &'static str) -> &'static str {
  if from_picture { with_picture } else { from_text }
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

/// The closest allowed length to what was asked for.
fn nearest(seconds: u16, allowed: &[u16]) -> u16 {
  allowed.iter().copied().min_by_key(|a| a.abs_diff(seconds)).unwrap_or(seconds)
}

/// The shape the user picked, if the model offers it. Otherwise the model's own default is kept.
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

  mod endpoints {
    use super::*;

    #[test]
    fn a_start_picture_picks_the_image_endpoint() {
      let job = fal_video_job(TauriVideoModel::Kling2p5TurboPro, &inputs(Some("data:image/png;base64,AAA"))).unwrap();
      assert_eq!(job.endpoint, "fal-ai/kling-video/v2.5-turbo/pro/image-to-video");
      assert_eq!(job.input["image_url"], "data:image/png;base64,AAA");
    }

    #[test]
    fn no_picture_picks_the_text_endpoint() {
      let job = fal_video_job(TauriVideoModel::Veo3p1Fast, &inputs(None)).unwrap();
      assert_eq!(job.endpoint, "fal-ai/veo3.1/fast");
      assert!(job.input.get("image_url").is_none());
    }

    #[test]
    fn every_supported_model_has_both_endpoints() {
      for model in [TauriVideoModel::Kling2p5TurboPro, TauriVideoModel::Kling2p6Pro, TauriVideoModel::Veo3p1Fast, TauriVideoModel::Seedance1p5Pro] {
        assert!(is_supported(model));
        let with = fal_video_job(model, &inputs(Some("data:x"))).unwrap();
        let without = fal_video_job(model, &inputs(None)).unwrap();
        assert_ne!(with.endpoint, without.endpoint, "{:?}", model);
      }
    }

    #[test]
    fn a_model_that_is_not_on_fal_is_refused() {
      assert!(!is_supported(TauriVideoModel::Sora2));
      assert!(fal_video_job(TauriVideoModel::Sora2, &inputs(None)).is_err());
    }
  }

  mod settings {
    use super::*;

    #[test]
    fn kling_lengths_snap_to_five_or_ten_and_are_sent_as_text() {
      for (asked, sent) in [(3, "5"), (5, "5"), (7, "5"), (8, "10"), (10, "10"), (15, "10")] {
        let mut request = inputs(None);
        request.duration_seconds = Some(asked);
        let job = fal_video_job(TauriVideoModel::Kling2p5TurboPro, &request).unwrap();
        assert_eq!(job.input["duration"], sent, "asked {}", asked);
      }
    }

    #[test]
    fn veo_lengths_carry_an_s() {
      let mut request = inputs(None);
      request.duration_seconds = Some(7);
      let job = fal_video_job(TauriVideoModel::Veo3p1Fast, &request).unwrap();
      assert_eq!(job.input["duration"], "6s");
    }

    #[test]
    fn seedance_lengths_are_kept_in_range() {
      let mut request = inputs(None);
      request.duration_seconds = Some(30);
      assert_eq!(fal_video_job(TauriVideoModel::Seedance1p5Pro, &request).unwrap().input["duration"], "12");
      request.duration_seconds = Some(1);
      assert_eq!(fal_video_job(TauriVideoModel::Seedance1p5Pro, &request).unwrap().input["duration"], "4");
    }

    #[test]
    fn only_shapes_the_model_offers_are_sent() {
      let mut request = inputs(None);
      request.aspect_ratio = Some(RouterAspectRatio::Square);
      assert_eq!(fal_video_job(TauriVideoModel::Kling2p6Pro, &request).unwrap().input["aspect_ratio"], "1:1");
      // Veo has no square, so it keeps its own default.
      assert!(fal_video_job(TauriVideoModel::Veo3p1Fast, &request).unwrap().input.get("aspect_ratio").is_none());
    }

    #[test]
    fn the_shape_is_left_to_the_picture_when_there_is_one() {
      let mut request = inputs(Some("data:x"));
      request.aspect_ratio = Some(RouterAspectRatio::WideSixteenByNine);
      let job = fal_video_job(TauriVideoModel::Kling2p5TurboPro, &request).unwrap();
      assert!(job.input.get("aspect_ratio").is_none());
    }

    #[test]
    fn resolution_is_only_sent_where_the_model_takes_it() {
      let mut request = inputs(None);
      request.resolution = Some(RouterResolution::TenEightyP);
      assert_eq!(fal_video_job(TauriVideoModel::Veo3p1Fast, &request).unwrap().input["resolution"], "1080p");
      assert!(fal_video_job(TauriVideoModel::Kling2p5TurboPro, &request).unwrap().input.get("resolution").is_none());
    }

    #[test]
    fn a_blank_negative_prompt_is_not_sent() {
      let mut request = inputs(None);
      request.negative_prompt = Some("   ");
      assert!(fal_video_job(TauriVideoModel::Kling2p5TurboPro, &request).unwrap().input.get("negative_prompt").is_none());
    }

    #[test]
    fn sound_is_only_sent_to_models_that_make_it() {
      let mut request = inputs(None);
      request.generate_audio = Some(true);
      assert_eq!(fal_video_job(TauriVideoModel::Kling2p6Pro, &request).unwrap().input["generate_audio"], true);
      assert!(fal_video_job(TauriVideoModel::Kling2p5TurboPro, &request).unwrap().input.get("generate_audio").is_none());
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
