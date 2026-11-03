use comfyui_client::client::ComfyUiClient;
use comfyui_client::workflows::ace_step_music;
use comfyui_manager::manager::{ComfyUiManager, ModelState};
use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_type::TaskType;
use log::info;
use serde_json::{Map, Value};

use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::events::generation_events::common::GenerationModel;
use crate::core::local_models::comfyui_engine::resolve_base_url;
use crate::core::local_models::comfyui_settings::ComfyUiSettings;
use crate::core::local_models::local_audio_model::{LocalAudioModel, ACE_STEP_MODEL_ID};
use crate::core::local_models::seed::random_seed;

/// Music on the local ComfyUI engine. `fields` are the audio page's request fields.
pub async fn handle_comfyui_audio(
  fields: &Map<String, Value>,
  settings: &ComfyUiSettings,
  manager: &ComfyUiManager,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let model = fields.get("model").and_then(Value::as_str)
    .and_then(LocalAudioModel::from_model_id)
    .ok_or_else(|| GenerateError::NotYetImplemented("That audio model does not run on this computer".to_string()))?;

  let text = |key: &str| fields.get(key).and_then(Value::as_str).unwrap_or("").trim().to_string();
  let (prompt, style) = (text("prompt"), text("style_prompt"));
  let instrumental = fields.get("is_instrumental").and_then(Value::as_bool).unwrap_or(false);

  if prompt.is_empty() && style.is_empty() {
    return Err(GenerateError::AnyhowError(anyhow::anyhow!("Describe the music you want, or write some lyrics")));
  }

  // Check before starting the engine, so a missing model does not cost a cold start.
  if !settings.is_external() && manager.model_state(model.catalog()) != ModelState::Installed {
    return Err(GenerateError::AnyhowError(anyhow::anyhow!(
      "{} isn't downloaded yet. Open Settings, then Local Models, to download it.", model.display_name()
    )));
  }

  let bpm = fields.get("bpm").and_then(Value::as_u64).map(|bpm| bpm as u32);
  let musical_key = fields.get("musical_key").and_then(Value::as_str);
  let params = model.params(&prompt, &style, instrumental, bpm, musical_key, random_seed());

  // Starts the engine first if it is installed but not running.
  let base_url = resolve_base_url(settings, manager).await
    .map_err(|error| GenerateError::AnyhowError(anyhow::anyhow!("{}", error)))?;
  let client = ComfyUiClient::new(&base_url);

  info!("Queueing local music: model={}, {} s, {} steps, instrumental={}",
    model.display_name(), params.seconds, params.steps, instrumental);

  let prompt_id = client.queue_prompt(&ace_step_music(&params)).await
    .map_err(|err| GenerateError::AnyhowError(err.into()))?;

  Ok(TaskEnqueueSuccess {
    task_type: TaskType::AudioGeneration,
    model: Some(GenerationModel::Unknown(ACE_STEP_MODEL_ID.to_string())),
    provider: GenerationProvider::ComfyUi,
    provider_job_id: Some(prompt_id),
    maybe_queue_status_url: None,
    maybe_queue_response_url: None,
    maybe_prompt_token: None,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use comfyui_manager::layout::RuntimeLayout;
  use serde_json::json;
  use tempfile::tempdir;

  fn fields(value: Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
  }

  async fn run(value: Value) -> String {
    let root = tempdir().unwrap();
    let manager = ComfyUiManager::new(RuntimeLayout::new(root.path().join("comfyui")));
    let settings = ComfyUiSettings { external_url: None, checkpoint: None };
    match handle_comfyui_audio(&fields(value), &settings, &manager).await {
      Err(error) => format!("{:?}", error),
      Ok(_) => panic!("expected a refusal"),
    }
  }

  #[tokio::test]
  async fn only_the_local_music_model_is_accepted() {
    assert!(run(json!({ "model": "suno_music", "prompt": "la" })).await.contains("does not run on this computer"));
    assert!(run(json!({ "prompt": "la" })).await.contains("does not run on this computer"));
  }

  #[tokio::test]
  async fn something_to_go_on_is_required() {
    for fields in [json!({ "model": "ace_step_1p5_turbo" }), json!({ "model": "ace_step_1p5_turbo", "prompt": "  ", "style_prompt": "" })] {
      assert!(run(fields).await.contains("Describe the music"));
    }
  }

  #[tokio::test]
  async fn a_model_that_is_not_downloaded_says_where_to_get_it() {
    let message = run(json!({ "model": "ace_step_1p5_turbo", "prompt": "la la", "style_prompt": "folk" })).await;
    assert!(message.contains("ACE-Step 1.5 Music") && message.contains("Local Models"), "{message}");
  }
}
