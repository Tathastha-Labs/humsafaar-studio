use comfyui_client::client::ComfyUiClient;
use comfyui_client::workflows::wan_video;
use comfyui_manager::manager::{ComfyUiManager, ModelState};
use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_type::TaskType;
use log::info;

use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::commands::generate::generate_video::request::TauriGenerateVideoRequest;
use crate::core::events::generation_events::common::GenerationModel;
use crate::core::local_media::local_media_store::LocalMediaStore;
use crate::core::local_models::comfyui_engine::resolve_base_url;
use crate::core::local_models::comfyui_settings::ComfyUiSettings;
use crate::core::local_models::local_video_model::LocalVideoModel;
use crate::core::local_models::seed::random_seed;

/// Video on the local ComfyUI engine, from a prompt alone or from a start picture.
pub async fn handle_comfyui_video(
  request: &TauriGenerateVideoRequest,
  settings: &ComfyUiSettings,
  manager: &ComfyUiManager,
  local_media: &LocalMediaStore,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let model = request.model.and_then(LocalVideoModel::from_tauri)
    .ok_or_else(|| GenerateError::NotYetImplemented("That video model does not run on this computer".to_string()))?;

  let prompt = request.prompt.as_deref().map(str::trim).filter(|p| !p.is_empty())
    .ok_or_else(|| GenerateError::AnyhowError(anyhow::anyhow!("A prompt is required")))?;

  // Check before starting the engine, so a missing model does not cost a cold start.
  if !settings.is_external() && manager.model_state(model.catalog()) != ModelState::Installed {
    return Err(GenerateError::AnyhowError(anyhow::anyhow!(
      "{} isn't downloaded yet. Open Settings, then Local Models, to download it.", model.display_name()
    )));
  }

  let start_picture = match &request.start_frame_image_media_token {
    Some(token) => {
      if !LocalMediaStore::is_local_token(token) {
        return Err(GenerateError::AnyhowError(anyhow::anyhow!(
          "That picture is not stored on this computer. Add it again from your computer to animate it."
        )));
      }
      let path = local_media.path_for_token(token)
        .ok_or_else(|| GenerateError::AnyhowError(anyhow::anyhow!("The start picture could not be found")))?;
      let bytes = tokio::fs::read(&path).await
        .map_err(|err| GenerateError::AnyhowError(anyhow::anyhow!("Could not read the start picture: {}", err)))?;
      Some((token.as_str().to_string(), bytes))
    }
    None => None,
  };

  // Starts the engine first if it is installed but not running.
  let base_url = resolve_base_url(settings, manager).await
    .map_err(|error| GenerateError::AnyhowError(anyhow::anyhow!("{}", error)))?;
  let client = ComfyUiClient::new(&base_url);

  // The size and length are fixed by the model description; see `LocalVideoModel`.
  let mut params = model.params(prompt, request.negative_prompt.as_deref().unwrap_or(""), random_seed());

  if let Some((token, bytes)) = start_picture {
    // Named by token so two jobs started together do not overwrite each other's picture.
    let uploaded = client.upload_image(bytes, &format!("{}.png", token)).await
      .map_err(|err| GenerateError::AnyhowError(err.into()))?;
    params.start_image = Some(uploaded);
  }

  info!("Queueing local video: model={}, {}x{}, {} frames, {} steps, from a picture={}",
    model.display_name(), params.width, params.height, params.frames, params.steps, params.start_image.is_some());

  let prompt_id = client.queue_prompt(&wan_video(&params)).await
    .map_err(|err| GenerateError::AnyhowError(err.into()))?;

  Ok(TaskEnqueueSuccess {
    task_type: TaskType::VideoGeneration,
    model: Some(GenerationModel::Unknown(model.catalog().id.to_string())),
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
  use tempfile::{tempdir, TempDir};

  mod refusals {
    use super::*;

    #[tokio::test]
    async fn a_prompt_is_required() {
      let fixture = Fixture::new();
      for prompt in [json!(null), json!(""), json!("   ")] {
        let error = fixture.run(json!({ "model": "wan_2p2_ti2v_5b", "prompt": prompt })).await.unwrap_err();
        assert!(message(&error).contains("prompt is required"), "{}", message(&error));
      }
    }

    #[tokio::test]
    async fn only_the_local_video_model_is_accepted() {
      let fixture = Fixture::new();
      let error = fixture.run(json!({ "model": "veo_3", "prompt": "a fox" })).await.unwrap_err();
      assert!(message(&error).contains("does not run on this computer"), "{}", message(&error));

      let error = fixture.run(json!({ "prompt": "a fox" })).await.unwrap_err();
      assert!(message(&error).contains("does not run on this computer"), "{}", message(&error));
    }

    #[tokio::test]
    async fn a_model_that_is_not_downloaded_says_where_to_get_it() {
      let fixture = Fixture::new();
      let error = fixture.run(json!({ "model": "wan_2p2_ti2v_5b", "prompt": "a fox" })).await.unwrap_err();
      let text = message(&error);
      assert!(text.contains("Wan 2.2 Video (5B)") && text.contains("Local Models"), "{text}");
    }
  }

  mod start_pictures {
    use super::*;

    #[tokio::test]
    async fn a_picture_from_the_hosted_service_is_refused_clearly() {
      let fixture = Fixture::with_model_installed();
      let error = fixture.run(json!({
        "model": "wan_2p2_ti2v_5b", "prompt": "a fox", "start_frame_image_media_token": "m_hosted123"
      })).await.unwrap_err();
      assert!(message(&error).contains("not stored on this computer"), "{}", message(&error));
    }

    #[tokio::test]
    async fn a_local_token_whose_file_is_gone_is_reported() {
      let fixture = Fixture::with_model_installed();
      let error = fixture.run(json!({
        "model": "wan_2p2_ti2v_5b", "prompt": "a fox", "start_frame_image_media_token": "mf_local_deadbeef"
      })).await.unwrap_err();
      assert!(message(&error).contains("could not be found"), "{}", message(&error));
    }
  }

  // ── helpers ──

  struct Fixture {
    root: TempDir,
    manager: ComfyUiManager,
    settings: ComfyUiSettings,
    local_media: LocalMediaStore,
  }

  impl Fixture {
    fn new() -> Self {
      let root = tempdir().unwrap();
      Self {
        manager: ComfyUiManager::new(RuntimeLayout::new(root.path().join("comfyui"))),
        settings: ComfyUiSettings { external_url: None, checkpoint: None },
        local_media: LocalMediaStore::new(root.path().join("media")),
        root,
      }
    }

    /// Wan's three files are in place, so the checks that come after the download check can run.
    fn with_model_installed() -> Self {
      let fixture = Self::new();
      for file in LocalVideoModel::Wan22Ti2v5b.catalog().files {
        let path = comfyui_manager::models::file_path(fixture.manager.layout(), file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let handle = std::fs::File::create(&path).unwrap();
        handle.set_len(file.size_bytes).unwrap(); // sparse, so this takes no real space
      }
      assert_eq!(fixture.manager.model_state(LocalVideoModel::Wan22Ti2v5b.catalog()), ModelState::Installed);
      fixture
    }

    /// Only the failure matters to these tests.
    async fn run(&self, request: serde_json::Value) -> Result<(), GenerateError> {
      let request: TauriGenerateVideoRequest = serde_json::from_value(request).unwrap();
      let _keep_alive = &self.root;
      handle_comfyui_video(&request, &self.settings, &self.manager, &self.local_media).await.map(|_| ())
    }
  }

  fn message(error: &GenerateError) -> String {
    format!("{:?}", error)
  }
}
