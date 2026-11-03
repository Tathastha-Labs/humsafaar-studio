use crate::error::{io_error, ManagerError, ManagerResult};
use crate::install::hide_window;
use crate::layout::RuntimeLayout;
use std::net::TcpListener;
use std::process::Stdio;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use tokio::process::{Child, Command};

/// A port that was free a moment ago. Another program could still take it, so the
/// caller treats a failed start as retryable.
pub fn pick_free_port() -> ManagerResult<u16> {
  let listener = TcpListener::bind("127.0.0.1:0")
    .map_err(|e| ManagerError::Other(format!("could not find a free port: {}", e)))?;
  listener.local_addr()
    .map(|address| address.port())
    .map_err(|e| ManagerError::Other(format!("could not find a free port: {}", e)))
}

/// Start ComfyUI listening only on this computer. Output goes to the log file.
pub fn spawn_engine(layout: &RuntimeLayout, port: u16) -> ManagerResult<Child> {
  let log_path = layout.log_file();
  if let Some(parent) = log_path.parent() {
    std::fs::create_dir_all(parent).map_err(|e| io_error(parent, e))?;
  }
  prepare_data_dir(layout)?;

  let log = std::fs::OpenOptions::new().create(true).append(true).open(&log_path)
    .map_err(|e| io_error(&log_path, e))?;
  let log_for_errors = log.try_clone().map_err(|e| io_error(&log_path, e))?;

  let mut command = Command::new(layout.venv_python());
  command
    .arg(layout.main_script())
    .args(["--listen", "127.0.0.1"])
    .args(["--port", &port.to_string()])
    .arg("--base-directory").arg(layout.data_dir())
    .arg("--disable-auto-launch")
    .current_dir(layout.app_dir())
    .env("PYTHONUNBUFFERED", "1")
    .stdin(Stdio::null())
    .stdout(Stdio::from(log))
    .stderr(Stdio::from(log_for_errors))
    .kill_on_drop(true);
  hide_window(&mut command);

  let child = command.spawn().map_err(|e| io_error(layout.venv_python(), e))?;
  if let Some(pid) = child.id() {
    let _ = std::fs::write(layout.pid_file(), pid.to_string());
  }
  Ok(child)
}

/// ComfyUI refuses to start if its base directory lacks these folders.
const DATA_SUBFOLDERS: [&str; 6] = ["custom_nodes", "models", "input", "output", "temp", "user"];

pub fn prepare_data_dir(layout: &RuntimeLayout) -> ManagerResult<()> {
  for folder in DATA_SUBFOLDERS {
    let path = layout.data_dir().join(folder);
    std::fs::create_dir_all(&path).map_err(|e| io_error(&path, e))?;
  }
  Ok(())
}

/// If an engine from an earlier run is still alive (the app crashed or was killed),
/// stop it. Only a process running our own `main.py` is touched.
pub fn reap_stale_engine(layout: &RuntimeLayout) {
  let pid_file = layout.pid_file();
  let Some(pid) = std::fs::read_to_string(&pid_file).ok().and_then(|t| t.trim().parse::<u32>().ok()) else {
    let _ = std::fs::remove_file(&pid_file);
    return;
  };

  let pid = Pid::from_u32(pid);
  let mut system = System::new();
  // The command line is not read unless asked for, and it is what identifies our engine.
  system.refresh_processes_specifics(
    ProcessesToUpdate::Some(&[pid]),
    true,
    ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always),
  );

  if let Some(process) = system.process(pid) {
    let ours = layout.main_script();
    let runs_our_engine = process.cmd().iter().any(|arg| std::path::Path::new(arg) == ours);
    if runs_our_engine {
      log::warn!("Stopping a leftover ComfyUI process (pid {})", pid);
      process.kill();
    }
  }
  let _ = std::fs::remove_file(&pid_file);
}

pub fn remove_pid_file(layout: &RuntimeLayout) {
  let _ = std::fs::remove_file(layout.pid_file());
}

/// The last lines of the engine's log, for error messages.
pub fn log_tail(layout: &RuntimeLayout, lines: usize) -> String {
  let Ok(text) = std::fs::read_to_string(layout.log_file()) else {
    return String::new();
  };
  let all: Vec<&str> = text.lines().collect();
  all[all.len().saturating_sub(lines)..].join("\n")
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::tempdir;

  #[test]
  fn free_ports_are_nonzero_and_usable() {
    let port = pick_free_port().unwrap();
    assert!(port > 0);
    assert!(TcpListener::bind(("127.0.0.1", port)).is_ok());
  }

  #[test]
  fn the_data_folder_gets_every_folder_comfyui_needs() {
    let dir = tempdir().unwrap();
    let layout = RuntimeLayout::new(dir.path());

    prepare_data_dir(&layout).unwrap();
    prepare_data_dir(&layout).unwrap(); // safe to repeat

    for folder in ["custom_nodes", "models", "input", "output", "temp", "user"] {
      assert!(layout.data_dir().join(folder).is_dir(), "{folder}");
    }
  }

  #[test]
  fn log_tail_returns_the_last_lines() {
    let dir = tempdir().unwrap();
    let layout = RuntimeLayout::new(dir.path());
    assert_eq!(log_tail(&layout, 3), "");

    std::fs::create_dir_all(layout.log_file().parent().unwrap()).unwrap();
    std::fs::write(layout.log_file(), "1\n2\n3\n4\n5\n").unwrap();
    assert_eq!(log_tail(&layout, 2), "4\n5");
    assert_eq!(log_tail(&layout, 99), "1\n2\n3\n4\n5");
  }

  #[test]
  fn reaping_with_no_or_garbage_pid_file_is_harmless() {
    let dir = tempdir().unwrap();
    let layout = RuntimeLayout::new(dir.path());
    reap_stale_engine(&layout);

    std::fs::write(layout.pid_file(), "not a pid").unwrap();
    reap_stale_engine(&layout);
    assert!(!layout.pid_file().exists());
  }

  #[test]
  fn a_process_that_is_not_our_engine_is_left_alone() {
    // This test process is alive but is not running our main.py.
    let dir = tempdir().unwrap();
    let layout = RuntimeLayout::new(dir.path());
    std::fs::write(layout.pid_file(), std::process::id().to_string()).unwrap();

    reap_stale_engine(&layout);

    assert!(!layout.pid_file().exists(), "the stale record is removed");
    // Still running, or this line would not execute.
  }
}
