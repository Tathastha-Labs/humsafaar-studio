use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use crate::core::local_models::voiceover_state::{VoiceoverInstall, VoiceoverState};
use comfyui_manager::manager::ComfyUiManager;
use comfyui_manager::tts::{is_installed, Voice, MAX_SPEED, MAX_TEXT_CHARS, MIN_SPEED, TTS_DOWNLOAD_BYTES, VOICES};
use serde_derive::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct VoiceoverGetStatusSuccessResponse {
  pub installed: bool,
  pub install: VoiceoverInstall,
  pub voices: Vec<Voice>,
  pub download_bytes: u64,
  pub max_text_chars: usize,
  pub min_speed: f32,
  pub max_speed: f32,
}

impl SerializeMarker for VoiceoverGetStatusSuccessResponse {}

/// Whether voiceover is ready, how its install is going, and which voices there are.
#[tauri::command]
pub async fn voiceover_get_status_command(
  manager: State<'_, ComfyUiManager>,
  state: State<'_, VoiceoverState>,
) -> ResponseOrErrorMessage<VoiceoverGetStatusSuccessResponse> {
  Ok(VoiceoverGetStatusSuccessResponse {
    installed: is_installed(manager.layout()),
    install: state.install(),
    voices: VOICES.to_vec(),
    download_bytes: TTS_DOWNLOAD_BYTES,
    max_text_chars: MAX_TEXT_CHARS,
    min_speed: MIN_SPEED,
    max_speed: MAX_SPEED,
  }.into())
}
