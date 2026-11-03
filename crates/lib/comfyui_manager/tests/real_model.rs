//! Real-world checks for one downloadable model at a time. They download gigabytes or need a
//! running engine, so they only run when asked.
//!
//! Download:  `COMFYUI_REAL_ROOT=<folder> COMFYUI_MODEL_ID=stable_diffusion_xl \
//!             cargo test -p comfyui_manager --test real_model download -- --ignored --nocapture`
//! Generate:  `COMFYUI_URL=http://127.0.0.1:<port> COMFYUI_CHECKPOINT=<file> COMFYUI_SIZE=1024 \
//!             COMFYUI_STEPS=25 COMFYUI_OUT=<file.png> \
//!             cargo test -p comfyui_manager --test real_model generate -- --ignored --nocapture`

use comfyui_client::client::ComfyUiClient;
use comfyui_client::workflows::{text_to_image, TextToImageParams};
use comfyui_manager::layout::RuntimeLayout;
use comfyui_manager::manager::{ComfyUiManager, ModelState};
use comfyui_manager::models::catalog_model;
use std::time::{Duration, Instant};

fn env(name: &str) -> String {
  std::env::var(name).unwrap_or_else(|_| panic!("set {}", name))
}

#[tokio::test]
#[ignore]
async fn download_a_catalog_model() {
  let manager = ComfyUiManager::new(RuntimeLayout::new(env("COMFYUI_REAL_ROOT")));
  let model = *catalog_model(&env("COMFYUI_MODEL_ID")).expect("a known catalog model id");

  println!("STEP state before: {:?}", manager.model_state(&model));
  let started = Instant::now();
  manager.download_model(model).expect("start the download");

  let mut last_print = Instant::now() - Duration::from_secs(10);
  loop {
    match manager.model_state(&model) {
      ModelState::Installed => break,
      ModelState::Failed { message } => panic!("download failed: {}", message),
      ModelState::Downloading { downloaded_bytes, total_bytes } => {
        if last_print.elapsed() >= Duration::from_secs(10) {
          let total = total_bytes.unwrap_or(model.total_size_bytes());
          println!("PROGRESS {:.1}% ({:.2} of {:.2} GB)", downloaded_bytes as f64 * 100.0 / total as f64,
            downloaded_bytes as f64 / 1e9, total as f64 / 1e9);
          last_print = Instant::now();
        }
      }
      ModelState::NotInstalled => {}
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
  }
  println!("STEP downloaded and checksum-verified in {:.0}s", started.elapsed().as_secs_f64());
  assert_eq!(manager.model_state(&model), ModelState::Installed);
}

#[tokio::test]
#[ignore]
async fn generate_with_a_running_engine() {
  let client = ComfyUiClient::new(&env("COMFYUI_URL"));
  let checkpoint = env("COMFYUI_CHECKPOINT");
  let size: u32 = env("COMFYUI_SIZE").parse().unwrap();
  let steps: u32 = env("COMFYUI_STEPS").parse().unwrap();

  let available = client.list_checkpoints().await.expect("list checkpoints");
  println!("STEP the engine lists: {:?}", available);
  assert!(available.contains(&checkpoint), "the running engine does not see {}", checkpoint);

  let mut params = TextToImageParams::new(&checkpoint, "a lighthouse on a cliff at sunrise, dramatic clouds, detailed oil painting", 21);
  params.width = size;
  params.height = size;
  params.steps = steps;
  params.negative_prompt = "blurry, low quality, distorted".into();

  let started = Instant::now();
  let id = client.queue_prompt(&text_to_image(&params)).await.expect("queue");
  let images = client.wait_for_outputs(&id, Duration::from_secs(900)).await.expect("outputs");
  let bytes = client.download_image(&images[0]).await.expect("download");
  assert_eq!(&bytes[1..4], b"PNG");
  std::fs::write(env("COMFYUI_OUT"), &bytes).unwrap();
  println!("STEP generated {}x{} in {:.0}s -> {} bytes", size, size, started.elapsed().as_secs_f64(), bytes.len());
}

/// Wan 2.2 video from a running engine that has the model. Set COMFYUI_START_IMAGE to a PNG to
/// animate that picture; leave it unset for video from the prompt alone.
///
/// `COMFYUI_URL=http://127.0.0.1:<port> COMFYUI_WIDTH=640 COMFYUI_HEIGHT=352 COMFYUI_FRAMES=25 \
///  COMFYUI_STEPS=12 COMFYUI_OUT=<file.mp4> [COMFYUI_START_IMAGE=<file.png>] \
///  cargo test -p comfyui_manager --test real_model video -- --ignored --nocapture`
#[tokio::test]
#[ignore]
async fn generate_video_with_a_running_engine() {
  use comfyui_client::workflows::{wan_video, WanVideoParams};

  let client = ComfyUiClient::new(&env("COMFYUI_URL"));
  let mut params = WanVideoParams::new(
    "ocean waves crash against the cliff, clouds drift across the sky, cinematic", 5);
  params.width = env("COMFYUI_WIDTH").parse().unwrap();
  params.height = env("COMFYUI_HEIGHT").parse().unwrap();
  params.frames = env("COMFYUI_FRAMES").parse().unwrap();
  params.steps = env("COMFYUI_STEPS").parse().unwrap();

  if let Ok(path) = std::env::var("COMFYUI_NEGATIVE_FILE") {
    params.negative_prompt = std::fs::read_to_string(path).expect("read the negative prompt").trim().to_string();
  }

  if let Ok(path) = std::env::var("COMFYUI_START_IMAGE") {
    let bytes = std::fs::read(&path).expect("read the start image");
    let name = client.upload_image(bytes, "humsafaar_start.png").await.expect("upload");
    println!("STEP uploaded the start image as {}", name);
    params.start_image = Some(name);
  }

  let started = Instant::now();
  let id = client.queue_prompt(&wan_video(&params)).await.expect("queue");
  println!("STEP queued {} ({}x{}, {} frames, {} steps)", id, params.width, params.height, params.frames, params.steps);

  let outputs = client.wait_for_outputs(&id, Duration::from_secs(3600)).await.expect("outputs");
  let bytes = client.download_image(&outputs[0]).await.expect("download");
  assert_eq!(&bytes[4..8], b"ftyp", "an MP4 starts with an ftyp box");
  std::fs::write(env("COMFYUI_OUT"), &bytes).unwrap();
  println!("STEP generated {} bytes of video in {:.0}s", bytes.len(), started.elapsed().as_secs_f64());
}

/// ACE-Step music from a running engine that has the model.
///
/// `COMFYUI_URL=http://127.0.0.1:<port> COMFYUI_SECONDS=30 COMFYUI_OUT=<file.mp3> \
///  cargo test -p comfyui_manager --test real_model music -- --ignored --nocapture`
#[tokio::test]
#[ignore]
async fn generate_music_with_a_running_engine() {
  use comfyui_client::workflows::{ace_step_music, AceStepParams};

  let client = ComfyUiClient::new(&env("COMFYUI_URL"));
  let mut params = AceStepParams::new(
    "warm acoustic folk, gentle guitar, soft vocals, calm, hopeful",
    "[Verse 1]\nMorning light on the quiet hill\nThe river hums and the world is still\n\n[Chorus]\nWe walk on, we walk on\nInto the golden dawn",
    9,
  );
  params.seconds = env("COMFYUI_SECONDS").parse().unwrap();

  let started = Instant::now();
  let id = client.queue_prompt(&ace_step_music(&params)).await.expect("queue");
  println!("STEP queued {} ({} s of music, {} steps)", id, params.seconds, params.steps);

  let outputs = client.wait_for_outputs(&id, Duration::from_secs(1800)).await.expect("outputs");
  let bytes = client.download_image(&outputs[0]).await.expect("download");
  let looks_like_mp3 = bytes.starts_with(b"ID3") || (bytes.len() > 2 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0);
  assert!(looks_like_mp3, "not an MP3: starts with {:02x?}", &bytes[..4.min(bytes.len())]);
  std::fs::write(env("COMFYUI_OUT"), &bytes).unwrap();
  println!("STEP generated {} bytes of audio in {:.0}s", bytes.len(), started.elapsed().as_secs_f64());
}
