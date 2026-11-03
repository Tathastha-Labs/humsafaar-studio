use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ManagerError {
  #[error("{0}")]
  Unsupported(#[from] crate::platform::UnsupportedPlatform),

  #[error("download failed: {0}")]
  Download(#[from] reqwest::Error),

  #[error("download of {url} returned status {status}")]
  DownloadStatus { url: String, status: u16 },

  #[error("checksum mismatch for {file}: expected {expected}, got {actual}")]
  ChecksumMismatch { file: String, expected: String, actual: String },

  #[error("the archive contains an unsafe path: {0}")]
  UnsafeArchivePath(String),

  #[error("file error at {path}: {source}")]
  Io { path: PathBuf, #[source] source: std::io::Error },

  #[error("archive error: {0}")]
  Archive(String),

  #[error("{step} failed: {detail}")]
  Command { step: String, detail: String },

  #[error("the engine is not installed")]
  NotInstalled,

  #[error("the engine did not become ready: {0}")]
  NotReady(String),

  #[error("{0}")]
  Other(String),
}

pub type ManagerResult<T> = Result<T, ManagerError>;

pub(crate) fn io_error(path: impl Into<PathBuf>, source: std::io::Error) -> ManagerError {
  ManagerError::Io { path: path.into(), source }
}
