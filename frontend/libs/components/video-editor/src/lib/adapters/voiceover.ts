// Optional slot for a host that can turn a script into speech on the user's computer. When it is
// absent the editor hides the Voiceover tab. The host owns the voices, the model and where the
// audio is stored; the editor only asks for speech and drops the result on the timeline.

export interface VoiceoverVoice {
  id: string;
  label: string;
  language: string;
  languageLabel: string;
  gender: "female" | "male";
}

export type VoiceoverInstallState =
  | { state: "idle" }
  | { state: "installing"; label: string; detail?: string | null; fraction?: number | null }
  | { state: "failed"; message: string };

export interface VoiceoverStatus {
  installed: boolean;
  install: VoiceoverInstallState;
  voices: VoiceoverVoice[];
  downloadBytes: number;
  maxTextChars: number;
  minSpeed: number;
  maxSpeed: number;
}

export interface VoiceoverClip {
  // Loadable from an <audio> element and from fetch().
  url: string;
  seconds: number;
}

export interface VoiceoverAdapter {
  getStatus(): Promise<VoiceoverStatus>;
  // Starts the one-time download and returns at once; poll `getStatus` to follow it.
  install(): Promise<void>;
  generate(opts: { text: string; voiceId: string; speed: number }): Promise<VoiceoverClip>;
}
