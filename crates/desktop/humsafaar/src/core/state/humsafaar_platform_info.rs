use crate::core::state::os_platform::OsPlatform;
use crate::version::HUMSAFAAR_VERSION;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct HumsafaarPlatformInfo {
  pub humsafaar_version: String,
  
  pub os_platform: HumsafaarOs,
  pub os_version: String,
  
  pub build_timestamp: DateTime<Utc>,
  
  pub git_commit_id: Option<String>,
  pub git_commit_short_id: Option<String>,
  pub git_commit_timestamp: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Debug)]
pub enum HumsafaarOs {
  Windows,
  MacOS,
  Linux,
  Unknown,
}

impl HumsafaarPlatformInfo {
  
  pub fn get() -> Self {
    let build_timestamp = build_metadata::build_timestamp();
    let git_commit_id = build_metadata::git_commit_id().map(|s| s.to_string());
    let git_commit_short_id = build_metadata::git_commit_short_id().map(|s| s.to_string());
    let git_commit_timestamp = build_metadata::git_commit_timestamp();

    let os_info = os_info::get();
    
    let maybe_os_version = Some(os_info.version().to_string())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    
    Self {
      humsafaar_version: HUMSAFAAR_VERSION.to_string(),
      os_platform: match OsPlatform::maybe_get() {
        Some(OsPlatform::Linux) => HumsafaarOs::Linux,
        Some(OsPlatform::MacOs) => HumsafaarOs::MacOS,
        Some(OsPlatform::Windows) => HumsafaarOs::Windows,
        None => HumsafaarOs::Unknown,
      },
      os_version: maybe_os_version,
      build_timestamp,
      git_commit_id,
      git_commit_short_id,
      git_commit_timestamp,
    }
  }
}

impl HumsafaarOs {
  pub fn as_str(&self) -> &'static str {
    match self {
      HumsafaarOs::Windows => "windows",
      HumsafaarOs::MacOS => "macos",
      HumsafaarOs::Linux => "linux",
      HumsafaarOs::Unknown => "unknown",
    }
  }
}
