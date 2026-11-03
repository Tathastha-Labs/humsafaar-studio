use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use crate::core::local_media::local_media_store::LocalMediaStore;
use comfyui_manager::manager::ComfyUiManager;
use comfyui_manager::tts::{check_request, synthesize};
use log::{error, info};
use serde_derive::{Deserialize, Serialize};
use tauri::State;
use uuid_utils::uuid::generate_random_uuid;

#[derive(Deserialize)]
pub struct VoiceoverGenerateRequest {
  pub text: String,
  pub voice_id: String,
  pub speed: Option<f32>,
}

#[derive(Serialize)]
pub struct VoiceoverGenerateSuccessResponse {
  pub media_token: String,
  /// A URL the webview can load the audio from.
  pub url: String,
  pub seconds: f32,
}

impl SerializeMarker for VoiceoverGenerateSuccessResponse {}

/// Speak the script and save it in the local media store. Takes a few seconds at most.
#[tauri::command]
pub async fn voiceover_generate_command(
  request: VoiceoverGenerateRequest,
  manager: State<'_, ComfyUiManager>,
  store: State<'_, LocalMediaStore>,
) -> ResponseOrErrorMessage<VoiceoverGenerateSuccessResponse> {
  let checked = check_request(&request.text, &request.voice_id, request.speed.unwrap_or(1.0))
    .map_err(|err| err.to_string())?;

  let scratch = manager.layout().downloads_dir().join(format!("voiceover-{}.wav", generate_random_uuid()));
  info!("voiceover_generate_command: {} chars with {}", checked.text.chars().count(), checked.voice.id);

  let spoken = synthesize(manager.layout(), &checked, &scratch).await.map_err(|err| {
    error!("Voiceover failed: {}", err);
    err.to_string()
  })?;

  let saved = store.put_file(&scratch);
  let _ = std::fs::remove_file(&scratch);
  let token = saved.map_err(|err| format!("Could not save the voiceover: {}", err))?;
  let path = store.path_for_token(&token).ok_or("The saved voiceover went missing.")?;

  Ok(VoiceoverGenerateSuccessResponse {
    media_token: token.to_string(),
    url: LocalMediaStore::asset_url_for_path(&path),
    seconds: spoken.seconds,
  }.into())
}
