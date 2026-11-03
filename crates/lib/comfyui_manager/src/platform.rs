use std::fmt;

/// The `uv` release the installer downloads. Pinned, with a checksum per file.
pub const UV_VERSION: &str = "0.12.19";

/// A known-good build of the ComfyUI source.
pub const COMFYUI_VERSION: &str = "v0.42.0";

const UV_RELEASES_URL: &str = "https://github.com/astral-sh/uv/releases/download";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
  MacOs,
  Windows,
  Linux,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arch {
  Aarch64,
  X86_64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Platform {
  pub os: Os,
  pub arch: Arch,
}

/// Which PyTorch build to install.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TorchFlavor {
  /// The standard wheels. On Apple Silicon these use the GPU (Metal).
  Default,
  /// NVIDIA GPUs.
  Cuda,
  /// No usable GPU. Works, but slowly.
  Cpu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveKind {
  TarGz,
  Zip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UvAsset {
  pub file_name: &'static str,
  pub archive: ArchiveKind,
  pub sha256: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TorchInstall {
  pub packages: &'static [&'static str],
  pub index_url: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnsupportedPlatform(pub Platform);

impl Platform {
  pub fn current() -> Self {
    let os = if cfg!(target_os = "macos") {
      Os::MacOs
    } else if cfg!(windows) {
      Os::Windows
    } else {
      Os::Linux
    };
    let arch = if cfg!(target_arch = "aarch64") { Arch::Aarch64 } else { Arch::X86_64 };
    Self { os, arch }
  }

  /// PyTorch no longer ships for Intel Macs or Windows on ARM.
  pub fn check_supported(&self) -> Result<(), UnsupportedPlatform> {
    match (self.os, self.arch) {
      (Os::MacOs, Arch::X86_64) | (Os::Windows, Arch::Aarch64) => Err(UnsupportedPlatform(*self)),
      _ => Ok(()),
    }
  }

  pub fn uv_asset(&self) -> Result<UvAsset, UnsupportedPlatform> {
    self.check_supported()?;
    Ok(match (self.os, self.arch) {
      (Os::MacOs, _) => UvAsset {
        file_name: "uv-aarch64-apple-darwin.tar.gz",
        archive: ArchiveKind::TarGz,
        sha256: "a9a8df1eedeb192f2e47e40e2faabfb387db4b850209118786d42f89dde3e0ba",
      },
      (Os::Windows, _) => UvAsset {
        file_name: "uv-x86_64-pc-windows-msvc.zip",
        archive: ArchiveKind::Zip,
        sha256: "6dbb02d79e419522f1c500f0adb1cddcff0cda7d59b0d66ea7f5e3b4a1b2f5f0",
      },
      (Os::Linux, Arch::X86_64) => UvAsset {
        file_name: "uv-x86_64-unknown-linux-gnu.tar.gz",
        archive: ArchiveKind::TarGz,
        sha256: "23bf5552d220e0842b65c862097b2ebaeba0064b74eda5e565e77fd25969d8c8",
      },
      (Os::Linux, Arch::Aarch64) => UvAsset {
        file_name: "uv-aarch64-unknown-linux-gnu.tar.gz",
        archive: ArchiveKind::TarGz,
        sha256: "0804e9b164c64b6914182d5920c08551958a095986f10a3731056df701126436",
      },
    })
  }

  /// Which PyTorch to install. macOS always uses the standard wheels (Metal);
  /// elsewhere an NVIDIA GPU selects CUDA and anything else falls back to CPU.
  pub fn torch_flavor(&self, has_nvidia_gpu: bool) -> TorchFlavor {
    match self.os {
      Os::MacOs => TorchFlavor::Default,
      Os::Windows | Os::Linux => {
        // CUDA wheels exist for Windows x86_64 and Linux x86_64 only.
        if has_nvidia_gpu && self.arch == Arch::X86_64 { TorchFlavor::Cuda } else { TorchFlavor::Cpu }
      }
    }
  }
}

impl UvAsset {
  pub fn url(&self) -> String {
    format!("{}/{}/{}", UV_RELEASES_URL, UV_VERSION, self.file_name)
  }

  /// Where the `uv` executable sits inside the extracted archive.
  pub fn executable_path_in_archive(&self) -> String {
    let folder = self.file_name
      .trim_end_matches(".tar.gz")
      .trim_end_matches(".zip");
    let exe = if self.archive == ArchiveKind::Zip { "uv.exe" } else { "uv" };
    // The Windows zip is flat; the tarballs have one top-level folder.
    if self.archive == ArchiveKind::Zip { exe.to_string() } else { format!("{}/{}", folder, exe) }
  }
}

impl TorchFlavor {
  pub fn install(&self) -> TorchInstall {
    const PACKAGES: &[&str] = &["torch", "torchvision", "torchaudio"];
    TorchInstall {
      packages: PACKAGES,
      index_url: match self {
        TorchFlavor::Default => None,
        TorchFlavor::Cuda => Some("https://download.pytorch.org/whl/cu128"),
        TorchFlavor::Cpu => Some("https://download.pytorch.org/whl/cpu"),
      },
    }
  }
}

pub fn comfyui_source_url() -> String {
  format!("https://github.com/comfyanonymous/ComfyUI/archive/refs/tags/{}.tar.gz", COMFYUI_VERSION)
}

impl fmt::Display for UnsupportedPlatform {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "local generation is not supported on {:?} {:?}", self.0.os, self.0.arch)
  }
}

impl std::error::Error for UnsupportedPlatform {}

#[cfg(test)]
mod tests {
  use super::*;

  const MAC_ARM: Platform = Platform { os: Os::MacOs, arch: Arch::Aarch64 };
  const MAC_INTEL: Platform = Platform { os: Os::MacOs, arch: Arch::X86_64 };
  const WIN: Platform = Platform { os: Os::Windows, arch: Arch::X86_64 };
  const WIN_ARM: Platform = Platform { os: Os::Windows, arch: Arch::Aarch64 };
  const LINUX: Platform = Platform { os: Os::Linux, arch: Arch::X86_64 };
  const LINUX_ARM: Platform = Platform { os: Os::Linux, arch: Arch::Aarch64 };

  mod support {
    use super::*;

    #[test]
    fn intel_mac_and_windows_arm_are_unsupported() {
      assert!(MAC_INTEL.check_supported().is_err());
      assert!(WIN_ARM.check_supported().is_err());
      assert!(MAC_INTEL.uv_asset().is_err());
    }

    #[test]
    fn the_rest_are_supported() {
      for p in [MAC_ARM, WIN, LINUX, LINUX_ARM] {
        assert!(p.check_supported().is_ok(), "{:?}", p);
      }
    }
  }

  mod torch {
    use super::*;

    #[test]
    fn mac_uses_the_standard_wheels_with_or_without_nvidia() {
      assert_eq!(MAC_ARM.torch_flavor(false), TorchFlavor::Default);
      assert_eq!(MAC_ARM.torch_flavor(true), TorchFlavor::Default);
      assert_eq!(TorchFlavor::Default.install().index_url, None);
    }

    #[test]
    fn nvidia_selects_cuda_on_x86_windows_and_linux() {
      assert_eq!(WIN.torch_flavor(true), TorchFlavor::Cuda);
      assert_eq!(LINUX.torch_flavor(true), TorchFlavor::Cuda);
      assert!(TorchFlavor::Cuda.install().index_url.unwrap().ends_with("/cu128"));
    }

    #[test]
    fn no_gpu_or_arm_linux_falls_back_to_cpu() {
      assert_eq!(WIN.torch_flavor(false), TorchFlavor::Cpu);
      assert_eq!(LINUX.torch_flavor(false), TorchFlavor::Cpu);
      assert_eq!(LINUX_ARM.torch_flavor(true), TorchFlavor::Cpu);
      assert!(TorchFlavor::Cpu.install().index_url.unwrap().ends_with("/cpu"));
    }
  }

  mod uv {
    use super::*;

    #[test]
    fn every_supported_platform_has_a_pinned_checksum() {
      for p in [MAC_ARM, WIN, LINUX, LINUX_ARM] {
        let asset = p.uv_asset().unwrap();
        assert_eq!(asset.sha256.len(), 64, "{:?}", p);
        assert!(asset.sha256.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(asset.url().contains(UV_VERSION));
      }
    }

    #[test]
    fn windows_is_a_flat_zip_and_the_rest_are_tarballs() {
      assert_eq!(WIN.uv_asset().unwrap().executable_path_in_archive(), "uv.exe");
      assert_eq!(
        LINUX.uv_asset().unwrap().executable_path_in_archive(),
        "uv-x86_64-unknown-linux-gnu/uv"
      );
      assert_eq!(
        MAC_ARM.uv_asset().unwrap().executable_path_in_archive(),
        "uv-aarch64-apple-darwin/uv"
      );
    }
  }

  #[test]
  fn comfyui_source_is_pinned_to_a_tag() {
    assert!(comfyui_source_url().ends_with(&format!("{}.tar.gz", COMFYUI_VERSION)));
  }
}
