use humsafaar_router::api::router_aspect_ratio::RouterAspectRatio;
use humsafaar_router::api::router_resolution::RouterResolution;
use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_type::TaskType;
use gmicloud_client::creds::gmicloud_api_key::GmiCloudApiKey;
use gmicloud_client::requests::common::create_request::{create_gmicloud_request, GmiCloudCreateRequest};
use serde_json::{json, Map, Value};

use crate::core::cloud_models::local_picture::LocalPicture;
use crate::core::cloud_models::replicate_queue::load_api_key;
use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::commands::generate::generate_video::request::{TauriGenerateVideoRequest, TauriVideoModel};
use crate::core::events::generation_events::common::GenerationModel;
use crate::core::local_media::local_media_store::LocalMediaStore;
use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;

/// Seedance 2.0 on GMI Cloud with the user's own key, from a prompt alone or from a start picture.
pub async fn handle_gmi_video(
  request: &TauriGenerateVideoRequest,
  credentials: &ProviderCredentialLoadingCache,
  local_media: &LocalMediaStore,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let model = request.model.ok_or_else(GenerateError::no_model_specified)?;
  let (gmi_model, model_id, fast) = gmi_model(model)
    .ok_or_else(|| GenerateError::NotYetImplemented(format!("{:?} is not a GMI Cloud model", model)))?;

  let prompt = request.prompt.as_deref().map(str::trim).filter(|p| !p.is_empty())
    .ok_or_else(|| GenerateError::AnyhowError(anyhow::anyhow!("A prompt is required")))?;

  let api_key = GmiCloudApiKey::new(load_api_key(credentials, ProviderCredentialKey::GmiCloudApiKey, "GMI Cloud")?);

  let mut payload = Map::new();
  payload.insert("prompt".into(), json!(prompt));
  if let Some(seconds) = request.duration_seconds { payload.insert("duration".into(), json!(seconds.clamp(4, 15))); }
  if let Some(res) = request.resolution.and_then(|r| resolution(r, fast)) { payload.insert("resolution".into(), json!(res)); }
  if let Some(ratio) = request.aspect_ratio.and_then(ratio) { payload.insert("ratio".into(), json!(ratio)); }
  if let Some(audio) = request.generate_audio { payload.insert("generate_audio".into(), json!(audio)); }
  if let Some(token) = &request.start_frame_image_media_token {
    payload.insert("first_frame".into(), json!(LocalPicture::read(token, local_media)?.data_uri()));
  }
  if let Some(token) = &request.end_frame_image_media_token {
    payload.insert("last_frame".into(), json!(LocalPicture::read(token, local_media)?.data_uri()));
  }

  let started = create_gmicloud_request(&api_key, &GmiCloudCreateRequest { model: gmi_model.to_string(), payload: Value::Object(payload) })
    .await
    .map_err(|err| GenerateError::AnyhowError(anyhow::anyhow!("GMI Cloud: {}", err)))?;

  Ok(TaskEnqueueSuccess {
    task_type: TaskType::VideoGeneration,
    model: Some(match fast {
      true => GenerationModel::Seedance2p0Fast,
      false => GenerationModel::Seedance2p0,
    }),
    provider: GenerationProvider::GmiCloud,
    provider_job_id: Some(started.request_id),
    maybe_queue_status_url: None,
    maybe_queue_response_url: None,
    maybe_prompt_token: None,
  })
}

/// GMI Cloud's model name, the app's name for it, and whether it is the Fast one.
fn gmi_model(model: TauriVideoModel) -> Option<(&'static str, &'static str, bool)> {
  match model {
    TauriVideoModel::Seedance2p0 => Some(("seedance-2-0-260128", "seedance_2p0", false)),
    TauriVideoModel::Seedance2p0Fast => Some(("seedance-2-0-fast-260128", "seedance_2p0_fast", true)),
    _ => None,
  }
}

fn ratio(ratio: RouterAspectRatio) -> Option<&'static str> {
  use RouterAspectRatio::*;
  Some(match ratio {
    Square | SquareHd => "1:1",
    WideSixteenByNine | Wide => "16:9",
    TallNineBySixteen | Tall => "9:16",
    WideFourByThree => "4:3",
    TallThreeByFour => "3:4",
    WideTwentyOneByNine => "21:9",
    _ => return None,
  })
}

/// Fast tops out at 720p, so a 1080p ask is lowered rather than refused.
fn resolution(resolution: RouterResolution, fast: bool) -> Option<&'static str> {
  Some(match resolution {
    RouterResolution::FourEightyP => "480p",
    RouterResolution::SevenTwentyP => "720p",
    RouterResolution::TenEightyP if fast => "720p",
    RouterResolution::TenEightyP => "1080p",
    _ => return None,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn only_seedance_models_are_taken() {
    assert_eq!(gmi_model(TauriVideoModel::Seedance2p0Fast).unwrap().0, "seedance-2-0-fast-260128");
    assert!(gmi_model(TauriVideoModel::Kling2p6Pro).is_none());
  }

  #[test]
  fn fast_is_held_to_720p() {
    assert_eq!(resolution(RouterResolution::TenEightyP, true), Some("720p"));
    assert_eq!(resolution(RouterResolution::TenEightyP, false), Some("1080p"));
  }
}
