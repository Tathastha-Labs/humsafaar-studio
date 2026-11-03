use crate::core::state::data_dir::trait_data_subdir::DataSubdir;
use std::path::{Path, PathBuf};

/// Locally stored media (generated results and imported inputs).
#[derive(Clone)]
pub struct AppMediaDir {
  path: PathBuf,
}

impl DataSubdir for AppMediaDir {
  const DIRECTORY_NAME: &'static str = "media";

  fn new_from<P: AsRef<Path>>(dir: P) -> Self {
    Self {
      path: dir.as_ref().to_path_buf(),
    }
  }

  fn path(&self) -> &Path {
    &self.path
  }
}
