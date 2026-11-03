use crate::core::commands::generate::generate_image::tauri_image_model::TauriImageModel;
use comfyui_manager::models::{catalog_model, CatalogModel};

/// The image models that run on the local engine, and how each one likes to be driven.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalImageModel {
  StableDiffusion15,
  StableDiffusionXl,
}

impl LocalImageModel {
  pub fn from_tauri(model: TauriImageModel) -> Option<Self> {
    match model {
      TauriImageModel::StableDiffusion15 => Some(Self::StableDiffusion15),
      TauriImageModel::StableDiffusionXl => Some(Self::StableDiffusionXl),
      _ => None,
    }
  }

  /// The downloadable model this one needs.
  pub fn catalog(&self) -> &'static CatalogModel {
    let id = match self {
      Self::StableDiffusion15 => "stable_diffusion_1p5",
      Self::StableDiffusionXl => "stable_diffusion_xl",
    };
    catalog_model(id).expect("every local image model is in the catalog")
  }

  pub fn display_name(&self) -> &'static str {
    self.catalog().display_name
  }

  pub fn checkpoint_file(&self) -> &'static str {
    self.catalog().primary_file().file_name
  }

  pub fn steps(&self) -> u32 {
    match self {
      Self::StableDiffusion15 => 20,
      Self::StableDiffusionXl => 25,
    }
  }

  pub fn cfg(&self) -> f32 {
    7.0
  }

  /// The size every picture is made at: each model's native square. These are the only sizes that
  /// have been run for real; wide and tall sizes were never tried, so they are not offered.
  pub fn dimensions(&self) -> (u32, u32) {
    match self {
      Self::StableDiffusion15 => (512, 512),
      Self::StableDiffusionXl => (1024, 1024),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  mod models {
    use super::*;

    #[test]
    fn only_the_local_models_map() {
      assert_eq!(LocalImageModel::from_tauri(TauriImageModel::StableDiffusion15), Some(LocalImageModel::StableDiffusion15));
      assert_eq!(LocalImageModel::from_tauri(TauriImageModel::StableDiffusionXl), Some(LocalImageModel::StableDiffusionXl));
      assert_eq!(LocalImageModel::from_tauri(TauriImageModel::NanoBananaPro), None);
    }

    #[test]
    fn each_model_points_at_its_own_checkpoint() {
      assert_eq!(LocalImageModel::StableDiffusion15.checkpoint_file(), "v1-5-pruned-emaonly-fp16.safetensors");
      assert_eq!(LocalImageModel::StableDiffusionXl.checkpoint_file(), "sd_xl_base_1.0.safetensors");
      assert_eq!(LocalImageModel::StableDiffusionXl.display_name(), "Stable Diffusion XL");
    }
  }

  mod verified_sizes {
    use super::*;

    #[test]
    fn pictures_are_always_each_models_native_square() {
      assert_eq!(LocalImageModel::StableDiffusion15.dimensions(), (512, 512));
      assert_eq!(LocalImageModel::StableDiffusionXl.dimensions(), (1024, 1024));
    }

    #[test]
    fn sizes_are_multiples_of_eight() {
      for model in [LocalImageModel::StableDiffusion15, LocalImageModel::StableDiffusionXl] {
        let (w, h) = model.dimensions();
        assert_eq!((w % 8, h % 8), (0, 0), "{:?}", model);
        assert_eq!(w, h, "{:?} is square", model);
      }
    }
  }

  #[test]
  fn steps_are_positive() {
    assert!(LocalImageModel::StableDiffusion15.steps() > 0);
    assert!(LocalImageModel::StableDiffusionXl.steps() >= LocalImageModel::StableDiffusion15.steps());
  }
}
