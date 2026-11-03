import { GetAppPreferences } from "@storyteller/tauri-api";
import { SoundEffect } from "./SoundEffect";
import { SoundRegistry } from "./SoundRegistry";

type SoundDef = {
  key: string;
  path: string;
  volume: number;
  // Present iff the sound is user-selectable in Settings → Alerts.
  // Sounds without a label are still registered and can be played by key.
  label?: string;
};

const SOUNDS: readonly SoundDef[] = [
  // Menu choices
  { key: "click", path: "resources/audio/ui/click.wav", volume: 0.2, label: "Click" },
  { key: "scifi_menu_beep_1", path: "resources/audio/ui/scifi_menu_beep_1.wav", volume: 0.3, label: "Sci-Fi Menu Beep 1" },
  { key: "scifi_menu_beep_2", path: "resources/audio/ui/scifi_menu_beep_2.wav", volume: 0.3, label: "Sci-Fi Menu Beep 2" },
  { key: "scifi_menu_select", path: "resources/audio/ui/scifi_menu_select.wav", volume: 0.3, label: "Sci-Fi Menu Select" },
  // Immediate enqueue success
  { key: "done", path: "resources/audio/ui/done.wav", volume: 0.4, label: "Dialog Done" },
  // Immediate failure
  { key: "error_chirp", path: "resources/audio/ui/error_chirp.wav", volume: 0.4, label: "Error Chirp" },
  { key: "spike_throw", path: "resources/audio/ui/spike_throw.wav", volume: 0.1, label: "Spike Throw" },
  { key: "giant_shell_kick", path: "resources/audio/ui/giant_shell_kick.wav", volume: 0.2, label: "Shell Kick" },
  { key: "wrong", path: "resources/audio/ui/wrong.wav", volume: 0.4, label: "Wrong" },
  // Async success
  { key: "special_flower", path: "resources/audio/ui/special_flower.wav", volume: 0.2, label: "Special Flower" },
  { key: "extra_power", path: "resources/audio/ui/extra_power.wav", volume: 0.2 },
  // Async errors
  { key: "crumble", path: "resources/audio/ui/crumble.wav", volume: 0.1, label: "Crumble" },
  { key: "ghost", path: "resources/audio/ui/ghost.wav", volume: 0.2, label: "Ghost" },
  { key: "special_alert", path: "resources/audio/ui/special_alert.wav", volume: 0.2, label: "Special Alert" },
  { key: "scifi_alert", path: "resources/audio/ui/scifi_alert.wav", volume: 0.2, label: "Sci-Fi Alert" },
  { key: "scifi_shrill_alert", path: "resources/audio/ui/scifi_shrill_alert.wav", volume: 0.2, label: "Sci-Fi Shrill Alert" },
  // Menus
  { key: "next", path: "resources/audio/ui/next.wav", volume: 0.2, label: "Dialog Next" },
  { key: "select", path: "resources/audio/ui/select.wav", volume: 0.4, label: "Select" },
  { key: "scifi_menu_open", path: "resources/audio/ui/scifi_menu_open.wav", volume: 0.3, label: "Sci-Fi Menu Open" },
  { key: "scifi_menu_close", path: "resources/audio/ui/scifi_menu_close.wav", volume: 0.3, label: "Sci-Fi Menu Close" },
  // Reward / celebration
  { key: "correct", path: "resources/audio/ui/correct.wav", volume: 0.1, label: "Correct" },
  { key: "flower", path: "resources/audio/ui/flower.wav", volume: 0.1, label: "Flower" },
  // Trash / delete
  { key: "trash", path: "resources/audio/ui/trash.wav", volume: 0.4, label: "Trash" },
  // Misc — registered but not user-selectable
  { key: "accept_chirp", path: "resources/audio/ui/accept_chirp.wav", volume: 0.2 },
  { key: "accept_normal_level_1", path: "resources/audio/ui/accept_normal_level_1.wav", volume: 0.2 },
  { key: "accept_normal_level_2", path: "resources/audio/ui/accept_normal_level_2.wav", volume: 0.2 },
  { key: "accept_normal_level_3", path: "resources/audio/ui/accept_normal_level_3.wav", volume: 0.2 },
  { key: "decline_chirp", path: "resources/audio/ui/decline_chirp.wav", volume: 0.2 },
  { key: "decline_normal", path: "resources/audio/ui/decline_normal.wav", volume: 0.2 },
];

export type SoundOption = { value: string; label: string };

type SoundPrefKey =
  | "delete_file_sound"
  | "enqueue_success_sound"
  | "enqueue_failure_sound"
  | "generation_success_sound"
  | "generation_failure_sound";

export class SoundManager {
  // Dropdown options for AudioSettingsPane.
  // "None (Silent)" first, then user-selectable sounds A→Z by label.
  static readonly OPTIONS: SoundOption[] = [
    { value: "none", label: "None (Silent)" },
    ...SOUNDS
      .filter((s): s is SoundDef & { label: string } => !!s.label)
      .map((s) => ({ value: s.key, label: s.label }))
      .sort((a, b) => a.label.localeCompare(b.label)),
  ];

  private static installed = false;

  // Register the sound catalog. Idempotent — safe to call twice.
  static install() {
    if (this.installed) return;
    const r = SoundRegistry.getInstance();
    for (const s of SOUNDS) {
      r.setSoundOnce(s.key, new SoundEffect(s.path, { defaultVolume: s.volume }));
    }
    this.installed = true;
  }

  // Settings preview — fires regardless of the master toggle.
  static playPreview(soundName: string) {
    if (!soundName || soundName === "none") return;
    SoundRegistry.getInstance().playSound(soundName);
  }

  // Event-driven playback — gated on `play_sounds`.
  static async playFileDeleted()       { await this.playEvent("delete_file_sound"); }
  static async playEnqueueSuccess()    { await this.playEvent("enqueue_success_sound"); }
  static async playEnqueueFailure()    { await this.playEvent("enqueue_failure_sound"); }
  static async playGenerationSuccess() { await this.playEvent("generation_success_sound"); }
  static async playGenerationFailure() { await this.playEvent("generation_failure_sound"); }

  private static async playEvent(prefKey: SoundPrefKey) {
    const prefs = (await GetAppPreferences()).preferences;
    if (!prefs?.play_sounds) return;
    const soundName = prefs[prefKey];
    if (!soundName) return;
    SoundRegistry.getInstance().playSound(soundName);
  }
}
