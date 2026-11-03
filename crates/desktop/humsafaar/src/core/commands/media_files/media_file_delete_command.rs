use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use crate::core::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::core::events::functional_events::media_file_deleted_event::MediaFileDeletedEvent;
use crate::core::local_media::local_media_store::LocalMediaStore;
use errors::AnyhowResult;
use log::{error, info, warn};
use serde_derive::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tokens::tokens::media_files::MediaFileToken;

#[derive(Deserialize)]
pub struct MediaFileDeleteRequest {
  /// REQUIRED.
  /// Media file to delete. The user must own the file.
  pub media_file_token: MediaFileToken,
}

#[derive(Serialize)]
pub struct MediaFileDeleteSuccessResponse {
  pub success: bool,
}

impl SerializeMarker for MediaFileDeleteSuccessResponse {}

#[tauri::command]
pub async fn media_file_delete_command(
  app: AppHandle,
  request: MediaFileDeleteRequest,
  local_media: State<'_, LocalMediaStore>,
) -> ResponseOrErrorMessage<MediaFileDeleteSuccessResponse> {

  info!("media_file_delete_command called");

  let result = handle_request(
    request,
    &app,
    &local_media,
  ).await;

  if let Err(err) = result {
    error!("media_file_delete_command failed: {:?}", err);
    return Err("delete failed".into())
  }

  Ok(MediaFileDeleteSuccessResponse {
    success: true,
  }.into())
}

pub async fn handle_request(
  request: MediaFileDeleteRequest,
  app: &AppHandle,
  local_media: &LocalMediaStore,
) -> AnyhowResult<()> {

  if LocalMediaStore::is_local_token(&request.media_file_token) {
    local_media.delete(&request.media_file_token)?;
    MediaFileDeletedEvent {
      media_file_token: request.media_file_token,
    }.send_infallible(app);
    return Ok(());
  }

  // Everything is stored on this computer; there is nothing else to delete.
  Err(anyhow::anyhow!("That file is not stored on this computer"))
}
