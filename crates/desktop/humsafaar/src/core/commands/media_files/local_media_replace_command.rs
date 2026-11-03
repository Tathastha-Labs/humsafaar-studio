use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use crate::core::local_media::local_media_store::LocalMediaStore;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use log::error;
use serde_derive::{Deserialize, Serialize};
use tauri::State;
use tokens::tokens::media_files::MediaFileToken;

#[derive(Deserialize)]
pub struct LocalMediaReplaceRequest {
  /// REQUIRED. The file to overwrite; it must already be in the store.
  pub media_file_token: MediaFileToken,
  /// REQUIRED. The new contents, standard base64.
  pub base64_data: String,
}

#[derive(Serialize)]
pub struct LocalMediaReplaceSuccessResponse {}

impl SerializeMarker for LocalMediaReplaceSuccessResponse {}

/// Save new contents over a file that is already in the store, keeping its token.
#[tauri::command]
pub async fn local_media_replace_command(
  request: LocalMediaReplaceRequest,
  local_media: State<'_, LocalMediaStore>,
) -> ResponseOrErrorMessage<LocalMediaReplaceSuccessResponse> {
  let bytes = BASE64_STANDARD.decode(request.base64_data.as_bytes())
    .map_err(|_| "invalid base64 data".to_string())?;

  local_media.replace_bytes(&request.media_file_token, &bytes).map_err(|err| {
    error!("local_media_replace_command failed: {:?}", err);
    "could not save file".to_string()
  })?;

  Ok(LocalMediaReplaceSuccessResponse {}.into())
}
