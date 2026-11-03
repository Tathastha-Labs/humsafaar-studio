use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use crate::core::local_media::local_media_store::LocalMediaStore;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use log::{error, info};
use serde_derive::{Deserialize, Serialize};
use tauri::State;
use tokens::tokens::media_files::MediaFileToken;

#[derive(Deserialize)]
pub struct LocalMediaImportRequest {
  /// REQUIRED.
  /// The file contents, standard base64.
  pub base64_data: String,

  /// REQUIRED.
  /// File extension without the period, eg. "png".
  pub extension: String,
}

#[derive(Serialize)]
pub struct LocalMediaImportSuccessResponse {
  /// Use this wherever a media file token is expected (eg. as a generation input).
  pub media_file_token: MediaFileToken,

  /// A URL the webview can load directly.
  pub url: String,
}

impl SerializeMarker for LocalMediaImportSuccessResponse {}

/// Save a file from the frontend into the local media store.
#[tauri::command]
pub async fn local_media_import_command(
  request: LocalMediaImportRequest,
  local_media: State<'_, LocalMediaStore>,
) -> ResponseOrErrorMessage<LocalMediaImportSuccessResponse> {
  let bytes = BASE64_STANDARD.decode(request.base64_data.as_bytes())
    .map_err(|_| "invalid base64 data".to_string())?;

  let token = local_media.put_bytes(&bytes, &request.extension)
    .map_err(|err| {
      error!("local_media_import_command failed: {:?}", err);
      "could not save file".to_string()
    })?;

  let path = local_media.path_for_token(&token)
    .ok_or_else(|| "could not save file".to_string())?;

  info!("local_media_import_command saved {} ({} bytes)", token.as_str(), bytes.len());

  Ok(LocalMediaImportSuccessResponse {
    url: LocalMediaStore::asset_url_for_path(&path),
    media_file_token: token,
  }.into())
}
