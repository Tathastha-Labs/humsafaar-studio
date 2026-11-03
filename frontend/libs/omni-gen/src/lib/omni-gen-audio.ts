import type {
  OmniGenAudioModelDetails,
  OmniGenAudioRequest,
  OmniGenAudioGenerateResponse,
  OmniGenMusicalKey,
} from "@storyteller/api";

// Audio request building and enqueue. The page injects its Tauri transport.

// Server rule (validate_audio_request): these models require exactly one
// audio reference (the remix/sample source). The models API has no
// "required" flag, so mirror it by id.
export const AUDIO_MODELS_REQUIRING_AUDIO_REF = new Set([
  "suno_remix",
  "suno_sample",
]);

// ── Request params ───────────────────────────────────────────────────────

// Raw UI state. buildAudioRequest gates every field on the selected model's
// capability flags so unsupported settings never reach the API.
export interface AudioGenerationSettings {
  prompt: string;
  stylePrompt: string;
  audioMediaTokens: string[];
  imageMediaTokens: string[];
  isInstrumental: boolean;
  keepLyrics: boolean;
  isLoopable: boolean;
  bpm: number | null;
  musicalKey: string;
  sampleRateHz: number | null;
  speed: number;
  volume: number;
  pitch: number;
}

// ── Request builder ──────────────────────────────────────────────────────

// Capability flags are serde-skipped when absent, so only `=== true` counts
// as supported.
const supports = (flag: boolean | null | undefined): boolean => flag === true;

export function buildAudioRequest(
  model: OmniGenAudioModelDetails,
  settings: AudioGenerationSettings,
): OmniGenAudioRequest {
  return {
    model: model.model,
    idempotency_token: crypto.randomUUID(),
    prompt: settings.prompt.trim() || null,
    style_prompt:
      supports(model.style_prompt_supported) && settings.stylePrompt.trim()
        ? settings.stylePrompt.trim()
        : null,
    audio_media_tokens:
      supports(model.audio_references_supported) &&
      settings.audioMediaTokens.length
        ? settings.audioMediaTokens
        : null,
    image_media_tokens:
      supports(model.image_references_supported) &&
      settings.imageMediaTokens.length
        ? settings.imageMediaTokens
        : null,
    keep_lyrics: supports(model.keep_lyrics_supported)
      ? settings.keepLyrics
      : null,
    is_instrumental: supports(model.instrumental_toggle_supported)
      ? settings.isInstrumental
      : null,
    is_loopable: supports(model.loopable_toggle_supported)
      ? settings.isLoopable
      : null,
    bpm:
      supports(model.bpm_supported) && settings.bpm !== null
        ? settings.bpm
        : null,
    musical_key: supports(model.musical_key_supported)
      ? (settings.musicalKey as OmniGenMusicalKey)
      : null,
    sample_rate_hz: model.sample_rate_hz_options?.length
      ? settings.sampleRateHz
      : null,
    speed: supports(model.speed_supported) ? settings.speed : null,
    volume: supports(model.volume_supported) ? settings.volume : null,
    pitch: supports(model.pitch_supported) ? settings.pitch : null,
  };
}

// ── Enqueue generation ───────────────────────────────────────────────────

export async function enqueueAudioGeneration(
  model: OmniGenAudioModelDetails,
  settings: AudioGenerationSettings,
  generate: (body: OmniGenAudioRequest) => Promise<OmniGenAudioGenerateResponse>,
): Promise<{
  success: boolean;
  // One request can create several jobs (Suno-style multi-clip) — callers
  // must treat every token as its own pending generation.
  jobTokens: string[];
  error?: string;
  errorCode?: number;
}> {
  const body = buildAudioRequest(model, settings);

  try {
    const response = await generate(body);
    if (response.success && response.inference_job_token) {
      const jobTokens = response.all_job_tokens?.length
        ? response.all_job_tokens
        : [response.inference_job_token];
      return { success: true, jobTokens };
    }
    return { success: false, jobTokens: [], error: "Generation failed" };
  } catch (err: any) {
    return {
      success: false,
      jobTokens: [],
      error: err.error_message ?? err.message ?? "Request failed",
      errorCode: parseAudioHttpStatusCode(err),
    };
  }
}

// Pull the HTTP status out of an ApiManager error. ApiManager throws
// `Error("HTTP error! status: 402")` on a non-2xx response (the JSON body is
// not surfaced), so callers can special-case codes like 402 Payment Required.
export function parseAudioHttpStatusCode(err: unknown): number | undefined {
  const message = err instanceof Error ? err.message : String(err);
  const match = /status:\s*(\d+)/.exec(message);
  return match ? Number(match[1]) : undefined;
}

// ── Cost estimate hook ───────────────────────────────────────────────────

/** Shown when starting a generation fails for a reason the page cannot name. */
export const OMNI_GENERATE_OUTAGE_MESSAGE =
  "Couldn't start generation. Please try again shortly.";
