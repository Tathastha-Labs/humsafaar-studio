use std::path::{Path, PathBuf};

const UV_EXECUTABLE: &str = if cfg!(windows) { "uv.exe" } else { "uv" };

/// Where everything the local engine needs lives, under a single root folder.
/// Deleting the root removes the engine, its models and its outputs.
///
/// The ComfyUI program (`app_dir`) is kept apart from the user's data (`data_dir`),
/// so the engine can be reinstalled or upgraded without touching models.
#[derive(Clone, Debug)]
pub struct RuntimeLayout {
  root: PathBuf,
}

impl RuntimeLayout {
  pub fn new<P: AsRef<Path>>(root: P) -> Self {
    Self { root: root.as_ref().to_path_buf() }
  }

  pub fn root(&self) -> &Path {
    &self.root
  }

  /// The `uv` executable that bootstraps Python and the packages.
  pub fn uv_executable(&self) -> PathBuf {
    self.root.join("tools").join(UV_EXECUTABLE)
  }

  /// Where `uv` keeps the Python builds it downloads.
  pub fn python_install_dir(&self) -> PathBuf {
    self.root.join("python")
  }

  pub fn uv_cache_dir(&self) -> PathBuf {
    self.root.join("cache")
  }

  pub fn venv_dir(&self) -> PathBuf {
    self.root.join("venv")
  }

  pub fn venv_python(&self) -> PathBuf {
    if cfg!(windows) {
      self.venv_dir().join("Scripts").join("python.exe")
    } else {
      self.venv_dir().join("bin").join("python")
    }
  }

  /// The ComfyUI source (`main.py`, `requirements.txt`).
  pub fn app_dir(&self) -> PathBuf {
    self.root.join("app")
  }

  pub fn main_script(&self) -> PathBuf {
    self.app_dir().join("main.py")
  }

  pub fn requirements_file(&self) -> PathBuf {
    self.app_dir().join("requirements.txt")
  }

  /// ComfyUI's models, input, output, temp and user folders.
  pub fn data_dir(&self) -> PathBuf {
    self.root.join("data")
  }

  pub fn models_dir(&self) -> PathBuf {
    self.data_dir().join("models")
  }

  pub fn checkpoints_dir(&self) -> PathBuf {
    self.models_dir().join("checkpoints")
  }

  /// Partial downloads, before they are verified and moved into place.
  pub fn downloads_dir(&self) -> PathBuf {
    self.root.join("downloads")
  }

  pub fn log_file(&self) -> PathBuf {
    self.root.join("logs").join("comfyui.log")
  }

  /// Records the running engine so a crashed app can clean it up next time.
  pub fn pid_file(&self) -> PathBuf {
    self.root.join("comfyui.pid")
  }

  /// Voiceover lives in its own folder and Python environment, apart from the image engine.
  pub fn tts_dir(&self) -> PathBuf {
    self.root.join("tts")
  }

  pub fn tts_venv_dir(&self) -> PathBuf {
    self.tts_dir().join("venv")
  }

  pub fn tts_python(&self) -> PathBuf {
    if cfg!(windows) {
      self.tts_venv_dir().join("Scripts").join("python.exe")
    } else {
      self.tts_venv_dir().join("bin").join("python")
    }
  }

  pub fn tts_models_dir(&self) -> PathBuf {
    self.tts_dir().join("models")
  }

  pub fn tts_script(&self) -> PathBuf {
    self.tts_dir().join("synth.py")
  }

  pub fn tts_manifest_file(&self) -> PathBuf {
    self.tts_dir().join("install.json")
  }

  /// What is installed, written once an install step completes.
  pub fn manifest_file(&self) -> PathBuf {
    self.root.join("install.json")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn everything_is_under_the_root() {
    let layout = RuntimeLayout::new("/data/comfyui");
    for path in [
      layout.uv_executable(), layout.python_install_dir(), layout.uv_cache_dir(),
      layout.venv_python(), layout.main_script(), layout.requirements_file(),
      layout.checkpoints_dir(), layout.downloads_dir(), layout.log_file(),
      layout.pid_file(), layout.manifest_file(), layout.tts_python(), layout.tts_models_dir(),
      layout.tts_script(), layout.tts_manifest_file(),
    ] {
      assert!(path.starts_with("/data/comfyui"), "{:?}", path);
    }
  }

  #[test]
  fn models_are_kept_apart_from_the_program() {
    let layout = RuntimeLayout::new("/data/comfyui");
    assert!(!layout.models_dir().starts_with(layout.app_dir()));
    assert_eq!(layout.checkpoints_dir(), layout.models_dir().join("checkpoints"));
  }

  #[cfg(unix)]
  #[test]
  fn unix_paths_use_bin() {
    let layout = RuntimeLayout::new("/r");
    assert_eq!(layout.venv_python(), PathBuf::from("/r/venv/bin/python"));
    assert_eq!(layout.uv_executable(), PathBuf::from("/r/tools/uv"));
  }
}
