use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::commands::generate::omni::{OmniRequest, OmniResponse, OmniResult};
use crate::core::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::core::events::generation_events::generation_enqueue_success_event::GenerationEnqueueSuccessEvent;
use crate::core::state::task_database::TaskDatabase;
use log::error;
use serde_json::json;
use tauri::{AppHandle, Manager};

/// A job that a provider has accepted: record it so the pages and the poller can follow it, tell the
/// frontend, and answer in the shape the generate pages expect.
pub async fn record_enqueued_job(request: &OmniRequest, success: TaskEnqueueSuccess, app: &AppHandle) -> OmniResult {
  let database = app.state::<TaskDatabase>();
  if let Err(err) = success
    .insert_into_task_database_with_frontend_payload(
      &database,
      request.frontend_caller,
      request.frontend_subscriber_id.as_deref(),
      request.frontend_subscriber_payload.as_deref(),
    )
    .await
  {
    // The provider already has the job, so do not encourage a duplicate retry.
    error!("Could not record the task: {:?}", err);
  }

  let job_token = success.provider_job_id.clone().unwrap_or_default();
  GenerationEnqueueSuccessEvent {
    action: success.to_frontend_event_action(),
    service: success.to_frontend_event_service(),
    model: success.model,
  }.send_infallible(app);

  Ok(OmniResponse(json!({
    "success": true,
    "inference_job_token": job_token,
    "all_job_tokens": [job_token],
  })).into())
}
