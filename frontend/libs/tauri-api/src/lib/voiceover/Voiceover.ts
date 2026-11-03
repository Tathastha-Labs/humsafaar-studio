import { invoke } from "@tauri-apps/api/core";
import { CommandResult } from "../common/CommandStatus";

// Rust is the source of truth: `Voice` in the comfyui_manager crate's `tts` module.
export interface VoiceoverVoice {
  id: string;
  label: string;
  language: string;
  language_label: string;
  gender: "female" | "male";
}

// Rust is the source of truth: `VoiceoverInstall` in the app's `voiceover_state` module.
export type VoiceoverInstallState =
  | { state: "idle" }
  | { state: "installing"; label: string; detail?: string | null; fraction?: number | null }
  | { state: "failed"; message: string };

export interface VoiceoverGetStatusSuccess extends CommandResult {
  payload: {
    installed: boolean;
    install: VoiceoverInstallState;
    voices: VoiceoverVoice[];
    download_bytes: number;
    max_text_chars: number;
    min_speed: number;
    max_speed: number;
  };
}

export interface VoiceoverGenerateSuccess extends CommandResult {
  payload: {
    media_token: string;
    /** A URL the webview can load the audio from. */
    url: string;
    seconds: number;
  };
}

export const VoiceoverGetStatus = async (): Promise<VoiceoverGetStatusSuccess> => {
  return await invoke("voiceover_get_status_command") as VoiceoverGetStatusSuccess;
}

/** Downloads the voice runtime and model. Returns at once; follow it with `VoiceoverGetStatus`. */
export const VoiceoverInstall = async (): Promise<CommandResult> => {
  return await invoke("voiceover_install_command") as CommandResult;
}

/** Speaks the script and saves it locally. Takes a few seconds. */
export const VoiceoverGenerate = async (
  request: { text: string; voice_id: string; speed?: number },
): Promise<VoiceoverGenerateSuccess> => {
  return await invoke("voiceover_generate_command", { request }) as VoiceoverGenerateSuccess;
}

export const GROQ_VOICES = ["autumn", "diana", "hannah", "austin", "daniel", "troy"] as const;

/** Speaks the script with the user's Groq key and saves it locally. The voice model needs the Groq account owner to accept its terms once. */
export const GroqVoiceoverGenerate = async (
  text: string,
  voice?: (typeof GROQ_VOICES)[number],
): Promise<VoiceoverGenerateSuccess> => {
  return await invoke("groq_voiceover_generate_command", { request: { text, voice } }) as VoiceoverGenerateSuccess;
}

/** Turns a recording on this computer into text with the user's Groq key. */
export const TranscribeAudio = async (filePath: string): Promise<string> => {
  const result = await invoke("transcribe_audio_command", { request: { file_path: filePath } }) as { payload: { text: string } };
  return result.payload.text;
}
