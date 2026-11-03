//! 3D models on Fal with the user's own key. Field names and allowed values come from Fal's
//! schemas for each endpoint.

use serde_json::{json, Map, Value};

use crate::core::cloud_models::fal_job::FalJob;

pub const HUNYUAN_3D_3: &str = "hunyuan_3d_3";
pub const HUNYUAN_3D_2_0: &str = "hunyuan_3d_2_0";

/// What the 3D page sends, before it is shaped for a model.
pub struct MeshInputs<'a> {
  pub prompt: Option<&'a str>,
  /// A `data:` URI of the picture to build from.
  pub image: Option<String>,
  /// "normal", "low_poly" or "geometry".
  pub output_type: Option<&'a str>,
  /// "triangle" or "quad".
  pub polygon_type: Option<&'a str>,
  pub face_count: Option<u32>,
  pub enable_pbr: Option<bool>,
  pub enable_texture: Option<bool>,
}

pub fn is_supported(model_id: &str) -> bool {
  matches!(model_id, HUNYUAN_3D_3 | HUNYUAN_3D_2_0)
}

pub fn fal_mesh_job(model_id: &str, inputs: &MeshInputs) -> Result<FalJob, String> {
  let prompt = inputs.prompt.map(str::trim).filter(|p| !p.is_empty());
  match model_id {
    HUNYUAN_3D_3 => {
      let mut input = Map::new();
      let endpoint = match (&inputs.image, prompt) {
        (Some(image), _) => {
          input.insert("input_image_url".into(), json!(image));
          "fal-ai/hunyuan3d-v3/image-to-3d"
        }
        (None, Some(prompt)) => {
          input.insert("prompt".into(), json!(prompt));
          "fal-ai/hunyuan3d-v3/text-to-3d"
        }
        (None, None) => return Err("Add a picture or describe the object to build.".to_string()),
      };
      if let Some(kind) = inputs.output_type.and_then(generate_type) {
        input.insert("generate_type".into(), json!(kind));
      }
      if let Some(polygon) = inputs.polygon_type.and_then(polygon_type) {
        input.insert("polygon_type".into(), json!(polygon));
      }
      if let Some(count) = inputs.face_count {
        input.insert("face_count".into(), json!(count));
      }
      if let Some(pbr) = inputs.enable_pbr {
        input.insert("enable_pbr".into(), json!(pbr));
      }
      Ok(FalJob { endpoint, input: Value::Object(input) })
    }
    HUNYUAN_3D_2_0 => {
      let image = inputs.image.as_ref().ok_or("Hunyuan 3D 2.0 builds from a picture. Add one first.")?;
      let mut input = Map::new();
      input.insert("input_image_url".into(), json!(image));
      if let Some(textured) = inputs.enable_texture {
        input.insert("textured_mesh".into(), json!(textured));
      }
      Ok(FalJob { endpoint: "fal-ai/hunyuan3d/v2", input: Value::Object(input) })
    }
    other => Err(format!("{} is not available on Fal in this app yet.", other)),
  }
}

fn generate_type(output_type: &str) -> Option<&'static str> {
  match output_type {
    "normal" => Some("Normal"),
    "low_poly" => Some("LowPoly"),
    "geometry" => Some("Geometry"),
    _ => None,
  }
}

fn polygon_type(polygon: &str) -> Option<&'static str> {
  match polygon {
    "triangle" => Some("triangle"),
    "quad" => Some("quadrilateral"),
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_picture_wins_over_a_prompt_for_hunyuan_3() {
    let job = fal_mesh_job(HUNYUAN_3D_3, &MeshInputs { image: Some("data:x".into()), prompt: Some("a chair"), ..empty() }).unwrap();
    assert_eq!(job.endpoint, "fal-ai/hunyuan3d-v3/image-to-3d");
    assert_eq!(job.input["input_image_url"], "data:x");
    assert!(job.input.get("prompt").is_none());
  }

  #[test]
  fn a_prompt_alone_builds_from_text() {
    let job = fal_mesh_job(HUNYUAN_3D_3, &MeshInputs { prompt: Some(" a chair "), ..empty() }).unwrap();
    assert_eq!(job.endpoint, "fal-ai/hunyuan3d-v3/text-to-3d");
    assert_eq!(job.input["prompt"], "a chair");
  }

  #[test]
  fn nothing_to_build_from_is_refused() {
    assert!(fal_mesh_job(HUNYUAN_3D_3, &MeshInputs { prompt: Some("  "), ..empty() }).is_err());
  }

  #[test]
  fn the_pages_words_become_fals_words() {
    let inputs = MeshInputs {
      image: Some("data:x".into()), output_type: Some("low_poly"), polygon_type: Some("quad"),
      face_count: Some(20_000), enable_pbr: Some(true), ..empty()
    };
    let job = fal_mesh_job(HUNYUAN_3D_3, &inputs).unwrap();
    assert_eq!(job.input["generate_type"], "LowPoly");
    assert_eq!(job.input["polygon_type"], "quadrilateral");
    assert_eq!(job.input["face_count"], 20_000);
    assert_eq!(job.input["enable_pbr"], true);
  }

  #[test]
  fn unknown_options_are_left_out() {
    let inputs = MeshInputs { image: Some("data:x".into()), output_type: Some("mystery"), polygon_type: Some("hex"), ..empty() };
    let job = fal_mesh_job(HUNYUAN_3D_3, &inputs).unwrap();
    assert!(job.input.get("generate_type").is_none());
    assert!(job.input.get("polygon_type").is_none());
  }

  #[test]
  fn hunyuan_2_needs_a_picture_and_maps_texture() {
    assert!(fal_mesh_job(HUNYUAN_3D_2_0, &MeshInputs { prompt: Some("a chair"), ..empty() }).is_err());
    let job = fal_mesh_job(HUNYUAN_3D_2_0, &MeshInputs { image: Some("data:x".into()), enable_texture: Some(true), ..empty() }).unwrap();
    assert_eq!(job.endpoint, "fal-ai/hunyuan3d/v2");
    assert_eq!(job.input["textured_mesh"], true);
  }

  #[test]
  fn other_models_are_refused() {
    assert!(!is_supported("meshy_6"));
    assert!(fal_mesh_job("meshy_6", &empty()).is_err());
  }

  fn empty() -> MeshInputs<'static> {
    MeshInputs { prompt: None, image: None, output_type: None, polygon_type: None, face_count: None, enable_pbr: None, enable_texture: None }
  }
}
