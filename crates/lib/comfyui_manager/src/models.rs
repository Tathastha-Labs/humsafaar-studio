use crate::download::{download_verified, Progress};
use crate::error::{io_error, ManagerResult};
use crate::layout::RuntimeLayout;
use crate::suitability::{CpuSupport, Requirements};
use std::path::PathBuf;

/// One file of a model, and the ComfyUI models folder it belongs in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelFile {
  /// A folder inside the models directory, eg. "checkpoints".
  pub folder: &'static str,
  pub file_name: &'static str,
  pub url: &'static str,
  pub sha256: &'static str,
  pub size_bytes: u64,
}

/// A model the app offers to download. Some models are several files that work together.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatalogModel {
  /// Stable identifier used by the UI and settings.
  pub id: &'static str,
  pub display_name: &'static str,
  pub license: &'static str,
  /// Never empty. The first file is the main one.
  pub files: &'static [ModelFile],
  pub requirements: Requirements,
}

impl CatalogModel {
  pub fn primary_file(&self) -> &ModelFile {
    &self.files[0]
  }

  pub fn total_size_bytes(&self) -> u64 {
    self.files.iter().map(|file| file.size_bytes).sum()
  }
}

/// Measured or estimated; see `suitability`. Wan's figures come from a real run on a 24 GB Mac.
pub const SD_1_5_NEEDS: Requirements = Requirements {
  nvidia_good_gb: 6, nvidia_workable_gb: 4,
  apple_good_gb: 8, apple_workable_gb: 8,
  system_ram_gb: 8, cpu: CpuSupport::Slow,
};

pub const SDXL_NEEDS: Requirements = Requirements {
  nvidia_good_gb: 10, nvidia_workable_gb: 6,
  apple_good_gb: 16, apple_workable_gb: 16,
  system_ram_gb: 16, cpu: CpuSupport::VerySlow,
};

pub const WAN_5B_NEEDS: Requirements = Requirements {
  nvidia_good_gb: 24, nvidia_workable_gb: 16,
  apple_good_gb: 32, apple_workable_gb: 24,
  system_ram_gb: 32, cpu: CpuSupport::Impractical,
};

/// Apple numbers come from one real run (30s song in 59s on a 24 GB M-series Mac, swap peaked at 3 GB, so 16 GB is tight
/// but plausible). NVIDIA numbers are still estimates from model size.
pub const ACE_STEP_NEEDS: Requirements = Requirements {
  nvidia_good_gb: 12, nvidia_workable_gb: 8,
  apple_good_gb: 16, apple_workable_gb: 16,
  system_ram_gb: 16, cpu: CpuSupport::VerySlow,
};

/// Stable Diffusion 1.5, half precision. Small and fast, which makes it the starter model.
pub const STABLE_DIFFUSION_1_5: CatalogModel = CatalogModel {
  id: "stable_diffusion_1p5",
  display_name: "Stable Diffusion 1.5",
  license: "CreativeML Open RAIL-M",
  files: &[ModelFile {
    folder: "checkpoints",
    file_name: "v1-5-pruned-emaonly-fp16.safetensors",
    url: "https://huggingface.co/Comfy-Org/stable-diffusion-v1-5-archive/resolve/main/v1-5-pruned-emaonly-fp16.safetensors",
    sha256: "e9476a13728cd75d8279f6ec8bad753a66a1957ca375a1464dc63b37db6e3916",
    size_bytes: 2_132_696_762,
  }],
  requirements: SD_1_5_NEEDS,
};

/// Stable Diffusion XL base 1.0. Much sharper than 1.5 at 1024 pixels, and still runs on a laptop.
pub const STABLE_DIFFUSION_XL_BASE: CatalogModel = CatalogModel {
  id: "stable_diffusion_xl",
  display_name: "Stable Diffusion XL",
  license: "CreativeML Open RAIL++-M",
  files: &[ModelFile {
    folder: "checkpoints",
    file_name: "sd_xl_base_1.0.safetensors",
    url: "https://huggingface.co/stabilityai/stable-diffusion-xl-base-1.0/resolve/main/sd_xl_base_1.0.safetensors",
    sha256: "31e35c80fc4829d14f90153f4c74cd59c90b779f6afe05a74cd6120b893f7e5b",
    size_bytes: 6_938_078_334,
  }],
  requirements: SDXL_NEEDS,
};

/// Wan 2.2 "TI2V" 5B: text-to-video and image-to-video from Alibaba. Apache 2.0, free for
/// commercial use. Three files that work together: the video model, its VAE and a text encoder.
pub const WAN_2_2_VIDEO_5B: CatalogModel = CatalogModel {
  id: "wan_2p2_ti2v_5b",
  display_name: "Wan 2.2 Video (5B)",
  license: "Apache 2.0",
  files: &[
    ModelFile {
      folder: "diffusion_models",
      file_name: "wan2.2_ti2v_5B_fp16.safetensors",
      url: "https://huggingface.co/Comfy-Org/Wan_2.2_ComfyUI_Repackaged/resolve/main/split_files/diffusion_models/wan2.2_ti2v_5B_fp16.safetensors",
      sha256: "456f901338bd9eadbded3828b819109a9b68e8a525ca5cf8d0049a69fcfeca1e",
      size_bytes: 9_999_658_848,
    },
    ModelFile {
      folder: "vae",
      file_name: "wan2.2_vae.safetensors",
      url: "https://huggingface.co/Comfy-Org/Wan_2.2_ComfyUI_Repackaged/resolve/main/split_files/vae/wan2.2_vae.safetensors",
      sha256: "e40321bd36b9709991dae2530eb4ac303dd168276980d3e9bc4b6e2b75fed156",
      size_bytes: 1_409_400_960,
    },
    ModelFile {
      folder: "text_encoders",
      file_name: "umt5_xxl_fp8_e4m3fn_scaled.safetensors",
      url: "https://huggingface.co/Comfy-Org/Wan_2.2_ComfyUI_Repackaged/resolve/main/split_files/text_encoders/umt5_xxl_fp8_e4m3fn_scaled.safetensors",
      sha256: "c3355d30191f1f066b26d93fba017ae9809dce6c627dda5f6a66eaa651204f68",
      size_bytes: 6_735_906_897,
    },
  ],
  requirements: WAN_5B_NEEDS,
};

/// ACE-Step 1.5 "turbo": music and songs from text, in a few sampling steps. One file holds the
/// music model, its text encoder and its decoder. MIT license, free for commercial use.
pub const ACE_STEP_1_5_TURBO: CatalogModel = CatalogModel {
  id: "ace_step_1p5_turbo",
  display_name: "ACE-Step 1.5 Music",
  license: "MIT",
  files: &[ModelFile {
    folder: "checkpoints",
    file_name: "ace_step_1.5_turbo_aio.safetensors",
    url: "https://huggingface.co/Comfy-Org/ace_step_1.5_ComfyUI_files/resolve/main/checkpoints/ace_step_1.5_turbo_aio.safetensors",
    sha256: "67b0f43aa5c51c840bd0228e6a935d8ff416ec87e5df2fc0637da17a561252bc",
    size_bytes: 10_025_478_736,
  }],
  requirements: ACE_STEP_NEEDS,
};

pub const CATALOG: [CatalogModel; 4] = [
  STABLE_DIFFUSION_1_5,
  STABLE_DIFFUSION_XL_BASE,
  WAN_2_2_VIDEO_5B,
  ACE_STEP_1_5_TURBO,
];

pub fn catalog_model(id: &str) -> Option<&'static CatalogModel> {
  CATALOG.iter().find(|model| model.id == id)
}

const CHECKPOINT_EXTENSIONS: [&str; 4] = ["safetensors", "ckpt", "pt", "pth"];

pub fn file_path(layout: &RuntimeLayout, file: &ModelFile) -> PathBuf {
  layout.models_dir().join(file.folder).join(file.file_name)
}

/// Present means a file of the exact expected size. The checksum was checked when it downloaded.
pub fn is_file_present(layout: &RuntimeLayout, file: &ModelFile) -> bool {
  std::fs::metadata(file_path(layout, file)).is_ok_and(|m| m.is_file() && m.len() == file.size_bytes)
}

pub fn is_model_installed(layout: &RuntimeLayout, model: &CatalogModel) -> bool {
  model.files.iter().all(|file| is_file_present(layout, file))
}

/// Download whatever a model is missing, one file after another, reporting progress across all
/// of them. Files already present are skipped.
pub async fn ensure_model(
  layout: &RuntimeLayout,
  model: &CatalogModel,
  on_progress: &mut (dyn FnMut(Progress) + Send),
) -> ManagerResult<()> {
  let total = model.total_size_bytes();
  let mut finished_before: u64 = 0;

  for file in model.files {
    if !is_file_present(layout, file) {
      let offset = finished_before;
      download_verified(file.url, &file_path(layout, file), Some(file.sha256), &mut |progress: Progress| {
        on_progress(Progress { downloaded: offset + progress.downloaded, total: Some(total) });
      }).await?;
    }
    finished_before += file.size_bytes;
  }
  Ok(())
}

/// Delete every file of a model. Files that are already gone are fine.
pub fn remove_model(layout: &RuntimeLayout, model: &CatalogModel) -> ManagerResult<()> {
  for file in model.files {
    let path = file_path(layout, file);
    match std::fs::remove_file(&path) {
      Ok(()) => {}
      Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
      Err(error) => return Err(io_error(path, error)),
    }
  }
  Ok(())
}

/// File names of every checkpoint in the models folder.
pub fn list_checkpoints(layout: &RuntimeLayout) -> ManagerResult<Vec<String>> {
  let dir = layout.checkpoints_dir();
  if !dir.is_dir() {
    return Ok(Vec::new());
  }
  let mut names = Vec::new();
  for entry in std::fs::read_dir(&dir).map_err(|e| io_error(&dir, e))? {
    let entry = entry.map_err(|e| io_error(&dir, e))?;
    let path = entry.path();
    let is_model = path.extension().and_then(|e| e.to_str())
      .is_some_and(|e| CHECKPOINT_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()));
    if path.is_file() && is_model {
      names.push(entry.file_name().to_string_lossy().into_owned());
    }
  }
  names.sort();
  Ok(names)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::{Arc, Mutex};
  use tempfile::tempdir;
  use tokio::io::{AsyncReadExt, AsyncWriteExt};
  use tokio::net::TcpListener;

  const FIRST: &[u8] = b"first file";
  const SECOND: &[u8] = b"the second, longer file";

  mod catalog {
    use super::*;

    #[test]
    fn every_model_is_fully_described_and_unique() {
      let mut ids = std::collections::HashSet::new();
      let mut paths = std::collections::HashSet::new();
      for model in CATALOG {
        assert!(ids.insert(model.id), "duplicate id {}", model.id);
        assert!(!model.files.is_empty(), "{}", model.id);
        assert!(!model.license.is_empty());
        for file in model.files {
          assert!(paths.insert((file.folder, file.file_name)), "duplicate file {}", file.file_name);
          assert_eq!(file.sha256.len(), 64, "{}", file.file_name);
          assert!(file.sha256.chars().all(|c| c.is_ascii_hexdigit()));
          assert!(file.url.starts_with("https://"), "{}", file.file_name);
          assert!(file.size_bytes > 0);
        }
      }
    }

    #[test]
    fn models_are_found_by_id() {
      assert_eq!(catalog_model("stable_diffusion_xl").unwrap(), &STABLE_DIFFUSION_XL_BASE);
      assert!(catalog_model("nope").is_none());
    }

    #[test]
    fn wan_is_a_three_file_bundle_in_the_folders_comfyui_expects() {
      let folders: Vec<&str> = WAN_2_2_VIDEO_5B.files.iter().map(|f| f.folder).collect();
      assert_eq!(folders, ["diffusion_models", "vae", "text_encoders"]);
      assert_eq!(WAN_2_2_VIDEO_5B.total_size_bytes(), 18_144_966_705);
      assert_eq!(WAN_2_2_VIDEO_5B.license, "Apache 2.0");
    }

    #[test]
    fn size_is_the_sum_of_the_files() {
      let model = served_model("http://x/", &[("a", FIRST), ("b", SECOND)]);
      assert_eq!(model.total_size_bytes(), (FIRST.len() + SECOND.len()) as u64);
      assert_eq!(model.primary_file().file_name, "a.bin");
    }
  }

  mod installed {
    use super::*;

    #[test]
    fn a_file_must_have_the_exact_size() {
      let dir = tempdir().unwrap();
      let layout = RuntimeLayout::new(dir.path());
      let model = served_model("http://x/", &[("a", FIRST)]);
      let file = model.primary_file();
      assert!(!is_file_present(&layout, file));

      std::fs::create_dir_all(file_path(&layout, file).parent().unwrap()).unwrap();
      std::fs::write(file_path(&layout, file), &FIRST[..3]).unwrap();
      assert!(!is_file_present(&layout, file), "a truncated file is not present");

      std::fs::write(file_path(&layout, file), FIRST).unwrap();
      assert!(is_file_present(&layout, file));
    }

    #[test]
    fn a_bundle_needs_every_file() {
      let dir = tempdir().unwrap();
      let layout = RuntimeLayout::new(dir.path());
      let model = served_model("http://x/", &[("a", FIRST), ("b", SECOND)]);

      write_file(&layout, &model.files[0], FIRST);
      assert!(!is_model_installed(&layout, &model), "one of two files");

      write_file(&layout, &model.files[1], SECOND);
      assert!(is_model_installed(&layout, &model));
    }

    #[test]
    fn files_land_in_their_own_folder() {
      let layout = RuntimeLayout::new("/r");
      let file = ModelFile { folder: "vae", file_name: "x.safetensors", url: "", sha256: "", size_bytes: 1 };
      assert_eq!(file_path(&layout, &file), layout.models_dir().join("vae").join("x.safetensors"));
    }
  }

  mod downloads {
    use super::*;

    #[tokio::test]
    async fn downloads_every_file_and_reports_combined_progress() {
      let url = serve_files(&[("a", FIRST), ("b", SECOND)]).await;
      let dir = tempdir().unwrap();
      let layout = RuntimeLayout::new(dir.path());
      let model = served_model(&url, &[("a", FIRST), ("b", SECOND)]);

      let updates = Arc::new(Mutex::new(Vec::new()));
      let sink = updates.clone();
      ensure_model(&layout, &model, &mut move |p| sink.lock().unwrap().push(p)).await.unwrap();

      assert!(is_model_installed(&layout, &model));
      let updates = updates.lock().unwrap();
      let total = model.total_size_bytes();
      assert!(updates.iter().all(|p| p.total == Some(total)), "one total across files");
      assert_eq!(updates.last().unwrap().downloaded, total);
      assert!(updates.windows(2).all(|w| w[1].downloaded >= w[0].downloaded), "never goes backwards");
    }

    #[tokio::test]
    async fn files_already_present_are_not_fetched_again() {
      let url = serve_files(&[("b", SECOND)]).await; // "a" is not served
      let dir = tempdir().unwrap();
      let layout = RuntimeLayout::new(dir.path());
      let model = served_model(&url, &[("a", FIRST), ("b", SECOND)]);
      write_file(&layout, &model.files[0], FIRST);

      ensure_model(&layout, &model, &mut |_| {}).await.unwrap();

      assert!(is_model_installed(&layout, &model));
    }

    #[tokio::test]
    async fn a_bundle_is_never_reported_installed_when_one_file_fails() {
      let url = serve_files(&[("a", FIRST)]).await; // "b" returns 404
      let dir = tempdir().unwrap();
      let layout = RuntimeLayout::new(dir.path());
      let model = served_model(&url, &[("a", FIRST), ("b", SECOND)]);

      assert!(ensure_model(&layout, &model, &mut |_| {}).await.is_err());
      assert!(!is_model_installed(&layout, &model));
    }
  }

  mod removal {
    use super::*;

    #[test]
    fn removes_every_file_and_tolerates_missing_ones() {
      let dir = tempdir().unwrap();
      let layout = RuntimeLayout::new(dir.path());
      let model = served_model("http://x/", &[("a", FIRST), ("b", SECOND)]);
      write_file(&layout, &model.files[0], FIRST); // "b" was never there

      remove_model(&layout, &model).unwrap();

      assert!(!file_path(&layout, &model.files[0]).exists());
      remove_model(&layout, &model).unwrap();
    }
  }

  #[test]
  fn lists_only_model_files() {
    let dir = tempdir().unwrap();
    let layout = RuntimeLayout::new(dir.path());
    assert!(list_checkpoints(&layout).unwrap().is_empty());

    std::fs::create_dir_all(layout.checkpoints_dir()).unwrap();
    for name in ["b.safetensors", "a.CKPT", "notes.txt", "c.safetensors.part"] {
      std::fs::write(layout.checkpoints_dir().join(name), b"x").unwrap();
    }
    assert_eq!(list_checkpoints(&layout).unwrap(), vec!["a.CKPT", "b.safetensors"]);
  }

  // ── helpers ──

  /// A model whose files are named `<name>.bin` and served at `<base>/<name>`.
  pub(crate) fn served_model(base: &str, files: &[(&str, &'static [u8])]) -> CatalogModel {
    use sha2::{Digest, Sha256};
    let specs: Vec<ModelFile> = files.iter().map(|(name, body)| ModelFile {
      folder: "checkpoints",
      file_name: Box::leak(format!("{}.bin", name).into_boxed_str()),
      url: Box::leak(format!("{}{}", base, name).into_boxed_str()),
      sha256: Box::leak(Sha256::digest(body).iter().map(|b| format!("{:02x}", b)).collect::<String>().into_boxed_str()),
      size_bytes: body.len() as u64,
    }).collect();
    CatalogModel {
      id: "test_model",
      display_name: "Test model",
      license: "test",
      files: Box::leak(specs.into_boxed_slice()),
      requirements: Requirements::ANY,
    }
  }

  fn write_file(layout: &RuntimeLayout, file: &ModelFile, body: &[u8]) {
    let path = file_path(layout, file);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
  }

  /// Serves `/<name>` for each given file and 404 for anything else. Returns the base URL ending in `/`.
  async fn serve_files(files: &[(&str, &'static [u8])]) -> String {
    let routes: Vec<(String, &'static [u8])> = files.iter().map(|(n, b)| (format!("/{}", n), *b)).collect();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap());
    let routes = Arc::new(routes);
    tokio::spawn(async move {
      loop {
        let Ok((mut socket, _)) = listener.accept().await else { return };
        let routes = routes.clone();
        tokio::spawn(async move {
          let mut buffer = vec![0u8; 4096];
          let read = socket.read(&mut buffer).await.unwrap_or(0);
          let request = String::from_utf8_lossy(&buffer[..read]).to_string();
          let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();
          let (status, body) = match routes.iter().find(|(p, _)| *p == path) {
            Some((_, body)) => (200, *body),
            None => (404, &b""[..]),
          };
          let head = format!("HTTP/1.1 {} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", status, body.len());
          let _ = socket.write_all(head.as_bytes()).await;
          let _ = socket.write_all(body).await;
          let _ = socket.shutdown().await;
        });
      }
    });
    base
  }
}
