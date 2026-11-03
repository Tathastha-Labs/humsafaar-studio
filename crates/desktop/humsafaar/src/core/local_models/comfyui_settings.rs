use std::env;

/// How to reach the local ComfyUI engine and which checkpoint to use.
///
/// By default the app installs and runs its own engine. Setting `HUMSAFAAR_COMFYUI_URL`
/// points it at a ComfyUI you run yourself instead, which the app then leaves alone.
/// `HUMSAFAAR_COMFYUI_CHECKPOINT` picks a checkpoint by file name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComfyUiSettings {
  /// `Some` means an external server that the app does not manage.
  pub external_url: Option<String>,

  /// `None` means "use the first checkpoint ComfyUI reports".
  pub checkpoint: Option<String>,
}

impl ComfyUiSettings {
  pub fn from_env() -> Self {
    Self::from_values(
      env::var("HUMSAFAAR_COMFYUI_URL").ok().as_deref(),
      env::var("HUMSAFAAR_COMFYUI_CHECKPOINT").ok().as_deref(),
    )
  }

  pub fn is_external(&self) -> bool {
    self.external_url.is_some()
  }

  fn from_values(url: Option<&str>, checkpoint: Option<&str>) -> Self {
    let non_empty = |v: &str| Some(v.trim()).filter(|v| !v.is_empty()).map(str::to_string);
    Self {
      external_url: url.and_then(non_empty),
      checkpoint: checkpoint.and_then(non_empty),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_engine_is_managed_by_default() {
    let s = ComfyUiSettings::from_values(None, None);
    assert!(!s.is_external());
    assert_eq!(s.checkpoint, None);
  }

  #[test]
  fn blank_values_are_ignored() {
    let s = ComfyUiSettings::from_values(Some("  "), Some(""));
    assert_eq!(s, ComfyUiSettings::from_values(None, None));
  }

  #[test]
  fn a_url_selects_an_external_server() {
    let s = ComfyUiSettings::from_values(Some("http://10.0.0.2:9000"), Some("sdxl.safetensors"));
    assert!(s.is_external());
    assert_eq!(s.external_url.as_deref(), Some("http://10.0.0.2:9000"));
    assert_eq!(s.checkpoint.as_deref(), Some("sdxl.safetensors"));
  }
}
