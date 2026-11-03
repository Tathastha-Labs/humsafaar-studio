use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use crate::core::local_media::local_media_store::LocalMediaStore;
use serde_derive::{Deserialize, Serialize};
use tauri::State;
use tokens::tokens::media_files::MediaFileToken;

#[derive(Deserialize)]
pub struct LocalMediaReadTextRequest {
  /// REQUIRED.
  pub media_file_token: MediaFileToken,
}

#[derive(Serialize)]
pub struct LocalMediaReadTextSuccessResponse {
  pub text: String,
}

impl SerializeMarker for LocalMediaReadTextSuccessResponse {}

/// A text file from the store, such as a saved scene.
#[tauri::command]
pub async fn local_media_read_text_command(
  request: LocalMediaReadTextRequest,
  local_media: State<'_, LocalMediaStore>,
) -> ResponseOrErrorMessage<LocalMediaReadTextSuccessResponse> {
  let text = local_media.read_text(&request.media_file_token)
    .map_err(|_| "that file could not be read".to_string())?;
  Ok(LocalMediaReadTextSuccessResponse { text }.into())
}
