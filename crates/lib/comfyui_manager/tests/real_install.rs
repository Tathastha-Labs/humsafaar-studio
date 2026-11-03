//! Installs the real engine, starts it, generates an image and stops it.
//! Downloads several gigabytes, so it only runs when asked:
//!
//! `COMFYUI_REAL_ROOT=/some/folder cargo test -p comfyui_manager --test real_install -- --ignored --nocapture`

use comfyui_client::client::ComfyUiClient;
use comfyui_client::workflows::{text_to_image, TextToImageParams};
use comfyui_manager::layout::RuntimeLayout;
use comfyui_manager::manager::{ComfyUiManager, EngineStatus};
use comfyui_manager::models::{list_checkpoints, STABLE_DIFFUSION_1_5};
use std::time::Duration;

#[tokio::test]
#[ignore]
async fn installs_starts_generates_and_stops() {
  let root = std::env::var("COMFYUI_REAL_ROOT").expect("set COMFYUI_REAL_ROOT to an empty or existing install folder");
  let manager = ComfyUiManager::new(RuntimeLayout::new(&root));

  // Print each distinct status as it happens.
  let mut receiver = manager.subscribe();
  let printer = tokio::spawn(async move {
    let mut last = String::new();
    while receiver.changed().await.is_ok() {
      let line = match &*receiver.borrow_and_update() {
        EngineStatus::Installing { label, step_index, step_count, detail, fraction } => format!(
          "[{}/{}] {} {} {}", step_index + 1, step_count, label,
          fraction.map(|f| format!("{:.0}%", f * 100.0)).unwrap_or_default(),
          detail.clone().unwrap_or_default()),
        other => format!("{:?}", other),
      };
      // Progress lines change constantly; only show a line when its step or state changes.
      let key: String = line.chars().take(40).collect();
      if key != last {
        println!("STATUS {}", line);
        last = key;
      }
    }
  });

  println!("STEP install (installed already: {})", manager.is_installed());
  manager.install(&[STABLE_DIFFUSION_1_5]).await.expect("install");
  assert!(manager.is_installed());
  assert!(list_checkpoints(manager.layout()).unwrap().contains(&STABLE_DIFFUSION_1_5.primary_file().file_name.to_string()));

  println!("STEP start");
  let url = manager.start().await.expect("start");
  println!("STEP running at {}", url);
  let client = ComfyUiClient::new(&url);
  assert!(client.is_healthy().await);

  println!("STEP generate");
  let checkpoint = client.list_checkpoints().await.expect("checkpoints").into_iter().next().expect("a checkpoint");
  let mut params = TextToImageParams::new(&checkpoint, "a lighthouse at dawn, watercolor", 11);
  params.steps = 6;
  let id = client.queue_prompt(&text_to_image(&params)).await.expect("queue");
  let images = client.wait_for_outputs(&id, Duration::from_secs(600)).await.expect("outputs");
  let bytes = client.download_image(&images[0]).await.expect("download");
  assert_eq!(&bytes[1..4], b"PNG");
  println!("STEP generated {} bytes", bytes.len());

  println!("STEP stop");
  manager.stop().await;
  assert!(!client.is_healthy().await);
  println!("STEP done");

  printer.abort();
}
