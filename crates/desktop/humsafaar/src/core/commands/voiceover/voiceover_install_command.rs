use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use crate::core::local_models::voiceover_state::VoiceoverState;
use comfyui_manager::manager::ComfyUiManager;
use comfyui_manager::platform::Platform;
use comfyui_manager::tts::{install, is_installed};
use log::{error, info};
use serde_derive::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct VoiceoverInstallSuccessResponse {}

impl SerializeMarker for VoiceoverInstallSuccessResponse {}

/// Download the voice runtime and model. Returns at once; follow it with `voiceover_get_status_command`.
#[tauri::command]
pub async fn voiceover_install_command(
  manager: State<'_, ComfyUiManager>,
  state: State<'_, VoiceoverState>,
) -> ResponseOrErrorMessage<VoiceoverInstallSuccessResponse> {
  if state.is_installing() || is_installed(manager.layout()) {
    return Ok(VoiceoverInstallSuccessResponse {}.into());
  }

  info!("voiceover_install_command: starting");
  let layout = manager.layout().clone();
  let state = state.inner().clone();
  state.set_idle();
  tauri::async_runtime::spawn(async move {
    let progress = state.clone();
    let result = install(&layout, Platform::current(), &mut |p| progress.set_progress(p)).await;
    match result {
      Ok(()) => state.set_idle(),
      Err(err) => {
        error!("Voiceover install failed: {}", err);
        state.set_failed(err.to_string());
      }
    }
  });

  Ok(VoiceoverInstallSuccessResponse {}.into())
}
