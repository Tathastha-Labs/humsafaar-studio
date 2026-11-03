use crate::hardware::{format_gb, GpuKind, HardwareProfile};
use serde_derive::Serialize;

/// How a model copes with having no GPU at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuSupport {
  /// Works; a picture takes a minute or a few.
  Slow,
  /// Works; a picture takes many minutes.
  VerySlow,
  /// Would take hours; do not bother.
  Impractical,
}

/// What a model needs. "Good" means it runs comfortably; "workable" means it runs, with a slowdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Requirements {
  pub nvidia_good_gb: u32,
  pub nvidia_workable_gb: u32,
  /// An Apple Silicon Mac's GPU shares the computer's memory, so this is total memory.
  pub apple_good_gb: u32,
  pub apple_workable_gb: u32,
  /// Needed when the card is too small and parts of the model are swapped through RAM, and when
  /// running on the CPU.
  pub system_ram_gb: u32,
  pub cpu: CpuSupport,
}

impl Requirements {
  /// For tests that do not care about hardware.
  pub const ANY: Requirements = Requirements {
    nvidia_good_gb: 0, nvidia_workable_gb: 0, apple_good_gb: 0, apple_workable_gb: 0,
    system_ram_gb: 0, cpu: CpuSupport::Slow,
  };

  pub fn minimum_text(&self) -> String {
    let cpu = match self.cpu {
      CpuSupport::Slow => " It also runs on a CPU, slowly.",
      CpuSupport::VerySlow => " It runs on a CPU only very slowly.",
      CpuSupport::Impractical => " It needs a graphics card; a CPU would take hours.",
    };
    format!(
      "{} GB video memory (NVIDIA) or {} GB memory on an Apple Silicon Mac, with {} GB of system memory.{}",
      self.nvidia_workable_gb, self.apple_workable_gb, self.system_ram_gb, cpu)
  }

  pub fn recommended_text(&self) -> String {
    format!(
      "{} GB video memory (NVIDIA) or {} GB memory on an Apple Silicon Mac.",
      self.nvidia_good_gb, self.apple_good_gb)
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Suitability {
  Good,
  Slow,
  VerySlow,
  NotRecommended,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Assessment {
  pub level: Suitability,
  /// Plain words for the user.
  pub message: String,
}

/// How well a model with these requirements will run on this computer.
pub fn assess(profile: &HardwareProfile, needs: &Requirements) -> Assessment {
  match profile.gpu_kind {
    GpuKind::AppleSilicon => assess_apple(profile, needs),
    GpuKind::Nvidia => assess_nvidia(profile, needs),
    GpuKind::None => assess_cpu(profile, needs),
  }
}

fn assess_apple(profile: &HardwareProfile, needs: &Requirements) -> Assessment {
  let memory = profile.system_memory_gb;
  let have = format_gb(memory);

  if memory >= needs.apple_good_gb as f64 - TOLERANCE_GB {
    good()
  } else if memory >= needs.apple_workable_gb as f64 - TOLERANCE_GB {
    rated(Suitability::Slow, format!(
      "This computer has {} of memory and {} GB is comfortable. It will run, but expect slower generation, and close other apps first.",
      have, needs.apple_good_gb))
  } else {
    rated(Suitability::NotRecommended, format!(
      "This needs at least {} GB of memory and this computer has {}. It will likely run out of memory.",
      needs.apple_workable_gb, have))
  }
}

fn assess_nvidia(profile: &HardwareProfile, needs: &Requirements) -> Assessment {
  let vram = profile.gpu_memory_gb.unwrap_or(0.0);
  let halved = profile.lacks_half_precision();
  // Such cards store the model at twice the size.
  let effective = if halved { vram / 2.0 } else { vram };
  let note = if halved {
    " This card type cannot use half precision, which doubles the memory a model needs."
  } else {
    ""
  };

  if effective >= needs.nvidia_good_gb as f64 - TOLERANCE_GB {
    good()
  } else if effective >= needs.nvidia_workable_gb as f64 - TOLERANCE_GB {
    rated(Suitability::Slow, format!(
      "Your graphics card has {} of video memory and {} GB is comfortable. It will run, but expect slower generation.{}",
      format_gb(vram), needs.nvidia_good_gb, note))
  } else if profile.system_memory_gb >= needs.system_ram_gb as f64 - TOLERANCE_GB {
    rated(Suitability::VerySlow, format!(
      "Your graphics card has only {} of video memory, so parts of the model are swapped through system memory. Expect very slow generation.{}",
      format_gb(vram), note))
  } else {
    rated(Suitability::NotRecommended, format!(
      "With a graphics card this small, this needs at least {} GB of system memory and you have {}.{}",
      needs.system_ram_gb, format_gb(profile.system_memory_gb), note))
  }
}

fn assess_cpu(profile: &HardwareProfile, needs: &Requirements) -> Assessment {
  if profile.system_memory_gb < needs.system_ram_gb as f64 - TOLERANCE_GB {
    return rated(Suitability::NotRecommended, format!(
      "This needs at least {} GB of memory and this computer has {}.",
      needs.system_ram_gb, format_gb(profile.system_memory_gb)));
  }
  match needs.cpu {
    CpuSupport::Slow => rated(Suitability::Slow,
      "There is no supported graphics card, so this runs on the CPU. Expect it to be slow.".to_string()),
    CpuSupport::VerySlow => rated(Suitability::VerySlow,
      "There is no supported graphics card, so this runs on the CPU. Expect it to be very slow, many minutes for each picture.".to_string()),
    CpuSupport::Impractical => rated(Suitability::NotRecommended,
      "This needs a supported graphics card. On the CPU it would take hours.".to_string()),
  }
}

/// Machines report slightly less than their nominal memory (a "16 GB" Mac shows 15.9).
const TOLERANCE_GB: f64 = 0.5;

fn good() -> Assessment {
  rated(Suitability::Good, "This should run well on this computer.".to_string())
}

fn rated(level: Suitability, message: String) -> Assessment {
  Assessment { level, message }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::{ACE_STEP_NEEDS, SD_1_5_NEEDS, SDXL_NEEDS, WAN_5B_NEEDS};
  use Suitability::*;

  fn mac(gb: f64) -> HardwareProfile {
    HardwareProfile { gpu_kind: GpuKind::AppleSilicon, gpu_name: Some("Apple M4".into()), gpu_memory_gb: None, system_memory_gb: gb }
  }

  fn nvidia(name: &str, vram: f64, ram: f64) -> HardwareProfile {
    HardwareProfile { gpu_kind: GpuKind::Nvidia, gpu_name: Some(name.into()), gpu_memory_gb: Some(vram), system_memory_gb: ram }
  }

  fn cpu_only(ram: f64) -> HardwareProfile {
    HardwareProfile { gpu_kind: GpuKind::None, gpu_name: None, gpu_memory_gb: None, system_memory_gb: ram }
  }

  fn level(profile: &HardwareProfile, needs: &Requirements) -> Suitability {
    assess(profile, needs).level
  }

  mod apple_silicon {
    use super::*;

    #[test]
    fn stable_diffusion_1_5_is_fine_on_an_8gb_mac() {
      assert_eq!(level(&mac(8.0), &SD_1_5_NEEDS), Good);
      assert_eq!(level(&mac(7.8), &SD_1_5_NEEDS), Good, "an 8 GB Mac reports a little under 8");
    }

    #[test]
    fn sdxl_wants_16gb() {
      assert_eq!(level(&mac(8.0), &SDXL_NEEDS), NotRecommended);
      assert_eq!(level(&mac(16.0), &SDXL_NEEDS), Good);
      assert_eq!(level(&mac(15.9), &SDXL_NEEDS), Good);
    }

    #[test]
    fn wan_is_slow_on_a_24gb_mac_as_measured_and_good_from_32gb() {
      assert_eq!(level(&mac(16.0), &WAN_5B_NEEDS), NotRecommended);
      assert_eq!(level(&mac(24.0), &WAN_5B_NEEDS), Slow);
      assert_eq!(level(&mac(23.9), &WAN_5B_NEEDS), Slow);
      assert_eq!(level(&mac(32.0), &WAN_5B_NEEDS), Good);
    }

    #[test]
    fn the_message_names_the_memory_you_have_and_what_is_comfortable() {
      let message = assess(&mac(24.0), &WAN_5B_NEEDS).message;
      assert!(message.contains("24 GB") && message.contains("32 GB"), "{message}");
      assert!(message.contains("close other apps"), "{message}");
    }
  }

  mod nvidia_cards {
    use super::*;

    #[test]
    fn a_4gb_card_runs_stable_diffusion_1_5() {
      assert_eq!(level(&nvidia("NVIDIA GeForce RTX 3050", 4.0, 16.0), &SD_1_5_NEEDS), Slow);
      assert_eq!(level(&nvidia("NVIDIA GeForce RTX 3060", 6.0, 16.0), &SD_1_5_NEEDS), Good);
    }

    #[test]
    fn a_4gb_card_swaps_through_ram_for_sdxl() {
      let assessment = assess(&nvidia("NVIDIA GeForce RTX 3050", 4.0, 16.0), &SDXL_NEEDS);
      assert_eq!(assessment.level, VerySlow);
      assert!(assessment.message.contains("swapped through system memory"), "{}", assessment.message);
    }

    #[test]
    fn a_small_card_with_little_ram_is_not_recommended() {
      assert_eq!(level(&nvidia("NVIDIA GeForce RTX 3050", 4.0, 8.0), &SDXL_NEEDS), NotRecommended);
    }

    #[test]
    fn video_needs_a_big_card() {
      assert_eq!(level(&nvidia("NVIDIA GeForce RTX 4090", 24.0, 64.0), &WAN_5B_NEEDS), Good);
      assert_eq!(level(&nvidia("NVIDIA GeForce RTX 4070", 12.0, 32.0), &WAN_5B_NEEDS), VerySlow);
      assert_eq!(level(&nvidia("NVIDIA GeForce RTX 3050", 4.0, 16.0), &WAN_5B_NEEDS), NotRecommended);
    }

    #[test]
    fn gtx_16_series_cards_count_as_having_half_the_memory() {
      // 6 GB would be comfortable for Stable Diffusion 1.5, but not on a card without half precision.
      let gtx = assess(&nvidia("NVIDIA GeForce GTX 1660 Ti", 6.0, 16.0), &SD_1_5_NEEDS);
      assert_eq!(gtx.level, VerySlow);
      assert!(gtx.message.contains("half precision"), "{}", gtx.message);
      assert_eq!(level(&nvidia("NVIDIA GeForce RTX 2060", 6.0, 16.0), &SD_1_5_NEEDS), Good);
    }

    #[test]
    fn the_common_4gb_gtx_1650_is_flagged_for_every_model() {
      let card = nvidia("NVIDIA GeForce GTX 1650", 4.0, 16.0);
      assert_eq!(level(&card, &SD_1_5_NEEDS), VerySlow);
      assert_eq!(level(&card, &SDXL_NEEDS), VerySlow);
      assert_eq!(level(&card, &WAN_5B_NEEDS), NotRecommended);
    }
  }

  mod cpu_only {
    use super::*;

    #[test]
    fn images_work_slowly_and_video_does_not() {
      assert_eq!(level(&cpu_only(16.0), &SD_1_5_NEEDS), Slow);
      assert_eq!(level(&cpu_only(16.0), &SDXL_NEEDS), VerySlow);
      assert_eq!(level(&cpu_only(64.0), &WAN_5B_NEEDS), NotRecommended);
    }

    #[test]
    fn too_little_memory_is_not_recommended() {
      assert_eq!(level(&cpu_only(4.0), &SD_1_5_NEEDS), NotRecommended);
    }
  }

  mod texts {
    use super::*;

    #[test]
    fn minimum_and_recommended_state_the_numbers() {
      let minimum = SDXL_NEEDS.minimum_text();
      assert!(minimum.contains("6 GB video memory") && minimum.contains("16 GB memory on an Apple Silicon Mac"), "{minimum}");
      assert!(minimum.contains("very slowly"), "{minimum}");
      assert!(SDXL_NEEDS.recommended_text().contains("10 GB video memory"));
      assert!(WAN_5B_NEEDS.minimum_text().contains("needs a graphics card"));
    }

    #[test]
    fn recommended_is_never_below_minimum() {
      for needs in [SD_1_5_NEEDS, SDXL_NEEDS, WAN_5B_NEEDS, ACE_STEP_NEEDS] {
        assert!(needs.nvidia_good_gb >= needs.nvidia_workable_gb);
        assert!(needs.apple_good_gb >= needs.apple_workable_gb);
      }
    }
  }
}
