use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use crate::core::local_media::local_media_store::{LocalMediaStore, MediaOrigin};
use crate::core::state::task_database::TaskDatabase;
use chrono::{DateTime, Utc};
use log::warn;
use serde_derive::Serialize;
use sqlite_tasks::queries::list_tasks_for_frontend::list_tasks_for_frontend;
use std::collections::HashMap;
use tauri::State;

#[derive(Serialize)]
pub struct LocalMediaListSuccessResponse {
  pub files: Vec<LocalMediaFile>,
}

#[derive(Serialize)]
pub struct LocalMediaFile {
  pub token: String,
  /// A URL the webview can load directly.
  pub url: String,
  /// "image", "video", "audio" or "dimensional".
  pub media_class: &'static str,
  /// "upload" or "inference", the words the library has always used.
  pub origin_category: &'static str,
  pub created_at: DateTime<Utc>,
  pub size_bytes: u64,
  /// The model that made it, when a task remembers.
  pub maybe_model_type: Option<String>,
}

impl SerializeMarker for LocalMediaListSuccessResponse {}

/// Everything saved on this computer, newest first. This is the library.
#[tauri::command]
pub async fn local_media_list_command(
  local_media: State<'_, LocalMediaStore>,
  task_database: State<'_, TaskDatabase>,
) -> ResponseOrErrorMessage<LocalMediaListSuccessResponse> {
  let models = models_by_token(&task_database).await;

  let files = local_media.list().into_iter()
    .filter_map(|stored| {
      let media_class = media_class_for_extension(&stored.extension)?;
      Some(LocalMediaFile {
        url: LocalMediaStore::asset_url_for_path(&stored.path),
        media_class,
        origin_category: match stored.origin {
          MediaOrigin::Upload => "upload",
          MediaOrigin::Generated => "inference",
        },
        created_at: DateTime::<Utc>::from(stored.modified),
        size_bytes: stored.size_bytes,
        maybe_model_type: models.get(stored.token.as_str()).cloned(),
        token: stored.token.to_string(),
      })
    })
    .collect();

  Ok(LocalMediaListSuccessResponse { files }.into())
}

async fn models_by_token(task_database: &TaskDatabase) -> HashMap<String, String> {
  let tasks = match list_tasks_for_frontend(task_database.get_connection()).await {
    Ok(list) => list.tasks,
    Err(err) => {
      warn!("Could not read tasks to name the models: {:?}", err);
      return HashMap::new();
    }
  };
  tasks.into_iter()
    .filter_map(|task| {
      let token = task.on_complete_primary_media_file_token?;
      let model = serde_json::to_value(task.model_type?).ok()?.as_str()?.to_string();
      Some((token.to_string(), model))
    })
    .collect()
}

/// What kind of media a stored file is, from its extension. Files of other kinds are not shown.
fn media_class_for_extension(extension: &str) -> Option<&'static str> {
  match extension {
    "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" => Some("image"),
    "mp4" | "webm" | "mov" | "m4v" => Some("video"),
    "mp3" | "wav" | "ogg" | "flac" | "m4a" | "aac" => Some("audio"),
    "glb" | "gltf" | "obj" | "fbx" => Some("dimensional"),
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn extensions_map_to_kinds() {
    assert_eq!(media_class_for_extension("png"), Some("image"));
    assert_eq!(media_class_for_extension("webm"), Some("video"));
    assert_eq!(media_class_for_extension("mp3"), Some("audio"));
    assert_eq!(media_class_for_extension("wav"), Some("audio"));
    assert_eq!(media_class_for_extension("glb"), Some("dimensional"));
  }

  #[test]
  fn other_files_are_not_media() {
    assert_eq!(media_class_for_extension("bin"), None);
    assert_eq!(media_class_for_extension("txt"), None);
    assert_eq!(media_class_for_extension(""), None);
  }
}
