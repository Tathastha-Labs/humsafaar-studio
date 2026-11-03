use humsafaar_router::api::image_list_ref::ImageListRef;
use humsafaar_router::api::router_provider::RouterProvider;
use humsafaar_router::client::generation_mode_mismatch_strategy::GenerationModeMismatchStrategy;
use humsafaar_router::client::request_mismatch_mitigation_strategy::RequestMismatchMitigationStrategy;
use humsafaar_router::client::router_client::RouterClient;
use humsafaar_router::client::router_fal_client::RouterFalClient;
use humsafaar_router::generate::generate_image::generate_image_request_builder::GenerateImageRequestBuilder;
use humsafaar_router::generate::generate_image::generate_image_response::GenerateImageResponse;
use humsafaar_router::generate::generate_image::image_generation_draft_or_request::ImageGenerationDraftOrRequest;
use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_type::TaskType;
use log::{error, info, warn};
use tokens::tokens::media_files::MediaFileToken;
use tokens::tokens::prompts::PromptToken;

use crate::core::api_adapters::models::image::tauri_image_model_to_generation_model::tauri_image_model_to_generation_model;
use crate::core::api_adapters::models::image::tauri_image_model_to_router_model::tauri_image_model_to_router_model;
use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::commands::generate::generate_image::providers::humsafaar_router::utils::convert_enums_to_router::{convert_aspect_ratio, convert_quality, convert_resolution};
use crate::core::commands::generate::generate_image::tauri_generate_image_request::TauriGenerateImageRequest;
use crate::core::commands::generate::generate_image::tauri_image_model::TauriImageModel;
use crate::core::local_media::local_media_store::LocalMediaStore;

/// Handle image generation for providers that authenticate via API key.
pub async fn handle_api_key_provider(
  request: &TauriGenerateImageRequest,
  provider: GenerationProvider,
  api_key: &str,
  local_media: &LocalMediaStore,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  match provider {
    GenerationProvider::Fal => {
      handle_fal(request, api_key, local_media).await
    }
    _ => {
      Err(GenerateError::NotYetImplemented(
        format!("API key provider {:?} is not yet supported via the router path", provider),
      ))
    }
  }
}

// ── FAL ──

async fn handle_fal(
  request: &TauriGenerateImageRequest,
  api_key: &str,
  local_media: &LocalMediaStore,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let tauri_model = request.model.ok_or(GenerateError::no_model_specified())?;

  let router_model = tauri_image_model_to_router_model(tauri_model)
    .ok_or(GenerateError::NotYetImplemented(
      format!("Model {:?} is not supported via the FAL router path", tauri_model),
    ))?;

  // Collect all media file tokens that need resolving.
  let image_inputs = resolve_image_inputs(request, local_media).await?;

  let router_request = GenerateImageRequestBuilder {
    model: router_model,
    provider: RouterProvider::Fal,
    prompt: request.prompt.clone(),
    image_inputs,
    resolution: request.resolution.map(convert_resolution),
    aspect_ratio: request.aspect_ratio.map(convert_aspect_ratio),
    quality: request.quality.map(convert_quality),
    image_batch_count: request.batch_size.map(|n| n as u16),
    horizontal_angle: request.adjust_horizontal_angle,
    vertical_angle: request.adjust_vertical_angle,
    zoom: request.adjust_zoom,
    request_mismatch_mitigation_strategy: RequestMismatchMitigationStrategy::PayMoreUpgrade,
    generation_mode_mismatch_strategy: Some(GenerationModeMismatchStrategy::GenerateAnyway),
    idempotency_token: None,
  };

  let fal_client = RouterFalClient::new_polling_only_from_raw_key(api_key);
  let client = RouterClient::Fal(fal_client);

  info!("Building FAL image generation plan: model={:?}", router_model);

  let request = match router_request.build2() {
    Ok(ImageGenerationDraftOrRequest::Request(request)) => request,
    Ok(ImageGenerationDraftOrRequest::Draft(draft)) => {
      warn!("Fal is trying to send draft request: {:?}", draft);
      return Err(GenerateError::NotYetImplemented("Fal should not be sending draft requests".to_string()));
    },
    Err(err) => {
      warn!("Could not use FAL: {:?}", err);
      return Err(GenerateError::NotYetImplemented("Error Message: TODO".to_string()));
    }
  };

  info!("Executing FAL image generation. Request: {:?}", request);

  match request.send_request(&client).await {
    Ok(response) => {
      build_task_enqueue_success(tauri_model, response)
    },
    Err(err) => {
      warn!("Fal image generation failed: {:?}", err);
      Err(err.into())
    }
  }
}

// ── Helpers ──

pub(super) async fn resolve_image_inputs(
  request: &TauriGenerateImageRequest,
  local_media: &LocalMediaStore,
) -> Result<Option<ImageListRef>, GenerateError> {
  let mut tokens: Vec<MediaFileToken> = Vec::new();

  if let Some(canvas_token) = &request.canvas_image_media_token {
    tokens.push(canvas_token.clone());
  }

  if let Some(scene_token) = &request.scene_image_media_token {
    tokens.push(scene_token.clone());
  }

  if let Some(media_tokens) = &request.image_media_tokens {
    tokens.extend(media_tokens.clone());
  }

  if tokens.is_empty() {
    return Ok(None);
  }

  // Pictures live on this computer, so they are sent inline as data URIs (Fal cannot reach this
  // machine). Input order is preserved.
  let mut urls = Vec::with_capacity(tokens.len());
  for token in &tokens {
    if !LocalMediaStore::is_local_token(token) {
      return Err(GenerateError::AnyhowError(anyhow::anyhow!(
        "That picture is not stored on this computer. Add it again from your computer."
      )));
    }
    let data_uri = local_media.data_uri_for_token(token)
      .map_err(|err| GenerateError::AnyhowError(err))?;
    urls.push(data_uri);
  }
  Ok(Some(ImageListRef::Urls(urls)))
}

fn build_task_enqueue_success(
  tauri_model: TauriImageModel,
  response: GenerateImageResponse,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let fal_payload = response.get_fal_payload()
    .ok_or(GenerateError::ResponseHadNoJobTokens)?;

  let provider_job_id = fal_payload.request_id
    .or(fal_payload.gateway_request_id)
    .ok_or(GenerateError::ResponseHadNoJobTokens)?;

  let generation_model = tauri_image_model_to_generation_model(tauri_model);

  Ok(TaskEnqueueSuccess {
    task_type: TaskType::ImageGeneration,
    model: Some(generation_model),
    provider: GenerationProvider::Fal,
    provider_job_id: Some(provider_job_id),
    maybe_queue_status_url: fal_payload.maybe_status_url,
    maybe_queue_response_url: fal_payload.maybe_response_url,
    maybe_prompt_token: None,
  })
}
