use comfyui_manager::tts::TtsProgress;
use serde_derive::Serialize;
use std::sync::{Arc, Mutex};

/// Where installing voiceover stands. Whether it is installed at all is read from disk.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum VoiceoverInstall {
  Idle,
  Installing { label: String, detail: Option<String>, fraction: Option<f32> },
  Failed { message: String },
}

#[derive(Clone)]
pub struct VoiceoverState {
  install: Arc<Mutex<VoiceoverInstall>>,
}

impl VoiceoverState {
  pub fn new() -> Self {
    Self { install: Arc::new(Mutex::new(VoiceoverInstall::Idle)) }
  }

  pub fn install(&self) -> VoiceoverInstall {
    self.install.lock().map(|state| state.clone()).unwrap_or(VoiceoverInstall::Idle)
  }

  pub fn is_installing(&self) -> bool {
    matches!(self.install(), VoiceoverInstall::Installing { .. })
  }

  pub fn set_progress(&self, progress: TtsProgress) {
    self.set(VoiceoverInstall::Installing {
      label: progress.label.to_string(),
      detail: progress.detail,
      fraction: progress.fraction,
    });
  }

  pub fn set_failed(&self, message: String) {
    self.set(VoiceoverInstall::Failed { message });
  }

  pub fn set_idle(&self) {
    self.set(VoiceoverInstall::Idle);
  }

  fn set(&self, next: VoiceoverInstall) {
    if let Ok(mut state) = self.install.lock() {
      *state = next;
    }
  }
}

impl Default for VoiceoverState {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn starts_idle_and_follows_progress() {
    let state = VoiceoverState::new();
    assert!(!state.is_installing());

    state.set_progress(TtsProgress { label: "Downloading the voice model", detail: None, fraction: Some(0.5) });
    assert!(state.is_installing());

    state.set_failed("no network".to_string());
    assert!(!state.is_installing());
    assert_eq!(state.install(), VoiceoverInstall::Failed { message: "no network".to_string() });

    state.set_idle();
    assert_eq!(state.install(), VoiceoverInstall::Idle);
  }

  #[test]
  fn clones_share_one_state() {
    let state = VoiceoverState::new();
    let other = state.clone();
    other.set_failed("x".to_string());
    assert!(matches!(state.install(), VoiceoverInstall::Failed { .. }));
  }
}
