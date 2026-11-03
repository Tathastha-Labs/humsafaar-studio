use crate::core::commands::enqueue::generate_error::{GenerateError, MissingCredentialsReason};
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::commands::generate::generate_image::providers::humsafaar_router::handle_router::handle_router;
use crate::core::commands::generate::generate_image::providers::comfyui::handle_comfyui::handle_comfyui;
use crate::core::commands::generate::generate_image::tauri_generate_image_request::{
  TauriGenerateImageErrorType, TauriGenerateImageRequest, TauriGenerateImageResponse,
};
use crate::core::commands::generate::omni::dispatch::{adapt_legacy_response, decode_native};
use crate::core::commands::generate::omni::{OmniRequest, OmniResult};
use crate::core::commands::response::failure_response_wrapper::{CommandErrorResponseWrapper, CommandErrorStatus};
use crate::core::commands::response::shorthand::Response;
use crate::core::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::core::events::generation_events::generation_enqueue_success_event::GenerationEnqueueSuccessEvent;
use crate::core::local_media::local_media_store::LocalMediaStore;
use crate::core::local_models::comfyui_settings::ComfyUiSettings;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;
use crate::core::state::task_database::TaskDatabase;
use comfyui_manager::manager::ComfyUiManager;
use enums::common::generation_provider::GenerationProvider;
use log::{error, info};
use tauri::{AppHandle, Manager, State};

/// Where an image is made when the page does not say. Hosted generation no longer exists.
const DEFAULT_PROVIDER: GenerationProvider = GenerationProvider::Fal;

#[tauri::command]
pub async fn generate_image_command(request: OmniRequest, app: AppHandle) -> OmniResult {
  adapt_legacy_response(generate_image_native(
    decode_native(request)?, app.clone(),
    app.state(), app.state(), app.state(), app.state(), app.state(),
  ).await)
}

async fn generate_image_native(
  request: TauriGenerateImageRequest,
  app: AppHandle,
  task_database: State<'_, TaskDatabase>,
  credential_cache: State<'_, ProviderCredentialLoadingCache>,
  local_media: State<'_, LocalMediaStore>,
  comfyui_settings: State<'_, ComfyUiSettings>,
  comfyui_manager: State<'_, ComfyUiManager>,
) -> Response<TauriGenerateImageResponse, TauriGenerateImageErrorType, ()> {

  info!("generate_image_command called, request: {:?}", request);

  let provider = request.provider.unwrap_or(DEFAULT_PROVIDER);

  let result = match provider {
    GenerationProvider::ComfyUi => handle_comfyui(&request, &comfyui_settings, &comfyui_manager).await,
    other => handle_router(&request, other, &credential_cache, &local_media).await,
  };

  match result {
    Ok(success) => handle_success_behavior(&app, &task_database, &request, success).await,
    Err(err) => handle_error_behavior(&app, err).await,
  }
}

// ── Result mapping ──

async fn handle_success_behavior(
  app: &AppHandle,
  task_database: &TaskDatabase,
  request: &TauriGenerateImageRequest,
  success: TaskEnqueueSuccess,
) -> Response<TauriGenerateImageResponse, TauriGenerateImageErrorType, ()> {


  // Insert into task database
  let db_result = success
      .insert_into_task_database_with_frontend_payload(
        &task_database,
        request.frontend_caller,
        request.frontend_subscriber_id.as_deref(),
        request.frontend_subscriber_payload.as_deref(),
      )
      .await;

  if let Err(err) = db_result {
    error!("Failed to create task in database: {:?}", err);
  }

  /*
  let usage_type = if is_image_to_image {
    HumsafaarUsageType::ImageToResult
  } else {
    HumsafaarUsageType::TextToResult
  };

  if let Err(err) = humsafaar_usage_tracker.record_image_generation(num_images, usage_type, HumsafaarUsagePage::ImagePage) {
    // NB: Fail open.
    warn!("Failed to report usage: {:?}", err);
  }
   */

  let event = GenerationEnqueueSuccessEvent {
    action: success.to_frontend_event_action(),
    service: success.to_frontend_event_service(),
    model: success.model,
  };

  if let Err(err) = event.send(&app) {
    error!("Failed to emit event: {:?}", err);
  }

  Ok(TauriGenerateImageResponse {}.into())
}

async fn handle_error_behavior(
  app: &AppHandle,
  err: GenerateError,
) -> Response<TauriGenerateImageResponse, TauriGenerateImageErrorType, ()> {
  error!("generate_image_command error: {:?}", err);


  let error_type = match &err {
    GenerateError::BadInput(_) => TauriGenerateImageErrorType::BadInput,
    GenerateError::MissingCredentials(MissingCredentialsReason::NeedsFalApiKey) => TauriGenerateImageErrorType::NeedsFalApiKey,
    GenerateError::NoProviderAvailable => TauriGenerateImageErrorType::NoProviderAvailable,
    _ => TauriGenerateImageErrorType::ServerError,
  };

  // TODO:
  /*
    let mut status = CommandErrorStatus::ServerError;
    let mut error_type = EnqueueTextToImageErrorType::ServerError;
    let mut error_message = "A server error occurred. Please try again. If it continues, please tell our staff about the problem.";

    match err {
      GenerateError::BadInput(BadInputReason::NoModelSpecified) => {
        status = CommandErrorStatus::BadRequest;
        error_type = EnqueueTextToImageErrorType::ModelNotSpecified;
        error_message = "No model specified for image generation";
      }
      GenerateError::NoProviderAvailable => {
        status = CommandErrorStatus::ServerError;
        error_type = EnqueueTextToImageErrorType::NoProviderAvailable;
        error_message = "No configured provider available for image generation";
      }
      GenerateError::MissingCredentials(MissingCredentialsReason::NeedsFalApiKey) => {
        status = CommandErrorStatus::Unauthorized;
        error_type = EnqueueTextToImageErrorType::NeedsFalApiKey;
        error_message = "You need to set a FAL api key";
      },
      _ => {}, // Fall-through
    }

    Err(CommandErrorResponseWrapper {
      status,
      error_message: Some(error_message.to_string()),
      error_type: Some(error_type),
      error_details: None,
    })
   */

  let message = match &err {
    GenerateError::AnyhowError(message) => message.to_string(),
    GenerateError::NotYetImplemented(message) => message.clone(),
    GenerateError::MissingCredentials(MissingCredentialsReason::NeedsFalApiKey) => "Add your Fal API key in Settings, then API Keys.".to_string(),
    _ => format!("{:?}", err),
  };

  Err(CommandErrorResponseWrapper {
    status: CommandErrorStatus::ServerError,
    error_message: Some(message),
    error_type: Some(error_type),
    error_details: None,
  })
}
