use crate::platform::{Arch, Os, Platform};
use serde_derive::Serialize;
use std::process::Stdio;
use std::time::Duration;
use sysinfo::System;
use tokio::process::Command;

const BYTES_PER_GB: f64 = 1_073_741_824.0;
const MIB_PER_GB: f64 = 1024.0;

/// What generates the pictures on this computer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuKind {
  /// The GPU shares the computer's memory.
  AppleSilicon,
  /// A card with its own video memory.
  Nvidia,
  /// No GPU the engine can use, so everything runs on the CPU.
  None,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HardwareProfile {
  pub gpu_kind: GpuKind,
  pub gpu_name: Option<String>,
  /// The card's own memory. Only known for NVIDIA cards.
  pub gpu_memory_gb: Option<f64>,
  pub system_memory_gb: f64,
}

impl HardwareProfile {
  pub async fn detect(platform: Platform) -> Self {
    let system_memory_gb = system_memory_gb();

    if platform.os == Os::MacOs && platform.arch == Arch::Aarch64 {
      return Self {
        gpu_kind: GpuKind::AppleSilicon,
        gpu_name: cpu_brand(),
        gpu_memory_gb: None,
        system_memory_gb,
      };
    }

    match query_nvidia().await.and_then(|output| parse_nvidia_smi(&output)) {
      Some((name, memory_gb)) => Self {
        gpu_kind: GpuKind::Nvidia,
        gpu_name: Some(name),
        gpu_memory_gb: Some(memory_gb),
        system_memory_gb,
      },
      None => Self { gpu_kind: GpuKind::None, gpu_name: None, gpu_memory_gb: None, system_memory_gb },
    }
  }

  /// One line for the screen.
  pub fn summary(&self) -> String {
    let ram = format_gb(self.system_memory_gb);
    match self.gpu_kind {
      GpuKind::AppleSilicon => format!(
        "{} · {} shared memory", self.gpu_name.as_deref().unwrap_or("Apple Silicon"), ram),
      GpuKind::Nvidia => format!(
        "{} · {} video memory · {} system memory",
        self.gpu_name.as_deref().unwrap_or("NVIDIA GPU"),
        format_gb(self.gpu_memory_gb.unwrap_or(0.0)),
        ram),
      GpuKind::None => format!("No supported graphics card · {} memory · runs on the CPU", ram),
    }
  }

  /// GTX 16-series and similar cards cannot use half precision reliably, so ComfyUI stores
  /// models at twice the size on them.
  pub fn lacks_half_precision(&self) -> bool {
    const CARDS: [&str; 12] = ["1660", "1650", "1630", "T500", "T550", "T600", "MX550", "MX450", "CMP 30HX", "T2000", "T1000", "T1200"];
    self.gpu_name.as_deref().is_some_and(|name| CARDS.iter().any(|card| name.contains(card)))
  }
}

/// The best card from `nvidia-smi --query-gpu=name,memory.total --format=csv,noheader,nounits`,
/// as (name, gigabytes). Each line looks like `NVIDIA GeForce GTX 1650, 4096`.
pub fn parse_nvidia_smi(output: &str) -> Option<(String, f64)> {
  output.lines()
    .filter_map(|line| {
      let (name, mib) = line.rsplit_once(',')?;
      let mib: f64 = mib.trim().parse().ok()?;
      let name = name.trim();
      (!name.is_empty() && mib > 0.0).then(|| (name.to_string(), mib / MIB_PER_GB))
    })
    .max_by(|a, b| a.1.total_cmp(&b.1))
}

async fn query_nvidia() -> Option<String> {
  let run = Command::new("nvidia-smi")
    .args(["--query-gpu=name,memory.total", "--format=csv,noheader,nounits"])
    .stdin(Stdio::null()).stderr(Stdio::null())
    .output();
  let output = tokio::time::timeout(Duration::from_secs(5), run).await.ok()?.ok()?;
  output.status.success().then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn system_memory_gb() -> f64 {
  let mut system = System::new();
  system.refresh_memory();
  system.total_memory() as f64 / BYTES_PER_GB
}

fn cpu_brand() -> Option<String> {
  let mut system = System::new();
  system.refresh_cpu_all();
  system.cpus().first().map(|cpu| cpu.brand().trim().to_string()).filter(|b| !b.is_empty())
}

pub(crate) fn format_gb(gb: f64) -> String {
  // 15.9 GB of a 16 GB machine should read as 16.
  if (gb - gb.round()).abs() < 0.25 { format!("{:.0} GB", gb.round()) } else { format!("{:.1} GB", gb) }
}

#[cfg(test)]
mod tests {
  use super::*;

  mod nvidia_parsing {
    use super::*;

    #[test]
    fn reads_one_card() {
      let (name, gb) = parse_nvidia_smi("NVIDIA GeForce GTX 1650, 4096\n").unwrap();
      assert_eq!(name, "NVIDIA GeForce GTX 1650");
      assert_eq!(gb, 4.0);
    }

    #[test]
    fn picks_the_card_with_the_most_memory() {
      let (name, gb) = parse_nvidia_smi("NVIDIA GeForce GTX 1050, 2048\nNVIDIA GeForce RTX 4090, 24564\n").unwrap();
      assert_eq!(name, "NVIDIA GeForce RTX 4090");
      assert!((gb - 23.99).abs() < 0.01);
    }

    #[test]
    fn a_name_with_a_comma_still_parses() {
      let (name, gb) = parse_nvidia_smi("NVIDIA RTX A2000, 12GB edition, 12282").unwrap();
      assert_eq!(name, "NVIDIA RTX A2000, 12GB edition");
      assert!(gb > 11.0);
    }

    #[test]
    fn rejects_empty_or_garbled_output() {
      assert!(parse_nvidia_smi("").is_none());
      assert!(parse_nvidia_smi("No devices were found").is_none());
      assert!(parse_nvidia_smi("NVIDIA Card, notanumber").is_none());
      assert!(parse_nvidia_smi("NVIDIA Card, 0").is_none());
    }
  }

  mod profile {
    use super::*;

    fn nvidia(name: &str, gb: f64, ram: f64) -> HardwareProfile {
      HardwareProfile { gpu_kind: GpuKind::Nvidia, gpu_name: Some(name.into()), gpu_memory_gb: Some(gb), system_memory_gb: ram }
    }

    #[test]
    fn summaries_read_naturally() {
      let mac = HardwareProfile { gpu_kind: GpuKind::AppleSilicon, gpu_name: Some("Apple M4 Pro".into()), gpu_memory_gb: None, system_memory_gb: 23.9 };
      assert_eq!(mac.summary(), "Apple M4 Pro · 24 GB shared memory");
      assert_eq!(nvidia("NVIDIA GeForce GTX 1650", 4.0, 16.0).summary(),
        "NVIDIA GeForce GTX 1650 · 4 GB video memory · 16 GB system memory");
      let cpu = HardwareProfile { gpu_kind: GpuKind::None, gpu_name: None, gpu_memory_gb: None, system_memory_gb: 7.8 };
      assert_eq!(cpu.summary(), "No supported graphics card · 8 GB memory · runs on the CPU");
    }

    #[test]
    fn sixteen_series_cards_are_recognised() {
      assert!(nvidia("NVIDIA GeForce GTX 1650", 4.0, 16.0).lacks_half_precision());
      assert!(nvidia("NVIDIA GeForce GTX 1660 Ti", 6.0, 16.0).lacks_half_precision());
      assert!(!nvidia("NVIDIA GeForce RTX 3060", 12.0, 16.0).lacks_half_precision());
      assert!(!HardwareProfile { gpu_kind: GpuKind::None, gpu_name: None, gpu_memory_gb: None, system_memory_gb: 8.0 }.lacks_half_precision());
    }

    #[test]
    fn formats_memory_sizes() {
      assert_eq!(format_gb(15.9), "16 GB");
      assert_eq!(format_gb(3.99), "4 GB");
      assert_eq!(format_gb(5.5), "5.5 GB");
    }
  }

  #[tokio::test]
  async fn detecting_this_computer_gives_a_sensible_answer() {
    let profile = HardwareProfile::detect(Platform::current()).await;
    assert!(profile.system_memory_gb > 0.5, "{profile:?}");
    assert!(!profile.summary().is_empty());
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
      assert_eq!(profile.gpu_kind, GpuKind::AppleSilicon);
    }
  }
}
