use crate::download::{download_verified, extract_archive, Progress};
use crate::error::{io_error, ManagerError, ManagerResult};
use crate::layout::RuntimeLayout;
use crate::platform::{comfyui_source_url, Platform, TorchFlavor, COMFYUI_VERSION, UV_VERSION};
use serde_derive::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub(crate) const PYTHON_VERSION: &str = "3.12";

/// How many trailing output lines to keep for an error message.
const ERROR_TAIL_LINES: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallStep {
  Uv,
  Python,
  Torch,
  ComfyUi,
  Requirements,
}

impl InstallStep {
  pub const ALL: [InstallStep; 5] = [
    InstallStep::Uv,
    InstallStep::Python,
    InstallStep::Torch,
    InstallStep::ComfyUi,
    InstallStep::Requirements,
  ];

  pub fn label(&self) -> &'static str {
    match self {
      InstallStep::Uv => "Downloading the installer",
      InstallStep::Python => "Setting up Python",
      InstallStep::Torch => "Installing PyTorch",
      InstallStep::ComfyUi => "Downloading ComfyUI",
      InstallStep::Requirements => "Installing ComfyUI's packages",
    }
  }

  pub fn index(&self) -> usize {
    Self::ALL.iter().position(|s| s == self).unwrap_or(0)
  }
}

/// What the install has finished, so an interrupted install can continue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallManifest {
  pub uv_version: String,
  pub comfyui_version: String,
  pub torch_flavor: TorchFlavorRecord,
  pub completed: Vec<InstallStep>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TorchFlavorRecord {
  Default,
  Cuda,
  Cpu,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InstallProgress {
  pub step: InstallStep,
  pub step_index: usize,
  pub step_count: usize,
  /// The latest line of output, or a byte count while downloading.
  pub detail: Option<String>,
  /// 0.0 to 1.0 when known.
  pub fraction: Option<f32>,
}

impl InstallManifest {
  fn new(flavor: TorchFlavor) -> Self {
    Self {
      uv_version: UV_VERSION.to_string(),
      comfyui_version: COMFYUI_VERSION.to_string(),
      torch_flavor: flavor.into(),
      completed: Vec::new(),
    }
  }

  pub fn load(layout: &RuntimeLayout) -> Option<Self> {
    let text = std::fs::read_to_string(layout.manifest_file()).ok()?;
    serde_json::from_str(&text).ok()
  }

  fn save(&self, layout: &RuntimeLayout) -> ManagerResult<()> {
    let path = layout.manifest_file();
    if let Some(parent) = path.parent() {
      std::fs::create_dir_all(parent).map_err(|e| io_error(parent, e))?;
    }
    let text = serde_json::to_string_pretty(self).map_err(|e| ManagerError::Other(e.to_string()))?;
    std::fs::write(&path, text).map_err(|e| io_error(&path, e))
  }

  #[cfg(test)]
  pub(crate) fn save_for_tests(&self, layout: &RuntimeLayout) {
    self.save(layout).unwrap();
  }

  /// A step still counts only if it was done for the versions we pin today.
  fn is_done(&self, step: InstallStep) -> bool {
    if !self.completed.contains(&step) {
      return false;
    }
    match step {
      InstallStep::Uv => self.uv_version == UV_VERSION,
      InstallStep::ComfyUi | InstallStep::Requirements => self.comfyui_version == COMFYUI_VERSION,
      InstallStep::Python | InstallStep::Torch => true,
    }
  }

  pub fn is_complete(&self) -> bool {
    InstallStep::ALL.iter().all(|step| self.is_done(*step))
  }
}

/// True once every step has finished and the files are still on disk.
pub fn is_installed(layout: &RuntimeLayout) -> bool {
  InstallManifest::load(layout).is_some_and(|m| m.is_complete())
    && layout.main_script().is_file()
    && layout.venv_python().is_file()
    && layout.uv_executable().is_file()
}

/// Install or finish installing the engine. Steps that already completed are skipped.
pub async fn install(
  layout: &RuntimeLayout,
  platform: Platform,
  has_nvidia_gpu: bool,
  on_progress: &mut (dyn FnMut(InstallProgress) + Send),
) -> ManagerResult<()> {
  platform.check_supported()?;
  let flavor = platform.torch_flavor(has_nvidia_gpu);

  // A different GPU setup than last time means PyTorch must be installed again.
  let mut manifest = match InstallManifest::load(layout) {
    Some(m) if m.torch_flavor == flavor.into() => m,
    Some(mut m) => {
      m.torch_flavor = flavor.into();
      m.completed.retain(|s| *s != InstallStep::Torch && *s != InstallStep::Requirements);
      m
    }
    None => InstallManifest::new(flavor),
  };

  for step in InstallStep::ALL {
    if manifest.is_done(step) {
      continue;
    }
    let mut report = |detail: Option<String>, fraction: Option<f32>| {
      on_progress(InstallProgress {
        step,
        step_index: step.index(),
        step_count: InstallStep::ALL.len(),
        detail,
        fraction,
      });
    };
    report(None, None);

    match step {
      InstallStep::Uv => install_uv(layout, platform, &mut report).await?,
      InstallStep::Python => create_venv(layout, &mut report).await?,
      InstallStep::Torch => install_torch(layout, flavor, &mut report).await?,
      InstallStep::ComfyUi => install_comfyui_source(layout, &mut report).await?,
      InstallStep::Requirements => install_requirements(layout, &mut report).await?,
    }

    manifest.completed.retain(|s| *s != step);
    manifest.completed.push(step);
    manifest.uv_version = UV_VERSION.to_string();
    manifest.comfyui_version = COMFYUI_VERSION.to_string();
    manifest.save(layout)?;
  }

  // The package cache only speeds up reinstalling and would keep a second copy of PyTorch.
  let _ = tokio::fs::remove_dir_all(layout.uv_cache_dir()).await;
  Ok(())
}

/// Whether an NVIDIA GPU with a working driver is present.
pub async fn detect_nvidia_gpu() -> bool {
  let run = Command::new("nvidia-smi").arg("-L")
    .stdin(Stdio::null()).stderr(Stdio::null())
    .output();
  match tokio::time::timeout(Duration::from_secs(5), run).await {
    Ok(Ok(output)) => output.status.success() && String::from_utf8_lossy(&output.stdout).contains("GPU"),
    _ => false,
  }
}

// ── Steps ──

pub(crate) type Report<'a> = dyn FnMut(Option<String>, Option<f32>) + Send + 'a;

pub(crate) async fn install_uv(layout: &RuntimeLayout, platform: Platform, report: &mut Report<'_>) -> ManagerResult<()> {
  let asset = platform.uv_asset()?;
  let archive = layout.downloads_dir().join(asset.file_name);
  let extracted = layout.downloads_dir().join("uv-extracted");

  download_verified(&asset.url(), &archive, Some(asset.sha256), &mut |p| report_download(report, p)).await?;

  let _ = tokio::fs::remove_dir_all(&extracted).await;
  extract_archive(asset.archive, &archive, &extracted, 0).await?;

  let executable = layout.uv_executable();
  if let Some(parent) = executable.parent() {
    tokio::fs::create_dir_all(parent).await.map_err(|e| io_error(parent, e))?;
  }
  let source = extracted.join(asset.executable_path_in_archive());
  tokio::fs::copy(&source, &executable).await.map_err(|e| io_error(&source, e))?;
  make_executable(&executable)?;

  let _ = tokio::fs::remove_dir_all(&extracted).await;
  let _ = tokio::fs::remove_file(&archive).await;
  Ok(())
}

async fn create_venv(layout: &RuntimeLayout, report: &mut Report<'_>) -> ManagerResult<()> {
  let _ = tokio::fs::remove_dir_all(layout.venv_dir()).await;
  run_uv(layout, "creating the Python environment", &[
    "venv",
    "--python", PYTHON_VERSION,
    "--python-preference", "only-managed",
    &path_arg(&layout.venv_dir()),
  ], report).await
}

async fn install_torch(layout: &RuntimeLayout, flavor: TorchFlavor, report: &mut Report<'_>) -> ManagerResult<()> {
  let torch = flavor.install();
  let python = path_arg(&layout.venv_python());
  let mut args: Vec<&str> = vec!["pip", "install", "--python", &python];
  args.extend(torch.packages);
  if let Some(index) = torch.index_url {
    args.extend(["--index-url", index]);
  }
  run_uv(layout, "installing PyTorch", &args, report).await
}

async fn install_comfyui_source(layout: &RuntimeLayout, report: &mut Report<'_>) -> ManagerResult<()> {
  let archive = layout.downloads_dir().join(format!("ComfyUI-{}.tar.gz", COMFYUI_VERSION));
  // GitHub does not promise byte-identical source archives, so this one is pinned by tag only.
  download_verified(&comfyui_source_url(), &archive, None, &mut |p| report_download(report, p)).await?;

  let _ = tokio::fs::remove_dir_all(layout.app_dir()).await;
  extract_archive(crate::platform::ArchiveKind::TarGz, &archive, &layout.app_dir(), 1).await?;
  let _ = tokio::fs::remove_file(&archive).await;

  if !layout.main_script().is_file() {
    return Err(ManagerError::Archive("the ComfyUI download has no main.py".to_string()));
  }
  Ok(())
}

async fn install_requirements(layout: &RuntimeLayout, report: &mut Report<'_>) -> ManagerResult<()> {
  let python = path_arg(&layout.venv_python());
  let requirements = path_arg(&layout.requirements_file());
  run_uv(layout, "installing ComfyUI's packages", &[
    "pip", "install", "--python", &python, "-r", &requirements,
  ], report).await
}

// ── Helpers ──

/// Run `uv`, streaming its output as progress and keeping the tail for errors.
pub(crate) async fn run_uv(layout: &RuntimeLayout, what: &str, args: &[&str], report: &mut Report<'_>) -> ManagerResult<()> {
  let mut command = Command::new(layout.uv_executable());
  command
    .args(args)
    .env("UV_PYTHON_INSTALL_DIR", layout.python_install_dir())
    .env("UV_CACHE_DIR", layout.uv_cache_dir())
    .env("UV_NO_CONFIG", "1")
    .env("UV_HTTP_TIMEOUT", "180")
    .env("UV_HTTP_RETRIES", "5")
    .env("NO_COLOR", "1")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .kill_on_drop(true);
  hide_window(&mut command);

  let mut child = command.spawn().map_err(|e| io_error(layout.uv_executable(), e))?;
  let stdout = child.stdout.take();
  let stderr = child.stderr.take();

  let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
  for stream in [stdout.map(box_reader), stderr.map(box_reader)].into_iter().flatten() {
    let tx = tx.clone();
    tokio::spawn(async move {
      let mut lines = BufReader::new(stream).lines();
      while let Ok(Some(line)) = lines.next_line().await {
        let _ = tx.send(line);
      }
    });
  }
  drop(tx);

  let mut tail: Vec<String> = Vec::new();
  while let Some(line) = rx.recv().await {
    let line = line.trim().to_string();
    if line.is_empty() {
      continue;
    }
    report(Some(line.clone()), None);
    tail.push(line);
    if tail.len() > ERROR_TAIL_LINES {
      tail.remove(0);
    }
  }

  let status = child.wait().await.map_err(|e| io_error(layout.uv_executable(), e))?;
  if status.success() {
    Ok(())
  } else {
    Err(ManagerError::Command { step: what.to_string(), detail: tail.join("\n") })
  }
}

fn box_reader<R: tokio::io::AsyncRead + Unpin + Send + 'static>(reader: R) -> Box<dyn tokio::io::AsyncRead + Unpin + Send> {
  Box::new(reader)
}

pub(crate) fn report_download(report: &mut Report<'_>, progress: Progress) {
  let fraction = progress.total.filter(|t| *t > 0).map(|t| progress.downloaded as f32 / t as f32);
  let detail = match progress.total {
    Some(total) => format!("{} of {}", megabytes(progress.downloaded), megabytes(total)),
    None => megabytes(progress.downloaded),
  };
  report(Some(detail), fraction);
}

fn megabytes(bytes: u64) -> String {
  format!("{:.1} MB", bytes as f64 / 1_000_000.0)
}

pub(crate) fn path_arg(path: &Path) -> String {
  path.to_string_lossy().into_owned()
}

#[cfg(unix)]
fn make_executable(path: &Path) -> ManagerResult<()> {
  use std::os::unix::fs::PermissionsExt;
  std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).map_err(|e| io_error(path, e))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> ManagerResult<()> {
  Ok(())
}

#[cfg(windows)]
pub(crate) fn hide_window(command: &mut Command) {
  const CREATE_NO_WINDOW: u32 = 0x0800_0000;
  command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
pub(crate) fn hide_window(_command: &mut Command) {}

impl From<TorchFlavor> for TorchFlavorRecord {
  fn from(flavor: TorchFlavor) -> Self {
    match flavor {
      TorchFlavor::Default => TorchFlavorRecord::Default,
      TorchFlavor::Cuda => TorchFlavorRecord::Cuda,
      TorchFlavor::Cpu => TorchFlavorRecord::Cpu,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::tempdir;

  mod manifest {
    use super::*;

    #[test]
    fn a_fresh_manifest_is_not_complete() {
      let manifest = InstallManifest::new(TorchFlavor::Cpu);
      assert!(!manifest.is_complete());
      assert!(!manifest.is_done(InstallStep::Uv));
    }

    #[test]
    fn all_steps_done_means_complete() {
      let mut manifest = InstallManifest::new(TorchFlavor::Default);
      manifest.completed = InstallStep::ALL.to_vec();
      assert!(manifest.is_complete());
    }

    #[test]
    fn a_step_done_for_an_old_pin_no_longer_counts() {
      let mut manifest = InstallManifest::new(TorchFlavor::Default);
      manifest.completed = InstallStep::ALL.to_vec();
      manifest.comfyui_version = "v0.0.1".to_string();
      assert!(!manifest.is_done(InstallStep::ComfyUi));
      assert!(!manifest.is_done(InstallStep::Requirements));
      assert!(manifest.is_done(InstallStep::Torch));
      assert!(!manifest.is_complete());
    }

    #[test]
    fn round_trips_through_the_file() {
      let dir = tempdir().unwrap();
      let layout = RuntimeLayout::new(dir.path());
      let mut manifest = InstallManifest::new(TorchFlavor::Cuda);
      manifest.completed = vec![InstallStep::Uv, InstallStep::Python];
      manifest.save(&layout).unwrap();
      assert_eq!(InstallManifest::load(&layout), Some(manifest));
    }

    #[test]
    fn a_missing_or_corrupt_manifest_loads_as_none() {
      let dir = tempdir().unwrap();
      let layout = RuntimeLayout::new(dir.path());
      assert_eq!(InstallManifest::load(&layout), None);
      std::fs::write(layout.manifest_file(), "{not json").unwrap();
      assert_eq!(InstallManifest::load(&layout), None);
    }
  }

  mod is_installed {
    use super::*;

    #[test]
    fn requires_the_files_as_well_as_the_manifest() {
      let dir = tempdir().unwrap();
      let layout = RuntimeLayout::new(dir.path());
      let mut manifest = InstallManifest::new(TorchFlavor::Default);
      manifest.completed = InstallStep::ALL.to_vec();
      manifest.save(&layout).unwrap();
      assert!(!is_installed(&layout), "files are missing");

      for file in [layout.main_script(), layout.venv_python(), layout.uv_executable()] {
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, "").unwrap();
      }
      assert!(is_installed(&layout));
    }
  }

  #[test]
  fn steps_are_ordered_and_labelled() {
    assert_eq!(InstallStep::Uv.index(), 0);
    assert_eq!(InstallStep::Requirements.index(), InstallStep::ALL.len() - 1);
    assert!(InstallStep::ALL.iter().all(|s| !s.label().is_empty()));
  }

  #[test]
  fn download_progress_is_reported_in_megabytes() {
    let mut seen: Vec<(Option<String>, Option<f32>)> = Vec::new();
    let mut report = |d: Option<String>, f: Option<f32>| seen.push((d, f));
    report_download(&mut report, Progress { downloaded: 500_000, total: Some(2_000_000) });
    assert_eq!(seen[0], (Some("0.5 MB of 2.0 MB".to_string()), Some(0.25)));
  }
}
