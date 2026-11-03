use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use crate::core::local_media::local_media_store::LocalMediaStore;
use serde_derive::{Deserialize, Serialize};
use tauri::State;
use tokens::tokens::media_files::MediaFileToken;

#[derive(Deserialize)]
pub struct LocalMediaGetUrlRequest {
  /// REQUIRED.
  pub media_file_token: MediaFileToken,
}

#[derive(Serialize)]
pub struct LocalMediaGetUrlSuccessResponse {
  /// `None` if the token is not local or its file no longer exists.
  pub url: Option<String>,
}

impl SerializeMarker for LocalMediaGetUrlSuccessResponse {}

/// Resolve a local media token to a URL the webview can load.
#[tauri::command]
pub async fn local_media_get_url_command(
  request: LocalMediaGetUrlRequest,
  local_media: State<'_, LocalMediaStore>,
) -> ResponseOrErrorMessage<LocalMediaGetUrlSuccessResponse> {
  let url = local_media.path_for_token(&request.media_file_token)
    .map(|path| LocalMediaStore::asset_url_for_path(&path));

  Ok(LocalMediaGetUrlSuccessResponse { url }.into())
}
