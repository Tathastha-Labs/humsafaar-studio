use crate::download::Progress;
use crate::hardware::HardwareProfile;
use crate::error::{ManagerError, ManagerResult};
use crate::install::{self, InstallProgress, InstallStep};
use crate::layout::RuntimeLayout;
use crate::models::{ensure_model, is_model_installed, remove_model, CatalogModel};
#[cfg(test)]
use crate::models::ModelFile;
use crate::platform::Platform;
use crate::process::{log_tail, pick_free_port, reap_stale_engine, remove_pid_file, spawn_engine};
use comfyui_client::client::ComfyUiClient;
use serde_derive::Serialize;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::process::Child;
use tokio::sync::{oneshot, watch};
use tokio::task::JoinHandle;

/// Loading PyTorch and the first model can take a while on a cold start.
const DEFAULT_READY_TIMEOUT: Duration = Duration::from_secs(180);
const POLL_INTERVAL: Duration = Duration::from_millis(300);
const STOP_TIMEOUT: Duration = Duration::from_secs(10);

/// An engine that keeps dying is not restarted forever.
const MAX_RESTARTS: usize = 3;
const RESTART_WINDOW: Duration = Duration::from_secs(300);

/// Keep the UI from being flooded with download progress.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);

const LOG_LINES_IN_ERRORS: usize = 8;

/// How long the engine may sit unused before the app stops it to free memory.
pub const IDLE_STOP_AFTER: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum EngineStatus {
  /// This computer cannot run it (for example an Intel Mac).
  Unsupported { reason: String },
  NotInstalled,
  Installing {
    label: String,
    step_index: usize,
    step_count: usize,
    detail: Option<String>,
    fraction: Option<f32>,
  },
  Stopped,
  Starting,
  Running { base_url: String },
  Failed { message: String },
}

/// Where a downloadable model stands.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ModelState {
  NotInstalled,
  Downloading { downloaded_bytes: u64, total_bytes: Option<u64> },
  Installed,
  Failed { message: String },
}

/// Installs, starts, watches and stops the local ComfyUI engine.
/// Cheap to clone; every clone controls the same engine.
#[derive(Clone)]
pub struct ComfyUiManager {
  inner: Arc<Inner>,
}

struct Inner {
  layout: RuntimeLayout,
  platform: Platform,
  ready_timeout: Duration,
  status: Mutex<EngineStatus>,
  status_sender: watch::Sender<EngineStatus>,
  /// Looked up once, the first time it is asked for.
  hardware: tokio::sync::OnceCell<HardwareProfile>,
  /// Models being downloaded, or whose download failed. Finished ones are found on disk.
  downloads: Mutex<HashMap<&'static str, ModelState>>,
  /// Held for the whole of an install, start or stop, so they never overlap.
  lifecycle: tokio::sync::Mutex<Lifecycle>,
  /// When the engine was last started or asked for, so an unused one can be stopped.
  last_activity: Mutex<Instant>,
}

#[derive(Default)]
struct Lifecycle {
  running: Option<Running>,
}

struct Running {
  stop: oneshot::Sender<()>,
  supervisor: JoinHandle<()>,
}

impl ComfyUiManager {
  pub fn new(layout: RuntimeLayout) -> Self {
    Self::with_platform(layout, Platform::current(), DEFAULT_READY_TIMEOUT)
  }

  pub fn with_platform(layout: RuntimeLayout, platform: Platform, ready_timeout: Duration) -> Self {
    let initial = initial_status(&layout, platform);
    let (status_sender, _) = watch::channel(initial.clone());
    Self {
      inner: Arc::new(Inner {
        layout,
        platform,
        ready_timeout,
        status: Mutex::new(initial),
        status_sender,
        hardware: tokio::sync::OnceCell::new(),
        downloads: Mutex::new(HashMap::new()),
        lifecycle: tokio::sync::Mutex::new(Lifecycle::default()),
        last_activity: Mutex::new(Instant::now()),
      }),
    }
  }

  pub fn layout(&self) -> &RuntimeLayout {
    &self.inner.layout
  }

  pub fn status(&self) -> EngineStatus {
    let current = self.inner.status.lock().expect("status lock").clone();

    // An install can appear on disk without this manager doing it (a restored folder, for example).
    if current == EngineStatus::NotInstalled && install::is_installed(&self.inner.layout) {
      self.inner.set_status(EngineStatus::Stopped);
      return EngineStatus::Stopped;
    }
    current
  }

  /// Notified on every status change.
  pub fn subscribe(&self) -> watch::Receiver<EngineStatus> {
    self.inner.status_sender.subscribe()
  }

  /// The address to talk to, while the engine is running.
  pub fn base_url(&self) -> Option<String> {
    match self.status() {
      EngineStatus::Running { base_url } => Some(base_url),
      _ => None,
    }
  }

  /// What this computer has to run models with.
  pub async fn hardware(&self) -> &HardwareProfile {
    self.inner.hardware
      .get_or_init(|| HardwareProfile::detect(self.inner.platform))
      .await
  }

  pub fn is_installed(&self) -> bool {
    install::is_installed(&self.inner.layout)
  }

  pub fn model_state(&self, model: &CatalogModel) -> ModelState {
    if let Some(state) = self.inner.downloads.lock().expect("downloads lock").get(model.id) {
      return state.clone();
    }
    if is_model_installed(&self.inner.layout, model) {
      ModelState::Installed
    } else {
      ModelState::NotInstalled
    }
  }

  /// Download a model in the background. Follow it with `model_state`. The engine does not
  /// need to be installed or running.
  pub fn download_model(&self, model: CatalogModel) -> ManagerResult<()> {
    {
      let mut downloads = self.inner.downloads.lock().expect("downloads lock");
      if matches!(downloads.get(model.id), Some(ModelState::Downloading { .. })) {
        return Err(ManagerError::Other(format!("{} is already downloading", model.display_name)));
      }
      if is_model_installed(&self.inner.layout, &model) {
        return Ok(());
      }
      downloads.insert(model.id, ModelState::Downloading { downloaded_bytes: 0, total_bytes: Some(model.total_size_bytes()) });
    }

    let inner = self.inner.clone();
    tokio::spawn(async move {
      let mut throttle = Throttle::new();
      let result = ensure_model(&inner.layout, &model, &mut |progress: Progress| {
        let finished = progress.total.is_some_and(|t| progress.downloaded >= t);
        if throttle.allow(finished.then_some(1.0)) {
          inner.downloads.lock().expect("downloads lock").insert(model.id, ModelState::Downloading {
            downloaded_bytes: progress.downloaded,
            total_bytes: progress.total,
          });
        }
      }).await;

      let mut downloads = inner.downloads.lock().expect("downloads lock");
      match result {
        Ok(()) => { downloads.remove(model.id); }
        Err(error) => {
          log::error!("Downloading {} failed: {}", model.display_name, error);
          downloads.insert(model.id, ModelState::Failed { message: error.to_string() });
        }
      }
    });
    Ok(())
  }

  /// Delete a downloaded model, and forget a failed download so it can be tried again.
  pub fn delete_model(&self, model: &CatalogModel) -> ManagerResult<()> {
    let mut downloads = self.inner.downloads.lock().expect("downloads lock");
    if matches!(downloads.get(model.id), Some(ModelState::Downloading { .. })) {
      return Err(ManagerError::Other(format!("{} is still downloading", model.display_name)));
    }
    downloads.remove(model.id);

    remove_model(&self.inner.layout, model)
  }

  /// Install the engine and download the given models. Steps already done are skipped,
  /// so running this again after an interruption continues where it stopped.
  pub async fn install(&self, models: &[CatalogModel]) -> ManagerResult<()> {
    if let Err(unsupported) = self.inner.platform.check_supported() {
      self.inner.set_status(EngineStatus::Unsupported { reason: unsupported.to_string() });
      return Err(unsupported.into());
    }

    let mut lifecycle = self.inner.lifecycle.lock().await;
    stop_locked(&self.inner, &mut lifecycle).await;

    let result = self.run_install(models).await;
    match &result {
      Ok(()) => self.inner.set_status(EngineStatus::Stopped),
      Err(error) => self.inner.set_status(EngineStatus::Failed { message: error.to_string() }),
    }
    result
  }

  /// Note that the engine is in use, so it is not stopped for being idle.
  pub fn touch(&self) {
    *self.inner.last_activity.lock().expect("activity lock") = Instant::now();
  }

  /// Stop the engine if nothing has used it for `idle_for`. Returns whether it was stopped.
  /// The check is repeated once the lifecycle lock is held, so a start that races with it wins.
  pub async fn stop_if_idle(&self, idle_for: Duration) -> bool {
    if !matches!(self.status(), EngineStatus::Running { .. }) || self.idle_time() < idle_for {
      return false;
    }
    let mut lifecycle = self.inner.lifecycle.lock().await;
    if lifecycle.running.is_none() || self.idle_time() < idle_for {
      return false;
    }
    stop_locked(&self.inner, &mut lifecycle).await;
    true
  }

  fn idle_time(&self) -> Duration {
    self.inner.last_activity.lock().expect("activity lock").elapsed()
  }

  /// Start the engine, or return the address of the one already running.
  pub async fn start(&self) -> ManagerResult<String> {
    let mut lifecycle = self.inner.lifecycle.lock().await;
    self.touch();

    if lifecycle.running.is_some() {
      if let Some(url) = self.base_url() {
        return Ok(url);
      }
    }
    if !self.is_installed() {
      return Err(ManagerError::NotInstalled);
    }

    reap_stale_engine(&self.inner.layout);
    self.inner.set_status(EngineStatus::Starting);

    match launch(&self.inner).await {
      Ok((child, base_url)) => {
        self.inner.set_status(EngineStatus::Running { base_url: base_url.clone() });
        let (stop, stop_receiver) = oneshot::channel();
        let supervisor = tokio::spawn(supervise(self.inner.clone(), child, stop_receiver));
        lifecycle.running = Some(Running { stop, supervisor });
        Ok(base_url)
      }
      Err(error) => {
        self.inner.set_status(EngineStatus::Failed { message: error.to_string() });
        Err(error)
      }
    }
  }

  /// Stop the engine. Safe to call when it is not running.
  pub async fn stop(&self) {
    let mut lifecycle = self.inner.lifecycle.lock().await;
    stop_locked(&self.inner, &mut lifecycle).await;
  }

  async fn run_install(&self, models: &[CatalogModel]) -> ManagerResult<()> {
    let engine_steps = InstallStep::ALL.len();
    let step_count = engine_steps + models.len();
    let has_nvidia = install::detect_nvidia_gpu().await;

    let inner = self.inner.clone();
    let mut throttle = Throttle::new();
    install::install(&self.inner.layout, self.inner.platform, has_nvidia, &mut |progress: InstallProgress| {
      if throttle.allow(progress.fraction) {
        inner.set_status(EngineStatus::Installing {
          label: progress.step.label().to_string(),
          step_index: progress.step_index,
          step_count,
          detail: progress.detail,
          fraction: progress.fraction,
        });
      }
    }).await?;

    for (index, spec) in models.iter().enumerate() {
      let inner = self.inner.clone();
      let label = format!("Downloading {}", spec.display_name);
      let mut throttle = Throttle::new();
      ensure_model(&self.inner.layout, spec, &mut |progress: Progress| {
        let fraction = progress.total.filter(|t| *t > 0).map(|t| progress.downloaded as f32 / t as f32);
        if throttle.allow(fraction) {
          inner.set_status(EngineStatus::Installing {
            label: label.clone(),
            step_index: engine_steps + index,
            step_count,
            detail: Some(format!("{:.0} MB", progress.downloaded as f64 / 1_000_000.0)),
            fraction,
          });
        }
      }).await?;
    }
    Ok(())
  }
}

impl Inner {
  fn set_status(&self, status: EngineStatus) {
    *self.status.lock().expect("status lock") = status.clone();
    self.status_sender.send_replace(status);
  }
}

fn initial_status(layout: &RuntimeLayout, platform: Platform) -> EngineStatus {
  if let Err(unsupported) = platform.check_supported() {
    EngineStatus::Unsupported { reason: unsupported.to_string() }
  } else if install::is_installed(layout) {
    EngineStatus::Stopped
  } else {
    EngineStatus::NotInstalled
  }
}

/// Start a process on a free port and wait until it answers.
async fn launch(inner: &Inner) -> ManagerResult<(Child, String)> {
  let port = pick_free_port()?;
  let base_url = format!("http://127.0.0.1:{}", port);
  let mut child = spawn_engine(&inner.layout, port)?;

  match wait_until_ready(&inner.layout, &mut child, &base_url, inner.ready_timeout).await {
    Ok(()) => Ok((child, base_url)),
    Err(error) => {
      let _ = child.kill().await;
      remove_pid_file(&inner.layout);
      Err(error)
    }
  }
}

async fn wait_until_ready(
  layout: &RuntimeLayout,
  child: &mut Child,
  base_url: &str,
  timeout: Duration,
) -> ManagerResult<()> {
  let client = ComfyUiClient::new(base_url);
  let deadline = Instant::now() + timeout;

  loop {
    if let Some(status) = child.try_wait().map_err(|e| ManagerError::NotReady(e.to_string()))? {
      return Err(ManagerError::NotReady(format!(
        "the engine exited ({}). {}", status, log_tail(layout, LOG_LINES_IN_ERRORS)
      )));
    }
    if client.is_healthy().await {
      return Ok(());
    }
    if Instant::now() >= deadline {
      return Err(ManagerError::NotReady(format!(
        "it did not answer within {} seconds. {}", timeout.as_secs(), log_tail(layout, LOG_LINES_IN_ERRORS)
      )));
    }
    tokio::time::sleep(POLL_INTERVAL).await;
  }
}

/// Owns the running process: stops it on request, and restarts it if it dies.
async fn supervise(inner: Arc<Inner>, mut child: Child, mut stop: oneshot::Receiver<()>) {
  let mut restarts: VecDeque<Instant> = VecDeque::new();

  loop {
    tokio::select! {
      // A request to stop, or the manager going away.
      _ = &mut stop => {
        let _ = child.kill().await;
        remove_pid_file(&inner.layout);
        inner.set_status(EngineStatus::Stopped);
        return;
      }
      exit = child.wait() => {
        log::warn!("The local engine exited unexpectedly: {:?}", exit);

        let now = Instant::now();
        restarts.push_back(now);
        while restarts.front().is_some_and(|t| now.duration_since(*t) > RESTART_WINDOW) {
          restarts.pop_front();
        }
        if restarts.len() > MAX_RESTARTS {
          remove_pid_file(&inner.layout);
          inner.set_status(EngineStatus::Failed {
            message: format!("The local engine keeps stopping. {}", log_tail(&inner.layout, LOG_LINES_IN_ERRORS)),
          });
          return;
        }

        inner.set_status(EngineStatus::Starting);
        match launch(&inner).await {
          Ok((new_child, base_url)) => {
            child = new_child;
            inner.set_status(EngineStatus::Running { base_url });
          }
          Err(error) => {
            inner.set_status(EngineStatus::Failed { message: error.to_string() });
            return;
          }
        }
      }
    }
  }
}

async fn stop_locked(inner: &Inner, lifecycle: &mut Lifecycle) {
  if let Some(running) = lifecycle.running.take() {
    let _ = running.stop.send(());
    let _ = tokio::time::timeout(STOP_TIMEOUT, running.supervisor).await;
    remove_pid_file(&inner.layout);
  }
  let status = initial_status(&inner.layout, inner.platform);
  if !matches!(status, EngineStatus::NotInstalled | EngineStatus::Unsupported { .. })
    || matches!(inner.status.lock().expect("status lock").clone(), EngineStatus::Running { .. } | EngineStatus::Starting)
  {
    inner.set_status(status);
  }
}

/// Lets a status through when enough time has passed or the step finished.
struct Throttle {
  last: Option<Instant>,
}

impl Throttle {
  fn new() -> Self {
    Self { last: None }
  }

  fn allow(&mut self, fraction: Option<f32>) -> bool {
    let due = self.last.is_none_or(|t| t.elapsed() >= PROGRESS_INTERVAL);
    let finished = fraction.is_some_and(|f| f >= 1.0);
    if due || finished {
      self.last = Some(Instant::now());
      true
    } else {
      false
    }
  }
}

#[cfg(test)]
mod model_download_tests {
  use super::*;
  use tempfile::tempdir;
  use tokio::io::{AsyncReadExt, AsyncWriteExt};
  use tokio::net::TcpListener;

  const BODY: &[u8] = b"pretend this is a model file";

  #[tokio::test]
  async fn downloads_a_model_and_then_reports_it_installed() {
    let dir = tempdir().unwrap();
    let manager = test_manager(dir.path());
    let model = model_served_at(serve(BODY).await, BODY);
    assert_eq!(manager.model_state(&model), ModelState::NotInstalled);

    manager.download_model(model).unwrap();
    wait_for_model(&manager, &model, |s| *s == ModelState::Installed).await;

    let file = manager.layout().checkpoints_dir().join(model.files[0].file_name);
    assert_eq!(std::fs::read(file).unwrap(), BODY);
  }

  #[tokio::test]
  async fn a_bad_checksum_is_reported_as_failed_and_can_be_retried() {
    let dir = tempdir().unwrap();
    let manager = test_manager(dir.path());
    let mut model = model_served_at(serve(BODY).await, BODY);
    let mut bad_file = model.files[0];
    bad_file.sha256 = Box::leak("0".repeat(64).into_boxed_str());
    model.files = Box::leak(Box::new([bad_file]));

    manager.download_model(model).unwrap();
    let state = wait_for_model(&manager, &model, |s| matches!(s, ModelState::Failed { .. })).await;
    let ModelState::Failed { message } = state else { unreachable!() };
    assert!(message.contains("checksum"), "{message}");

    // Failed downloads can be started again.
    manager.download_model(model).unwrap();
    wait_for_model(&manager, &model, |s| matches!(s, ModelState::Failed { .. })).await;
  }

  #[tokio::test]
  async fn a_model_that_is_already_there_is_not_downloaded_again() {
    let dir = tempdir().unwrap();
    let manager = test_manager(dir.path());
    // The address is closed, so this only passes if nothing is fetched.
    let model = model_served_at("http://127.0.0.1:1/x".to_string(), BODY);
    std::fs::create_dir_all(manager.layout().checkpoints_dir()).unwrap();
    std::fs::write(manager.layout().checkpoints_dir().join(model.files[0].file_name), BODY).unwrap();

    manager.download_model(model).unwrap();
    assert_eq!(manager.model_state(&model), ModelState::Installed);
  }

  #[tokio::test]
  async fn a_second_download_of_the_same_model_is_refused_while_one_runs() {
    let dir = tempdir().unwrap();
    let manager = test_manager(dir.path());
    let model = model_served_at(serve_slowly(BODY).await, BODY);

    manager.download_model(model).unwrap();
    assert!(matches!(manager.model_state(&model), ModelState::Downloading { .. }));
    assert!(manager.download_model(model).is_err());
    assert!(manager.delete_model(&model).is_err(), "cannot delete mid-download");
  }

  #[tokio::test]
  async fn deleting_removes_the_file_and_is_harmless_when_absent() {
    let dir = tempdir().unwrap();
    let manager = test_manager(dir.path());
    let model = model_served_at("http://127.0.0.1:1/x".to_string(), BODY);
    manager.delete_model(&model).unwrap();

    std::fs::create_dir_all(manager.layout().checkpoints_dir()).unwrap();
    let file = manager.layout().checkpoints_dir().join(model.files[0].file_name);
    std::fs::write(&file, BODY).unwrap();
    assert_eq!(manager.model_state(&model), ModelState::Installed);

    manager.delete_model(&model).unwrap();
    assert!(!file.exists());
    assert_eq!(manager.model_state(&model), ModelState::NotInstalled);
  }

  // ── helpers ──

  fn test_manager(root: &std::path::Path) -> ComfyUiManager {
    ComfyUiManager::with_platform(RuntimeLayout::new(root), Platform::current(), Duration::from_secs(5))
  }

  fn model_served_at(url: String, body: &[u8]) -> CatalogModel {
    use sha2::{Digest, Sha256};
    let sha256: String = Sha256::digest(body).iter().map(|b| format!("{:02x}", b)).collect();
    let file = ModelFile {
      folder: "checkpoints",
      file_name: "test_model.safetensors",
      url: Box::leak(url.into_boxed_str()),
      sha256: Box::leak(sha256.into_boxed_str()),
      size_bytes: body.len() as u64,
    };
    CatalogModel {
      id: "test_model",
      display_name: "Test model",
      license: "test",
      files: Box::leak(Box::new([file])),
      requirements: crate::suitability::Requirements::ANY,
    }
  }

  async fn wait_for_model(manager: &ComfyUiManager, model: &CatalogModel, wanted: impl Fn(&ModelState) -> bool) -> ModelState {
    for _ in 0..100 {
      let state = manager.model_state(model);
      if wanted(&state) {
        return state;
      }
      tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("model state never matched; last was {:?}", manager.model_state(model));
  }

  async fn serve(body: &'static [u8]) -> String {
    serve_with(body, Duration::ZERO).await
  }

  /// Holds the connection open so a download stays in progress.
  async fn serve_slowly(body: &'static [u8]) -> String {
    serve_with(body, Duration::from_secs(30)).await
  }

  async fn serve_with(body: &'static [u8], delay: Duration) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/model", listener.local_addr().unwrap());
    tokio::spawn(async move {
      loop {
        let Ok((mut socket, _)) = listener.accept().await else { return };
        tokio::spawn(async move {
          let mut buffer = vec![0u8; 4096];
          let _ = socket.read(&mut buffer).await;
          let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
          let _ = socket.write_all(head.as_bytes()).await;
          tokio::time::sleep(delay).await;
          let _ = socket.write_all(body).await;
          let _ = socket.shutdown().await;
        });
      }
    });
    url
  }
}

#[cfg(all(test, unix))]
mod tests {
  use super::*;
  use crate::install::{InstallManifest, TorchFlavorRecord};
  use crate::platform::{COMFYUI_VERSION, UV_VERSION};
  use tempfile::{tempdir, TempDir};

  /// Stands in for ComfyUI: serves `/system_stats` on `--port`, and exits on request.
  const FAKE_MAIN: &str = r#"
import sys, http.server
port = int(sys.argv[sys.argv.index('--port') + 1])
class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200); self.end_headers(); self.wfile.write(b'{}')
    def log_message(self, *args): pass
print('fake engine starting on', port, flush=True)
http.server.HTTPServer(('127.0.0.1', port), Handler).serve_forever()
"#;

  const BROKEN_MAIN: &str = "import sys\nprint('boom: cannot load model')\nsys.exit(3)\n";

  mod lifecycle {
    use super::*;

    #[tokio::test]
    async fn starts_serves_and_stops() {
      let Some(fixture) = Fixture::installed(FAKE_MAIN) else { return };

      assert_eq!(fixture.manager.status(), EngineStatus::Stopped);
      let url = fixture.manager.start().await.unwrap();
      assert_eq!(fixture.manager.status(), EngineStatus::Running { base_url: url.clone() });
      assert!(ComfyUiClient::new(&url).is_healthy().await);
      assert!(fixture.manager.layout().pid_file().exists());

      fixture.manager.stop().await;
      assert_eq!(fixture.manager.status(), EngineStatus::Stopped);
      assert!(!ComfyUiClient::new(&url).is_healthy().await);
      assert!(!fixture.manager.layout().pid_file().exists());
    }

    #[tokio::test]
    async fn starting_twice_returns_the_same_engine() {
      let Some(fixture) = Fixture::installed(FAKE_MAIN) else { return };

      let first = fixture.manager.start().await.unwrap();
      let second = fixture.manager.start().await.unwrap();
      assert_eq!(first, second);

      fixture.manager.stop().await;
    }

    #[tokio::test]
    async fn an_idle_engine_is_stopped() {
      let Some(fixture) = Fixture::installed(FAKE_MAIN) else { return };
      fixture.manager.start().await.unwrap();

      tokio::time::sleep(Duration::from_millis(60)).await;
      assert!(fixture.manager.stop_if_idle(Duration::from_millis(30)).await);
      assert_eq!(fixture.manager.status(), EngineStatus::Stopped);
    }

    #[tokio::test]
    async fn an_engine_in_use_is_left_running() {
      let Some(fixture) = Fixture::installed(FAKE_MAIN) else { return };
      fixture.manager.start().await.unwrap();

      tokio::time::sleep(Duration::from_millis(60)).await;
      fixture.manager.touch();
      assert!(!fixture.manager.stop_if_idle(Duration::from_millis(50)).await);
      assert!(matches!(fixture.manager.status(), EngineStatus::Running { .. }));

      fixture.manager.stop().await;
    }

    #[tokio::test]
    async fn an_engine_that_is_not_running_is_not_stopped() {
      let Some(fixture) = Fixture::installed(FAKE_MAIN) else { return };
      assert!(!fixture.manager.stop_if_idle(Duration::ZERO).await);
      assert_eq!(fixture.manager.status(), EngineStatus::Stopped);
    }

    #[tokio::test]
    async fn starting_again_after_an_idle_stop_works() {
      let Some(fixture) = Fixture::installed(FAKE_MAIN) else { return };
      fixture.manager.start().await.unwrap();
      tokio::time::sleep(Duration::from_millis(40)).await;
      assert!(fixture.manager.stop_if_idle(Duration::from_millis(20)).await);

      let url = fixture.manager.start().await.unwrap();
      assert!(ComfyUiClient::new(&url).is_healthy().await);
      fixture.manager.stop().await;
    }

    #[tokio::test]
    async fn stopping_when_not_running_is_harmless() {
      let Some(fixture) = Fixture::installed(FAKE_MAIN) else { return };
      fixture.manager.stop().await;
      assert_eq!(fixture.manager.status(), EngineStatus::Stopped);
    }

    #[tokio::test]
    async fn cannot_start_before_installing() {
      let dir = tempdir().unwrap();
      let manager = test_manager(dir.path());

      assert_eq!(manager.status(), EngineStatus::NotInstalled);
      let error = manager.start().await.unwrap_err();
      assert!(matches!(error, ManagerError::NotInstalled), "{error:?}");
      assert_eq!(manager.status(), EngineStatus::NotInstalled);
    }

    #[tokio::test]
    async fn an_install_that_appears_on_disk_is_noticed() {
      let Some(fixture) = Fixture::installed(FAKE_MAIN) else { return };
      // A manager created before the files existed, as when something else installed the engine.
      let late = ComfyUiManager::with_platform(
        fixture.manager.layout().clone(), Platform::current(), Duration::from_secs(20));
      late.inner.set_status(EngineStatus::NotInstalled);

      assert_eq!(late.status(), EngineStatus::Stopped);
    }

    #[tokio::test]
    async fn status_changes_are_published() {
      let Some(fixture) = Fixture::installed(FAKE_MAIN) else { return };
      let mut receiver = fixture.manager.subscribe();

      fixture.manager.start().await.unwrap();
      assert!(receiver.has_changed().unwrap());
      assert!(matches!(&*receiver.borrow_and_update(), EngineStatus::Running { .. }));

      fixture.manager.stop().await;
    }
  }

  mod failures {
    use super::*;

    #[tokio::test]
    async fn an_engine_that_dies_on_startup_reports_why() {
      let Some(fixture) = Fixture::installed(BROKEN_MAIN) else { return };

      let error = fixture.manager.start().await.unwrap_err();

      let message = error.to_string();
      assert!(matches!(error, ManagerError::NotReady(_)));
      assert!(message.contains("boom: cannot load model"), "{message}");
      assert!(matches!(fixture.manager.status(), EngineStatus::Failed { .. }));
      assert!(!fixture.manager.layout().pid_file().exists());
    }

    #[tokio::test]
    async fn a_crashed_engine_is_restarted() {
      let Some(fixture) = Fixture::installed(FAKE_MAIN) else { return };

      let first = fixture.manager.start().await.unwrap();
      let pid = read_pid(&fixture);
      kill_hard(pid);

      let second = wait_for_other_url(&fixture.manager, &first).await;
      assert!(ComfyUiClient::new(&second).is_healthy().await);

      fixture.manager.stop().await;
    }

    #[tokio::test]
    async fn an_engine_that_keeps_crashing_is_given_up_on() {
      let Some(fixture) = Fixture::installed(FAKE_MAIN) else { return };

      let mut url = fixture.manager.start().await.unwrap();
      for _ in 0..=MAX_RESTARTS {
        kill_hard(read_pid(&fixture));
        match wait_for_status(&fixture.manager, |s| match s {
          EngineStatus::Running { base_url } => *base_url != url,
          EngineStatus::Failed { .. } => true,
          _ => false,
        }).await {
          EngineStatus::Running { base_url } => url = base_url,
          _ => break,
        }
      }

      let status = wait_for_status(&fixture.manager, |s| matches!(s, EngineStatus::Failed { .. })).await;
      let EngineStatus::Failed { message } = status else { unreachable!() };
      assert!(message.contains("keeps stopping"), "{message}");
    }
  }

  mod cleanup {
    use super::*;

    #[tokio::test]
    async fn a_leftover_engine_from_a_crashed_app_is_stopped_on_start() {
      let Some(fixture) = Fixture::installed(FAKE_MAIN) else { return };

      // An engine nobody is watching: the pid file is all that is left of it.
      let orphan = spawn_engine(fixture.manager.layout(), pick_free_port().unwrap()).unwrap();
      let orphan_pid = orphan.id().unwrap();
      std::mem::forget(orphan);
      assert!(is_alive(orphan_pid));

      fixture.manager.start().await.unwrap();
      wait_until(|| !is_alive(orphan_pid)).await;
      assert!(!is_alive(orphan_pid), "the leftover process should have been stopped");

      fixture.manager.stop().await;
    }
  }

  mod throttle {
    use super::*;

    #[test]
    fn lets_the_first_and_the_finishing_update_through() {
      let mut throttle = Throttle::new();
      assert!(throttle.allow(Some(0.0)));
      assert!(!throttle.allow(Some(0.5)), "too soon");
      assert!(throttle.allow(Some(1.0)), "finishing is always shown");
    }
  }

  // ── helpers ──

  struct Fixture {
    manager: ComfyUiManager,
    _dir: TempDir,
  }

  impl Fixture {
    /// An "installed" engine whose Python is the system python3 and whose main.py is `script`.
    /// `None` when python3 is not available, so the test is skipped.
    fn installed(script: &str) -> Option<Fixture> {
      if std::process::Command::new("python3").arg("--version").output().is_err() {
        eprintln!("python3 not found; skipping");
        return None;
      }
      let dir = tempdir().unwrap();
      let layout = RuntimeLayout::new(dir.path());

      write_executable(&layout.venv_python(), "#!/bin/sh\nexec python3 \"$@\"\n");
      write_file(&layout.main_script(), script);
      write_file(&layout.uv_executable(), "");
      InstallManifest {
        uv_version: UV_VERSION.to_string(),
        comfyui_version: COMFYUI_VERSION.to_string(),
        torch_flavor: TorchFlavorRecord::Default,
        completed: InstallStep::ALL.to_vec(),
      }.save_for_tests(&layout);

      Some(Fixture { manager: test_manager(dir.path()), _dir: dir })
    }
  }

  fn test_manager(root: &std::path::Path) -> ComfyUiManager {
    let platform = Platform::current();
    ComfyUiManager::with_platform(RuntimeLayout::new(root), platform, Duration::from_secs(20))
  }

  fn write_file(path: &std::path::Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
  }

  fn write_executable(path: &std::path::Path, text: &str) {
    use std::os::unix::fs::PermissionsExt;
    write_file(path, text);
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
  }

  fn read_pid(fixture: &Fixture) -> u32 {
    std::fs::read_to_string(fixture.manager.layout().pid_file()).unwrap().trim().parse().unwrap()
  }

  fn kill_hard(pid: u32) {
    let _ = std::process::Command::new("kill").args(["-9", &pid.to_string()]).status();
  }

  /// Running, not merely a zombie. A killed child of this test process stays a zombie
  /// until it is reaped, which a real leftover engine (adopted by init) would not.
  fn is_alive(pid: u32) -> bool {
    let output = std::process::Command::new("ps").args(["-o", "stat=", "-p", &pid.to_string()])
      .output().unwrap();
    let state = String::from_utf8_lossy(&output.stdout).trim().to_string();
    !state.is_empty() && !state.starts_with('Z')
  }

  async fn wait_until(condition: impl Fn() -> bool) {
    for _ in 0..100 {
      if condition() {
        return;
      }
      tokio::time::sleep(Duration::from_millis(100)).await;
    }
  }

  async fn wait_for_status(manager: &ComfyUiManager, wanted: impl Fn(&EngineStatus) -> bool) -> EngineStatus {
    for _ in 0..200 {
      let status = manager.status();
      if wanted(&status) {
        return status;
      }
      tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("status never matched; last was {:?}", manager.status());
  }

  async fn wait_for_other_url(manager: &ComfyUiManager, old: &str) -> String {
    match wait_for_status(manager, |s| matches!(s, EngineStatus::Running { base_url } if base_url != old)).await {
      EngineStatus::Running { base_url } => base_url,
      _ => unreachable!(),
    }
  }
}
