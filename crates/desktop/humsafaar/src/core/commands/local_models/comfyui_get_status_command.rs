use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use crate::core::local_models::comfyui_settings::ComfyUiSettings;
use comfyui_manager::hardware::GpuKind;
use comfyui_manager::manager::{ComfyUiManager, EngineStatus, ModelState, IDLE_STOP_AFTER};
use comfyui_manager::models::CATALOG;
use comfyui_manager::suitability::{assess, Assessment};
use serde_derive::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct ComfyUiGetStatusSuccessResponse {
  pub status: EngineStatus,

  /// The engine is a ComfyUI the user runs; the app does not install or start it.
  pub is_external: bool,

  /// What this computer has to run models with.
  pub hardware: HardwareInfo,

  /// Every model the app can download, with where each one stands.
  pub models: Vec<LocalModelEntry>,

  /// Where the engine and its models are stored.
  pub install_folder: String,

  /// The engine stops by itself after this many idle minutes, and starts again when needed.
  pub idle_stop_minutes: u64,
}

#[derive(Serialize)]
pub struct HardwareInfo {
  /// One line, eg. "Apple M4 Pro · 24 GB shared memory".
  pub summary: String,
  pub gpu_kind: GpuKind,
}

#[derive(Serialize)]
pub struct LocalModelEntry {
  pub id: String,
  pub display_name: String,
  pub license: String,
  pub size_bytes: u64,
  pub state: ModelState,
  /// What it needs at the least, and what makes it comfortable.
  pub minimum_hardware: String,
  pub recommended_hardware: String,
  /// How well it will run on this computer.
  pub assessment: Assessment,
}

impl SerializeMarker for ComfyUiGetStatusSuccessResponse {}

#[tauri::command]
pub async fn comfyui_get_status_command(
  settings: State<'_, ComfyUiSettings>,
  manager: State<'_, ComfyUiManager>,
) -> ResponseOrErrorMessage<ComfyUiGetStatusSuccessResponse> {
  let hardware = manager.hardware().await;
  let models = CATALOG.iter()
    .map(|model| LocalModelEntry {
      id: model.id.to_string(),
      display_name: model.display_name.to_string(),
      license: model.license.to_string(),
      size_bytes: model.total_size_bytes(),
      state: manager.model_state(model),
      minimum_hardware: model.requirements.minimum_text(),
      recommended_hardware: model.requirements.recommended_text(),
      assessment: assess(hardware, &model.requirements),
    })
    .collect();

  Ok(ComfyUiGetStatusSuccessResponse {
    status: manager.status(),
    is_external: settings.is_external(),
    hardware: HardwareInfo { summary: hardware.summary(), gpu_kind: hardware.gpu_kind },
    models,
    install_folder: manager.layout().root().display().to_string(),
    idle_stop_minutes: IDLE_STOP_AFTER.as_secs() / 60,
  }.into())
}
