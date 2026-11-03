use crate::core::commands::generate::generate_video::request::TauriVideoModel;
use comfyui_client::workflows::WanVideoParams;
use comfyui_manager::models::{catalog_model, CatalogModel};

const FRAMES_PER_SECOND: u32 = 24;

/// The only size and length that have produced a clean clip on a real run (576x576, 1 second, on
/// a 24 GB Apple Silicon Mac). Larger sizes and longer clips developed artifacts such as colored
/// blocks and washed-out frames, and took far longer, so nothing else is offered yet.
const VERIFIED_SIZE: (u32, u32) = (576, 576);
const VERIFIED_SECONDS: u32 = 1;

/// The video models that run on the local engine, and how each one likes to be driven.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalVideoModel {
  Wan22Ti2v5b,
}

impl LocalVideoModel {
  pub fn from_tauri(model: TauriVideoModel) -> Option<Self> {
    match model {
      TauriVideoModel::Wan22Ti2v5b => Some(Self::Wan22Ti2v5b),
      _ => None,
    }
  }

  /// The downloadable model this one needs.
  pub fn catalog(&self) -> &'static CatalogModel {
    catalog_model("wan_2p2_ti2v_5b").expect("every local video model is in the catalog")
  }

  pub fn display_name(&self) -> &'static str {
    self.catalog().display_name
  }

  /// The ComfyUI settings for this model: the three files it loads, and how to sample.
  pub fn params(&self, prompt: &str, negative_prompt: &str, seed: u64) -> WanVideoParams {
    let mut params = WanVideoParams::new(prompt, seed);
    params.negative_prompt = negative_prompt.to_string();

    let file_in = |folder: &str| self.catalog().files.iter()
      .find(|file| file.folder == folder)
      .expect("the catalog lists every file Wan needs")
      .file_name
      .to_string();
    params.diffusion_model = file_in("diffusion_models");
    params.text_encoder = file_in("text_encoders");
    params.vae = file_in("vae");

    let (width, height) = self.dimensions();
    params.width = width;
    params.height = height;
    params.frames = self.frames();
    params
  }

  /// The size every clip is made at. See `VERIFIED_SIZE`.
  pub fn dimensions(&self) -> (u32, u32) {
    VERIFIED_SIZE
  }

  /// The model wants a frame count of 4n + 1, at 24 frames a second. See `VERIFIED_SECONDS`.
  pub fn frames(&self) -> u32 {
    VERIFIED_SECONDS * FRAMES_PER_SECOND + 1
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const WAN: LocalVideoModel = LocalVideoModel::Wan22Ti2v5b;

  mod models {
    use super::*;

    #[test]
    fn only_the_local_model_maps() {
      assert_eq!(LocalVideoModel::from_tauri(TauriVideoModel::Wan22Ti2v5b), Some(WAN));
      assert_eq!(LocalVideoModel::from_tauri(TauriVideoModel::Veo3), None);
    }

    #[test]
    fn it_loads_the_three_catalog_files_by_name() {
      let params = WAN.params("a fox", "", 1);
      assert_eq!(params.diffusion_model, "wan2.2_ti2v_5B_fp16.safetensors");
      assert_eq!(params.text_encoder, "umt5_xxl_fp8_e4m3fn_scaled.safetensors");
      assert_eq!(params.vae, "wan2.2_vae.safetensors");
      assert_eq!(WAN.display_name(), "Wan 2.2 Video (5B)");
    }

    #[test]
    fn prompts_and_seed_are_passed_through() {
      let params = WAN.params("a fox runs", "blurry", 77);
      assert_eq!(params.prompt, "a fox runs");
      assert_eq!(params.negative_prompt, "blurry");
      assert_eq!(params.seed, 77);
      assert!(params.start_image.is_none());
    }
  }

  mod verified_settings {
    use super::*;

    #[test]
    fn clips_are_always_the_size_and_length_that_worked() {
      let params = WAN.params("a fox", "", 1);
      assert_eq!((params.width, params.height), (576, 576));
      assert_eq!(params.frames, 25);
      assert_eq!(params.steps, 20);
    }

    #[test]
    fn the_size_and_frame_count_satisfy_the_models_rules() {
      let (w, h) = WAN.dimensions();
      assert_eq!((w % 32, h % 32), (0, 0), "the decoder needs multiples of 32");
      assert_eq!((WAN.frames() - 1) % 4, 0, "the model wants 4n + 1 frames");
    }
  }
}
