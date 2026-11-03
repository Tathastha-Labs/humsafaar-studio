use crate::core::local_models::comfyui_settings::ComfyUiSettings;
use crate::core::state::task_database::TaskDatabase;
use crate::core::utils::task_database_pending_statuses::TASK_DATABASE_PENDING_STATUSES;
use comfyui_manager::manager::{ComfyUiManager, IDLE_STOP_AFTER};
use enums::common::generation_provider::GenerationProvider;
use log::{info, warn};
use sqlite_tasks::queries::list_non_humsafaar_pending_tasks::{
  list_non_humsafaar_pending_tasks, ListNonHumsafaarPendingTasksArgs,
};
use std::time::Duration;

const CHECK_EVERY: Duration = Duration::from_secs(30);

/// Stops the local engine once it has sat unused for a while, which frees a lot of memory.
/// It starts again by itself the next time a local job needs it. The engine counts as in use
/// while any local job is waiting or running.
pub async fn comfyui_idle_stop_thread(
  manager: ComfyUiManager,
  settings: ComfyUiSettings,
  task_database: TaskDatabase,
) -> ! {
  loop {
    tokio::time::sleep(CHECK_EVERY).await;

    // An engine the user runs themselves is theirs to stop.
    if settings.is_external() {
      continue;
    }

    match has_local_job_in_progress(&task_database).await {
      Ok(true) => manager.touch(),
      Ok(false) => {}
      Err(err) => {
        // Not knowing is not a reason to stop an engine that may be mid-job.
        warn!("[IdleStop] Could not check for local jobs: {:?}", err);
        manager.touch();
        continue;
      }
    }

    if manager.stop_if_idle(IDLE_STOP_AFTER).await {
      info!("[IdleStop] Stopped the local engine after {} minutes unused", IDLE_STOP_AFTER.as_secs() / 60);
    }
  }
}

async fn has_local_job_in_progress(task_database: &TaskDatabase) -> Result<bool, sqlite_tasks::error::SqliteTasksError> {
  let pending = list_non_humsafaar_pending_tasks(ListNonHumsafaarPendingTasksArgs {
    db: task_database.get_connection(),
    task_statuses: &TASK_DATABASE_PENDING_STATUSES,
  }).await?;
  Ok(any_local(pending.tasks.iter().map(|task| task.provider)))
}

fn any_local(providers: impl IntoIterator<Item = GenerationProvider>) -> bool {
  providers.into_iter().any(|provider| provider == GenerationProvider::ComfyUi)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_waiting_local_job_keeps_the_engine_alive() {
    assert!(any_local([GenerationProvider::Fal, GenerationProvider::ComfyUi]));
  }

  #[test]
  fn cloud_jobs_do_not() {
    assert!(!any_local([GenerationProvider::Fal, GenerationProvider::Replicate]));
    assert!(!any_local([]));
  }
}
