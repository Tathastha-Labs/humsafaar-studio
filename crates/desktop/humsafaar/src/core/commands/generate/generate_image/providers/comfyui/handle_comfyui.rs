use comfyui_client::client::ComfyUiClient;
use comfyui_client::workflows::{text_to_image, TextToImageParams};
use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_type::TaskType;
use log::info;

use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::commands::generate::generate_image::tauri_generate_image_request::TauriGenerateImageRequest;
use crate::core::events::generation_events::common::GenerationModel;
use crate::core::local_models::comfyui_engine::resolve_base_url;
use crate::core::local_models::comfyui_settings::ComfyUiSettings;
use crate::core::local_models::local_image_model::LocalImageModel;
use crate::core::local_models::seed::random_seed;
use comfyui_manager::manager::{ComfyUiManager, ModelState};

const MAX_BATCH_SIZE: u32 = 4;

/// Text-to-image on the local ComfyUI server. Image-conditioned generation is not supported yet.
pub async fn handle_comfyui(
  request: &TauriGenerateImageRequest,
  settings: &ComfyUiSettings,
  manager: &ComfyUiManager,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let prompt = request.prompt.as_deref().map(str::trim).filter(|p| !p.is_empty())
    .ok_or_else(|| GenerateError::AnyhowError(anyhow::anyhow!("A prompt is required")))?;

  let has_image_inputs = request.image_media_tokens.as_ref().is_some_and(|t| !t.is_empty())
    || request.canvas_image_media_token.is_some()
    || request.scene_image_media_token.is_some()
    || request.canvas_image_raw_bytes.is_some()
    || request.scene_image_raw_bytes.is_some();
  if has_image_inputs {
    return Err(GenerateError::NotYetImplemented(
      "Local Stable Diffusion supports text-to-image only for now".to_string(),
    ));
  }

  let model = request.model.and_then(LocalImageModel::from_tauri).unwrap_or(LocalImageModel::StableDiffusion15);
  let not_downloaded = || GenerateError::AnyhowError(anyhow::anyhow!(
    "{} isn't downloaded yet. Open Settings, then Local Models, to download it.", model.display_name()
  ));

  // Check before starting the engine, so a missing model does not cost a cold start.
  if !settings.is_external() && manager.model_state(model.catalog()) != ModelState::Installed {
    return Err(not_downloaded());
  }

  // Starts the engine first if it is installed but not running.
  let base_url = resolve_base_url(settings, manager).await
    .map_err(|error| GenerateError::AnyhowError(anyhow::anyhow!("{}", error)))?;
  let client = ComfyUiClient::new(&base_url);

  let checkpoint = match &settings.checkpoint {
    Some(name) => name.clone(),
    None => {
      let available = client.list_checkpoints().await
        .map_err(|err| GenerateError::AnyhowError(err.into()))?;
      if !available.iter().any(|name| name == model.checkpoint_file()) {
        return Err(not_downloaded());
      }
      model.checkpoint_file().to_string()
    }
  };

  let (width, height) = model.dimensions();
  let mut params = TextToImageParams::new(&checkpoint, prompt, random_seed());
  params.width = width;
  params.height = height;
  params.steps = model.steps();
  params.cfg = model.cfg();
  params.batch_size = request.batch_size.unwrap_or(1).clamp(1, MAX_BATCH_SIZE);

  info!("Queueing local ComfyUI text-to-image: model={}, checkpoint={}, {}x{}, {} steps, batch={}",
    model.display_name(), checkpoint, width, height, params.steps, params.batch_size);

  let prompt_id = client.queue_prompt(&text_to_image(&params)).await
    .map_err(|err| GenerateError::AnyhowError(err.into()))?;

  Ok(TaskEnqueueSuccess {
    task_type: TaskType::ImageGeneration,
    model: Some(GenerationModel::Unknown(model.catalog().id.to_string())),
    provider: GenerationProvider::ComfyUi,
    provider_job_id: Some(prompt_id),
    maybe_queue_status_url: None,
    maybe_queue_response_url: None,
    maybe_prompt_token: None,
  })
}
