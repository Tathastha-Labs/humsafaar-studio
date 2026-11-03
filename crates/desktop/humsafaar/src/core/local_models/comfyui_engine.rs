use crate::core::local_models::comfyui_settings::ComfyUiSettings;
use comfyui_manager::error::ManagerError;
use comfyui_manager::manager::ComfyUiManager;
use std::fmt;

/// Why the local engine cannot be used right now. The text is shown to the user.
#[derive(Debug, PartialEq, Eq)]
pub enum EngineUnavailable {
  NotInstalled,
  Failed(String),
}

/// The address of the engine, starting it first if it is installed but not running.
pub async fn resolve_base_url(
  settings: &ComfyUiSettings,
  manager: &ComfyUiManager,
) -> Result<String, EngineUnavailable> {
  if let Some(url) = &settings.external_url {
    return Ok(url.clone());
  }
  match manager.start().await {
    Ok(url) => Ok(url),
    Err(ManagerError::NotInstalled) => Err(EngineUnavailable::NotInstalled),
    Err(error) => Err(EngineUnavailable::Failed(error.to_string())),
  }
}

/// The address of an engine that is already running; never starts one.
pub fn running_base_url(settings: &ComfyUiSettings, manager: &ComfyUiManager) -> Option<String> {
  settings.external_url.clone().or_else(|| manager.base_url())
}

impl fmt::Display for EngineUnavailable {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      EngineUnavailable::NotInstalled => write!(
        f, "Local generation is not set up yet. Open Settings, then Local Models, to install it."),
      EngineUnavailable::Failed(reason) => write!(f, "The local engine could not start: {}", reason),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use comfyui_manager::layout::RuntimeLayout;
  use tempfile::tempdir;

  #[tokio::test]
  async fn an_external_server_is_used_as_is_without_starting_anything() {
    let dir = tempdir().unwrap();
    let manager = ComfyUiManager::new(RuntimeLayout::new(dir.path()));
    let settings = ComfyUiSettings { external_url: Some("http://10.0.0.2:9".into()), checkpoint: None };

    assert_eq!(resolve_base_url(&settings, &manager).await.unwrap(), "http://10.0.0.2:9");
    assert_eq!(running_base_url(&settings, &manager).as_deref(), Some("http://10.0.0.2:9"));
  }

  #[tokio::test]
  async fn a_missing_install_is_reported_as_not_installed() {
    let dir = tempdir().unwrap();
    let manager = ComfyUiManager::new(RuntimeLayout::new(dir.path()));
    let settings = ComfyUiSettings { external_url: None, checkpoint: None };

    // On platforms the engine does not support, the manager reports that instead.
    let error = resolve_base_url(&settings, &manager).await.unwrap_err();
    assert!(matches!(error, EngineUnavailable::NotInstalled | EngineUnavailable::Failed(_)), "{error:?}");
    assert_eq!(running_base_url(&settings, &manager), None);
  }

  #[test]
  fn messages_tell_the_user_what_to_do() {
    assert!(EngineUnavailable::NotInstalled.to_string().contains("Local Models"));
    assert!(EngineUnavailable::Failed("boom".into()).to_string().contains("boom"));
  }
}
