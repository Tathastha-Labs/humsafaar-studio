//! Voiceover: text to speech with Kokoro-82M (Apache-2.0), run through `kokoro-onnx` in its own
//! small Python environment. No voice cloning; the voices are the fixed ones listed in `VOICES`.

use crate::download::download_verified;
use crate::error::{io_error, ManagerError, ManagerResult};
use crate::install::{install_uv, path_arg, report_download, run_uv, Report, PYTHON_VERSION};
use crate::layout::RuntimeLayout;
use crate::platform::Platform;
use serde_derive::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub const KOKORO_PACKAGE: &str = "kokoro-onnx==0.6.1";
const SOUNDFILE_PACKAGE: &str = "soundfile";

/// Longest script accepted in one go, which is about ten minutes of speech.
pub const MAX_TEXT_CHARS: usize = 8_000;
pub const MIN_SPEED: f32 = 0.7;
pub const MAX_SPEED: f32 = 1.3;

/// A synthesis that takes longer than this is stuck, not slow.
const SYNTH_TIMEOUT: Duration = Duration::from_secs(600);

const SYNTH_SCRIPT: &str = include_str!("tts_synth.py");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Voice {
  pub id: &'static str,
  pub label: &'static str,
  /// `kokoro-onnx` language code, which decides how the text is pronounced.
  pub language: &'static str,
  pub language_label: &'static str,
  pub gender: &'static str,
}

/// A small, deliberate choice: a woman and a man in each of three accents.
pub const VOICES: [Voice; 6] = [
  Voice { id: "af_heart", label: "Heart", language: "en-us", language_label: "English (US)", gender: "female" },
  Voice { id: "am_michael", label: "Michael", language: "en-us", language_label: "English (US)", gender: "male" },
  Voice { id: "bf_emma", label: "Emma", language: "en-gb", language_label: "English (UK)", gender: "female" },
  Voice { id: "bm_george", label: "George", language: "en-gb", language_label: "English (UK)", gender: "male" },
  Voice { id: "hf_alpha", label: "Alpha", language: "hi", language_label: "Hindi", gender: "female" },
  Voice { id: "hm_omega", label: "Omega", language: "hi", language_label: "Hindi", gender: "male" },
];

pub fn find_voice(id: &str) -> Option<&'static Voice> {
  VOICES.iter().find(|voice| voice.id == id)
}

#[derive(Clone, Copy, Debug)]
pub struct TtsFile {
  pub file_name: &'static str,
  pub url: &'static str,
  pub sha256: &'static str,
  pub size_bytes: u64,
}

/// Same bytes as the upstream `kokoro-onnx` release files, served from Hugging Face,
/// which downloads far faster than GitHub's release storage.
pub const TTS_FILES: [TtsFile; 2] = [
  TtsFile {
    file_name: "kokoro-v1.0.onnx",
    url: "https://huggingface.co/fastrtc/kokoro-onnx/resolve/main/kokoro-v1.0.onnx",
    sha256: "7d5df8ecf7d4b1878015a32686053fd0eebe2bc377234608764cc0ef3636a6c5",
    size_bytes: 325_532_387,
  },
  TtsFile {
    file_name: "voices-v1.0.bin",
    url: "https://huggingface.co/fastrtc/kokoro-onnx/resolve/main/voices-v1.0.bin",
    sha256: "bca610b8308e8d99f32e6fe4197e7ec01679264efed0cac9140fe9c29f1fbf7d",
    size_bytes: 28_214_398,
  },
];

/// Runtime plus model, roughly: 127 MB of packages and 354 MB of model files.
pub const TTS_DOWNLOAD_BYTES: u64 = 127_000_000 + 325_532_387 + 28_214_398;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct TtsManifest {
  package: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TtsProgress {
  pub label: &'static str,
  pub detail: Option<String>,
  pub fraction: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spoken {
  pub seconds: f32,
  pub sample_rate: u32,
}

fn model_path(layout: &RuntimeLayout, file: &TtsFile) -> std::path::PathBuf {
  layout.tts_models_dir().join(file.file_name)
}

fn packages_installed(layout: &RuntimeLayout) -> bool {
  let manifest = std::fs::read_to_string(layout.tts_manifest_file()).ok()
    .and_then(|text| serde_json::from_str::<TtsManifest>(&text).ok());
  manifest.is_some_and(|m| m.package == KOKORO_PACKAGE) && layout.tts_python().is_file()
}

fn models_installed(layout: &RuntimeLayout) -> bool {
  TTS_FILES.iter().all(|file| {
    std::fs::metadata(model_path(layout, file)).is_ok_and(|m| m.len() == file.size_bytes)
  })
}

pub fn is_installed(layout: &RuntimeLayout) -> bool {
  packages_installed(layout) && models_installed(layout)
}

/// Install or finish installing voiceover. Parts that are already in place are skipped.
pub async fn install(
  layout: &RuntimeLayout,
  platform: Platform,
  on_progress: &mut (dyn FnMut(TtsProgress) + Send),
) -> ManagerResult<()> {
  platform.check_supported()?;
  tokio::fs::create_dir_all(layout.tts_dir()).await.map_err(|e| io_error(layout.tts_dir(), e))?;

  if !layout.uv_executable().is_file() {
    let mut report = |detail: Option<String>, fraction: Option<f32>| {
      on_progress(TtsProgress { label: "Downloading the installer", detail, fraction });
    };
    install_uv(layout, platform, &mut report).await?;
  }

  if !packages_installed(layout) {
    let mut report = |detail: Option<String>, fraction: Option<f32>| {
      on_progress(TtsProgress { label: "Installing the voice runtime", detail, fraction });
    };
    install_packages(layout, &mut report).await?;
  }

  for file in &TTS_FILES {
    let destination = model_path(layout, file);
    if std::fs::metadata(&destination).is_ok_and(|m| m.len() == file.size_bytes) {
      continue;
    }
    let mut report = |detail: Option<String>, fraction: Option<f32>| {
      on_progress(TtsProgress { label: "Downloading the voice model", detail, fraction });
    };
    download_verified(file.url, &destination, Some(file.sha256), &mut |p| report_download(&mut report, p)).await?;
  }

  let _ = tokio::fs::remove_dir_all(layout.uv_cache_dir()).await;
  Ok(())
}

async fn install_packages(layout: &RuntimeLayout, report: &mut Report<'_>) -> ManagerResult<()> {
  let _ = tokio::fs::remove_dir_all(layout.tts_venv_dir()).await;
  run_uv(layout, "creating the voice environment", &[
    "venv",
    "--python", PYTHON_VERSION,
    "--python-preference", "only-managed",
    &path_arg(&layout.tts_venv_dir()),
  ], report).await?;

  let python = path_arg(&layout.tts_python());
  run_uv(layout, "installing the voice runtime", &[
    "pip", "install", "--python", &python, KOKORO_PACKAGE, SOUNDFILE_PACKAGE,
  ], report).await?;

  let manifest = TtsManifest { package: KOKORO_PACKAGE.to_string() };
  let text = serde_json::to_string_pretty(&manifest).map_err(|e| ManagerError::Other(e.to_string()))?;
  let path = layout.tts_manifest_file();
  tokio::fs::write(&path, text).await.map_err(|e| io_error(&path, e))
}

/// Remove the voice runtime and model. Image models and the engine stay.
pub fn uninstall(layout: &RuntimeLayout) -> ManagerResult<()> {
  match std::fs::remove_dir_all(layout.tts_dir()) {
    Ok(()) => Ok(()),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
    Err(e) => Err(io_error(layout.tts_dir(), e)),
  }
}

/// What the script is told to do, after checking it.
#[derive(Debug, PartialEq)]
pub struct SynthRequest<'a> {
  pub text: &'a str,
  pub voice: &'static Voice,
  pub speed: f32,
}

pub fn check_request<'a>(text: &'a str, voice_id: &str, speed: f32) -> ManagerResult<SynthRequest<'a>> {
  let text = text.trim();
  if text.is_empty() {
    return Err(ManagerError::Other("Type the script to speak first.".to_string()));
  }
  if text.chars().count() > MAX_TEXT_CHARS {
    return Err(ManagerError::Other(format!(
      "That script is too long for one voiceover. Keep it under {} characters, or split it.", MAX_TEXT_CHARS
    )));
  }
  let voice = find_voice(voice_id)
    .ok_or_else(|| ManagerError::Other(format!("There is no voice called {}.", voice_id)))?;
  let speed = if speed.is_finite() { speed.clamp(MIN_SPEED, MAX_SPEED) } else { 1.0 };
  Ok(SynthRequest { text, voice, speed })
}

/// Speak `text` and write it to `output` as a WAV file.
pub async fn synthesize(layout: &RuntimeLayout, request: &SynthRequest<'_>, output: &Path) -> ManagerResult<Spoken> {
  if !is_installed(layout) {
    return Err(ManagerError::NotInstalled);
  }
  if let Some(parent) = output.parent() {
    tokio::fs::create_dir_all(parent).await.map_err(|e| io_error(parent, e))?;
  }
  tokio::fs::write(layout.tts_script(), SYNTH_SCRIPT).await.map_err(|e| io_error(layout.tts_script(), e))?;

  let payload = serde_json::json!({
    "text": request.text,
    "voice": request.voice.id,
    "lang": request.voice.language,
    "speed": request.speed,
    "model": model_path(layout, &TTS_FILES[0]),
    "voices": model_path(layout, &TTS_FILES[1]),
    "out": output,
  });

  let mut command = Command::new(layout.tts_python());
  command
    .arg("-I")
    .arg(layout.tts_script())
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .kill_on_drop(true);
  crate::install::hide_window(&mut command);

  let mut child = command.spawn().map_err(|e| io_error(layout.tts_python(), e))?;
  if let Some(mut stdin) = child.stdin.take() {
    stdin.write_all(payload.to_string().as_bytes()).await.map_err(|e| io_error(layout.tts_python(), e))?;
  }

  let result = tokio::time::timeout(SYNTH_TIMEOUT, child.wait_with_output()).await
    .map_err(|_| ManagerError::Other("The voiceover took too long and was stopped.".to_string()))?
    .map_err(|e| io_error(layout.tts_python(), e))?;

  if !result.status.success() {
    let stderr = String::from_utf8_lossy(&result.stderr);
    let tail: Vec<&str> = stderr.lines().rev().take(8).collect::<Vec<_>>().into_iter().rev().collect();
    return Err(ManagerError::Command { step: "making the voiceover".to_string(), detail: tail.join("\n") });
  }

  let stdout = String::from_utf8_lossy(&result.stdout);
  parse_spoken(stdout.lines().last().unwrap_or(""))
}

fn parse_spoken(line: &str) -> ManagerResult<Spoken> {
  let value: serde_json::Value = serde_json::from_str(line.trim())
    .map_err(|_| ManagerError::Other(format!("The voice runtime gave an unexpected answer: {}", line)))?;
  let seconds = value["seconds"].as_f64().ok_or_else(|| ManagerError::Other("The voice runtime gave no length.".to_string()))?;
  let sample_rate = value["sample_rate"].as_u64().unwrap_or(24_000) as u32;
  Ok(Spoken { seconds: seconds as f32, sample_rate })
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::tempdir;

  #[test]
  fn there_is_a_woman_and_a_man_for_each_accent() {
    for language in ["en-us", "en-gb", "hi"] {
      let genders: Vec<&str> = VOICES.iter().filter(|v| v.language == language).map(|v| v.gender).collect();
      assert_eq!(genders, vec!["female", "male"], "{}", language);
    }
  }

  #[test]
  fn voice_ids_are_unique() {
    let mut ids: Vec<&str> = VOICES.iter().map(|v| v.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), VOICES.len());
  }

  #[test]
  fn model_checksums_look_like_sha256() {
    for file in TTS_FILES {
      assert_eq!(file.sha256.len(), 64, "{}", file.file_name);
      assert!(file.sha256.chars().all(|c| c.is_ascii_hexdigit()));
    }
  }

  #[test]
  fn an_empty_script_is_refused() {
    let err = check_request("   \n", "af_heart", 1.0).unwrap_err();
    assert!(err.to_string().contains("script"));
  }

  #[test]
  fn an_overlong_script_is_refused() {
    let text = "a".repeat(MAX_TEXT_CHARS + 1);
    assert!(check_request(&text, "af_heart", 1.0).unwrap_err().to_string().contains("too long"));
    assert!(check_request(&"a".repeat(MAX_TEXT_CHARS), "af_heart", 1.0).is_ok());
  }

  #[test]
  fn an_unknown_voice_is_refused() {
    assert!(check_request("hello", "xx_nobody", 1.0).unwrap_err().to_string().contains("no voice"));
  }

  #[test]
  fn speed_is_kept_in_range_and_nan_becomes_normal() {
    assert_eq!(check_request("hi", "af_heart", 5.0).unwrap().speed, MAX_SPEED);
    assert_eq!(check_request("hi", "af_heart", 0.1).unwrap().speed, MIN_SPEED);
    assert_eq!(check_request("hi", "af_heart", f32::NAN).unwrap().speed, 1.0);
  }

  #[test]
  fn the_script_is_trimmed() {
    assert_eq!(check_request("  hello  ", "af_heart", 1.0).unwrap().text, "hello");
  }

  #[test]
  fn parses_the_runtime_answer() {
    let spoken = parse_spoken(r#"{"seconds": 3.9, "sample_rate": 24000}"#).unwrap();
    assert_eq!(spoken, Spoken { seconds: 3.9, sample_rate: 24_000 });
    assert!(parse_spoken("Traceback").is_err());
  }

  #[test]
  fn not_installed_until_packages_and_models_are_present() {
    let dir = tempdir().unwrap();
    let layout = RuntimeLayout::new(dir.path());
    assert!(!is_installed(&layout));
  }

  #[tokio::test]
  async fn synthesizing_without_an_install_says_so() {
    let dir = tempdir().unwrap();
    let layout = RuntimeLayout::new(dir.path());
    let request = check_request("hello", "af_heart", 1.0).unwrap();
    let err = synthesize(&layout, &request, &dir.path().join("out.wav")).await.unwrap_err();
    assert!(matches!(err, ManagerError::NotInstalled));
  }

  #[test]
  fn uninstalling_nothing_is_fine() {
    let dir = tempdir().unwrap();
    assert!(uninstall(&RuntimeLayout::new(dir.path())).is_ok());
  }
}
