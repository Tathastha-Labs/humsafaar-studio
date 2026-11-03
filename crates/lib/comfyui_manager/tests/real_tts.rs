//! Real voiceover check: installs the voice runtime and model, then speaks with every voice.
//! It downloads about 480 MB, so it only runs when asked.
//!
//! `COMFYUI_REAL_ROOT=<folder> COMFYUI_OUT=<folder> \
//!   cargo test -p comfyui_manager --test real_tts -- --ignored --nocapture`

use comfyui_manager::layout::RuntimeLayout;
use comfyui_manager::platform::Platform;
use comfyui_manager::tts::{check_request, install, is_installed, synthesize, VOICES};
use std::path::PathBuf;
use std::time::Instant;

const ENGLISH: &str = "Welcome to the launch. Today we are showing how a short script becomes a voiceover.";
const HINDI: &str = "नमस्ते, यह एक छोटा सा वॉइसओवर परीक्षण है।";

fn env(name: &str) -> String {
  std::env::var(name).unwrap_or_else(|_| panic!("set {}", name))
}

#[tokio::test]
#[ignore]
async fn install_then_speak_with_every_voice() {
  let layout = RuntimeLayout::new(env("COMFYUI_REAL_ROOT"));
  let out = PathBuf::from(env("COMFYUI_OUT"));

  println!("STEP installed before: {}", is_installed(&layout));
  let started = Instant::now();
  let mut last = String::new();
  install(&layout, Platform::current(), &mut |p| {
    if p.label != last {
      println!("STEP {} ({:.0}s)", p.label, started.elapsed().as_secs_f32());
      last = p.label.to_string();
    }
  }).await.expect("install voiceover");
  assert!(is_installed(&layout));
  println!("STEP installed in {:.0}s", started.elapsed().as_secs_f32());

  for voice in VOICES {
    let text = if voice.language == "hi" { HINDI } else { ENGLISH };
    let request = check_request(text, voice.id, 1.0).unwrap();
    let file = out.join(format!("voiceover-{}.wav", voice.id));
    let began = Instant::now();
    let spoken = synthesize(&layout, &request, &file).await.expect("speak");
    println!("STEP {} spoke {:.1}s of audio in {:.1}s -> {}", voice.id, spoken.seconds, began.elapsed().as_secs_f32(), file.display());
    assert!(spoken.seconds > 1.0);
    assert!(std::fs::metadata(&file).unwrap().len() > 10_000);
  }
}
