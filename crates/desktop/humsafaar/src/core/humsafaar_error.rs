use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

#[derive(Debug)]
pub enum HumsafaarError {
  AnyhowError(anyhow::Error),
  DecodeError(base64::DecodeError),
  IoError(std::io::Error),
  ReqwestError(reqwest::Error),
  // Service errors
  // Lock errors
  RwLockReadError,
  RwLockWriteError,
  MutexLockError,
  // Download errors
  BadDownloadFilename { path: PathBuf },
  CannotDownloadFilePathAlreadyExists { path: PathBuf },
}

impl Error for HumsafaarError {}

impl Display for HumsafaarError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::AnyhowError(e) => write!(f, "AnyhowError: {:?}", e),
      Self::DecodeError(e) => write!(f, "DecodeError: {:?}", e),
      Self::IoError(e) => write!(f, "IoError: {:?}", e),
      Self::ReqwestError(e) => write!(f, "ReqwestError: {:?}", e),
      Self::RwLockReadError => write!(f, "RwLockReadError"),
      Self::RwLockWriteError => write!(f, "RwLockWriteError"),
      Self::MutexLockError => write!(f, "MutexLockError"),
      Self::BadDownloadFilename { path } => write!(f, "BadDownloadFilename: {:?}", path),
      Self::CannotDownloadFilePathAlreadyExists { path } => write!(f, "CannotDownloadFilePathAlreadyExists {:?}", path),
    }
  }
}

impl From<anyhow::Error> for HumsafaarError {
  fn from(value: anyhow::Error) -> Self {
    Self::AnyhowError(value)
  }
}

impl From<base64::DecodeError> for HumsafaarError {
  fn from(value: base64::DecodeError) -> Self {
    Self::DecodeError(value)
  }
}

impl From<std::io::Error> for HumsafaarError {
  fn from(value: std::io::Error) -> Self {
    Self::IoError(value)
  }
}

impl From<reqwest::Error> for HumsafaarError {
  fn from(value: reqwest::Error) -> Self {
    Self::ReqwestError(value)
  }
}

