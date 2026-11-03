use crate::core::commands::enqueue::generate_error::{BadInputReason, GenerateError, MissingCredentialsReason};
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::commands::generate::generate_video::comfyui::handle_comfyui_video::handle_comfyui_video;
use crate::core::commands::generate::generate_video::fal::handle_fal_video::handle_fal_video;
use crate::core::commands::generate::generate_video::gmi::handle_gmi_video::handle_gmi_video;
use crate::core::commands::generate::generate_video::grok::handle_grok_video::handle_grok_video;
use crate::core::commands::generate::generate_video::replicate::handle_replicate_video::handle_replicate_video;
use crate::core::commands::generate::generate_video::request::{
  TauriGenerateVideoErrorType, TauriGenerateVideoRequest, TauriGenerateVideoResponse,
};
use crate::core::commands::generate::omni::dispatch::{adapt_legacy_response, decode_native};
use crate::core::commands::generate::omni::{OmniRequest, OmniResult};
use crate::core::commands::response::failure_response_wrapper::{CommandErrorResponseWrapper, CommandErrorStatus};
use crate::core::commands::response::shorthand::Response;
use crate::core::events::generation_events::generation_enqueue_success_event::GenerationEnqueueSuccessEvent;
use crate::core::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::core::local_media::local_media_store::LocalMediaStore;
use crate::core::local_models::comfyui_settings::ComfyUiSettings;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;
use crate::core::state::task_database::TaskDatabase;
use comfyui_manager::manager::ComfyUiManager;
use enums::common::generation_provider::GenerationProvider;
use log::{error, info};
use tauri::{AppHandle, Manager, State};

/// Where a video is made when the page does not say. Hosted generation no longer exists.
const DEFAULT_PROVIDER: GenerationProvider = GenerationProvider::Fal;

#[tauri::command]
pub async fn generate_video_command(request: OmniRequest, app: AppHandle) -> OmniResult {
  adapt_legacy_response(generate_video_native(
    decode_native(request)?, app.clone(),
    app.state(), app.state(), app.state(), app.state(), app.state(),
  ).await)
}

async fn generate_video_native(
  mut request: TauriGenerateVideoRequest,
  app: AppHandle,
  task_database: State<'_, TaskDatabase>,
  credentials: State<'_, ProviderCredentialLoadingCache>,
  comfyui_settings: State<'_, ComfyUiSettings>,
  comfyui_manager: State<'_, ComfyUiManager>,
  local_media: State<'_, LocalMediaStore>,
) -> Response<TauriGenerateVideoResponse, TauriGenerateVideoErrorType, ()> {

  info!("generate_video_command called, request: {:?}", request);

  // Older callers send one name for the start picture and newer ones the other.
  if request.image_media_token.is_none() && request.start_frame_image_media_token.is_some() {
    request.image_media_token = request.start_frame_image_media_token.clone();
  }
  if request.image_media_token.is_some() && request.start_frame_image_media_token.is_none() {
    request.start_frame_image_media_token = request.image_media_token.clone();
  }

  let result = handle_request(&request, &task_database, &credentials, &comfyui_settings, &comfyui_manager, &local_media).await;

  match result {
    Err(err) => {
      error!("error: {:?}", err);
      Err(error_response(err))
    }
    Ok(success) => {
      let event = GenerationEnqueueSuccessEvent {
        action: success.to_frontend_event_action(),
        service: success.to_frontend_event_service(),
        model: success.model,
      };
      if let Err(err) = event.send(&app) {
        error!("Failed to emit event: {:?}", err);
      }
      Ok(TauriGenerateVideoResponse {}.into())
    }
  }
}

async fn handle_request(
  request: &TauriGenerateVideoRequest,
  task_database: &TaskDatabase,
  credentials: &ProviderCredentialLoadingCache,
  comfyui_settings: &ComfyUiSettings,
  comfyui_manager: &ComfyUiManager,
  local_media: &LocalMediaStore,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let model = request.model.ok_or_else(GenerateError::no_model_specified)?;
  let provider = request.provider.unwrap_or(DEFAULT_PROVIDER);

  info!("generate video with {:?} via provider {:?}", &model, &provider);

  let success = match provider {
    GenerationProvider::ComfyUi => handle_comfyui_video(request, comfyui_settings, comfyui_manager, local_media).await?,
    GenerationProvider::Fal => handle_fal_video(request, credentials, local_media).await?,
    GenerationProvider::Grok => handle_grok_video(request, credentials, local_media).await?,
    GenerationProvider::GmiCloud => handle_gmi_video(request, credentials, local_media).await?,
    GenerationProvider::Replicate => handle_replicate_video(request, credentials, local_media).await?,
    other => return Err(GenerateError::NotYetImplemented(format!("Video is not available through {:?}", other))),
  };

  let result = success
    .insert_into_task_database_with_frontend_payload(
      task_database,
      request.frontend_caller,
      request.frontend_subscriber_id.as_deref(),
      request.frontend_subscriber_payload.as_deref(),
    )
    .await;

  if let Err(err) = result {
    // The provider already has the job, so do not encourage a duplicate retry.
    error!("Failed to create task in database: {:?}", err);
  }

  Ok(success)
}

/// What the page is told. Plain-language errors (a missing key, a refused setting) are passed on as
/// written, since they say what to do.
fn error_response(err: GenerateError) -> CommandErrorResponseWrapper<TauriGenerateVideoErrorType, ()> {
  let (status, error_type, message) = match &err {
    GenerateError::BadInput(BadInputReason::NoModelSpecified) => (
      CommandErrorStatus::BadRequest, TauriGenerateVideoErrorType::ModelNotSpecified,
      "No model specified for video generation".to_string(),
    ),
    GenerateError::NoProviderAvailable => (
      CommandErrorStatus::ServerError, TauriGenerateVideoErrorType::NoProviderAvailable,
      "No configured provider available for video generation".to_string(),
    ),
    GenerateError::MissingCredentials(MissingCredentialsReason::NeedsFalApiKey) => (
      CommandErrorStatus::Unauthorized, TauriGenerateVideoErrorType::NeedsFalApiKey,
      "Add your Fal API key in Settings, then API Keys.".to_string(),
    ),
    GenerateError::AnyhowError(message) => (
      CommandErrorStatus::ServerError, TauriGenerateVideoErrorType::ServerError, message.to_string(),
    ),
    GenerateError::NotYetImplemented(message) => (
      CommandErrorStatus::BadRequest, TauriGenerateVideoErrorType::NoProviderAvailable, message.clone(),
    ),
    _ => (
      CommandErrorStatus::ServerError, TauriGenerateVideoErrorType::ServerError,
      "Something went wrong starting this video. Please try again.".to_string(),
    ),
  };

  CommandErrorResponseWrapper {
    status,
    error_message: Some(message),
    error_type: Some(error_type),
    error_details: None,
  }
}
