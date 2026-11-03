use crate::core::cloud_models::fal_queue::{enqueue_on_fal, load_fal_api_key};
use crate::core::cloud_models::local_picture::LocalPicture;
use crate::core::commands::generate::enqueued_job::record_enqueued_job;
use crate::core::commands::generate::omni::OmniRequest;
use crate::core::commands::response::failure_response_wrapper::{CommandErrorResponseWrapper, CommandErrorStatus};
use crate::core::local_media::local_media_store::LocalMediaStore;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;
use enums::tauri::tasks::task_type::TaskType;
use log::error;
use serde_json::{json, Map};
use tauri::{AppHandle, Manager};
use crate::core::commands::response::shorthand::ResponseOrErrorType;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use enums::tauri::ux::tauri_command_caller::TauriCommandCaller;
use serde_derive::{Deserialize, Serialize};
use tokens::tokens::media_files::MediaFileToken;

#[derive(Deserialize, Debug)]
pub struct EnqueueImageBgRemovalCommand {
  // /// The bg removal model to use (optional).
  // pub model: Option<TODO>,

  /// Image media file; the image to remove the background from.
  pub image_media_token: Option<MediaFileToken>,

  /// Base64-encoded image
  pub base64_image: Option<String>,

  /// OPTIONAL.
  /// Name of the frontend caller.
  /// We'll use this to selectively trigger events.
  pub frontend_caller: Option<TauriCommandCaller>,

  /// OPTIONAL.
  /// A frontend-defined identifier that we'll send back to the frontend
  /// as a Tauri event on task completion.
  pub frontend_subscriber_id: Option<String>,

  /// OPTIONAL.
  /// A frontend-defined payload that we'll send back to the frontend
  /// as a Tauri event on task completion.
  pub frontend_subscriber_payload: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum EnqueueImageBgRemovalErrorType {
  /// No model available for image generation
  NoProviderAvailable,

  /// No image was provided for background removal
  MissingImage,
  
  /// Bad base64 image data
  Base64DecodeError,
  
  /// Generic bad request error
  BadRequest,

  /// Generic server error
  ServerError,
}

#[derive(Serialize)]
pub struct EnqueueImageBgRemovalSuccessResponse {
}

impl SerializeMarker for EnqueueImageBgRemovalSuccessResponse {}

const FAL_BACKGROUND_REMOVAL_ENDPOINT: &str = "fal-ai/imageutils/rembg";
const FAL_BACKGROUND_REMOVAL_MODEL: &str = "rembg";

/// Background removal on Fal with the user's own key. The shared poller saves the result and
/// sends the completion event the page waits for.
#[tauri::command]
pub async fn enqueue_image_bg_removal_command(
  request: EnqueueImageBgRemovalCommand,
  app: AppHandle,
) -> ResponseOrErrorType<EnqueueImageBgRemovalSuccessResponse, EnqueueImageBgRemovalErrorType> {
  let image = match picture_data_uri(&request, &app.state::<LocalMediaStore>()) {
    Ok(image) => image,
    Err(err) => return Err(fail(EnqueueImageBgRemovalErrorType::MissingImage, err)),
  };

  let credentials = app.state::<ProviderCredentialLoadingCache>();
  let api_key = load_fal_api_key(&credentials)
    .map_err(|_| fail(EnqueueImageBgRemovalErrorType::NoProviderAvailable, "Add your Fal key in Settings, then API Keys, to remove backgrounds.".to_string()))?;

  let success = enqueue_on_fal(
    FAL_BACKGROUND_REMOVAL_ENDPOINT,
    json!({ "image_url": image }),
    &api_key,
    TaskType::BackgroundRemoval,
    FAL_BACKGROUND_REMOVAL_MODEL,
  ).await.map_err(|err| {
    error!("Background removal failed to start: {:?}", err);
    fail(EnqueueImageBgRemovalErrorType::ServerError, format!("{:?}", err))
  })?;

  let omni = OmniRequest {
    provider: None,
    frontend_caller: request.frontend_caller,
    frontend_subscriber_id: request.frontend_subscriber_id,
    frontend_subscriber_payload: request.frontend_subscriber_payload,
    fields: Map::new(),
  };
  // The job is on Fal already; a recording problem is logged inside and must not fail the request.
  let _ = record_enqueued_job(&omni, success, &app).await;
  Ok(EnqueueImageBgRemovalSuccessResponse {}.into())
}

/// The picture as an inline `data:` URI, from base64 sent by the page or a picture saved on this computer.
fn picture_data_uri(request: &EnqueueImageBgRemovalCommand, local_media: &LocalMediaStore) -> Result<String, String> {
  if let Some(base64) = request.base64_image.as_deref().filter(|b| !b.is_empty()) {
    return Ok(if base64.starts_with("data:") { base64.to_string() } else { format!("data:image/png;base64,{}", base64) });
  }
  match &request.image_media_token {
    Some(token) => LocalPicture::read(token, local_media).map(|picture| picture.data_uri()).map_err(|err| format!("{:?}", err)),
    None => Err("Add an image first.".to_string()),
  }
}

fn fail(error_type: EnqueueImageBgRemovalErrorType, message: String) -> CommandErrorResponseWrapper<EnqueueImageBgRemovalErrorType, ()> {
  CommandErrorResponseWrapper {
    status: CommandErrorStatus::ServerError,
    error_message: Some(message),
    error_type: Some(error_type),
    error_details: None,
  }
}
