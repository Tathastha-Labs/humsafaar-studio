import type { VoiceoverAdapter } from "@storyteller/ui-video-editor";
import {
  VoiceoverGenerate,
  VoiceoverGetStatus,
  VoiceoverInstall,
} from "@storyteller/tauri-api";

// Voiceover is made on this computer by the Rust side; see `voiceover_*_command`.
export const tauriVoiceoverAdapter: VoiceoverAdapter = {
  async getStatus() {
    const { payload } = await VoiceoverGetStatus();
    return {
      installed: payload.installed,
      install: payload.install,
      voices: payload.voices.map((voice) => ({
        id: voice.id,
        label: voice.label,
        language: voice.language,
        languageLabel: voice.language_label,
        gender: voice.gender,
      })),
      downloadBytes: payload.download_bytes,
      maxTextChars: payload.max_text_chars,
      minSpeed: payload.min_speed,
      maxSpeed: payload.max_speed,
    };
  },

  async install() {
    await VoiceoverInstall();
  },

  async generate({ text, voiceId, speed }) {
    const { payload } = await VoiceoverGenerate({ text, voice_id: voiceId, speed });
    return { url: payload.url, seconds: payload.seconds };
  },
};
