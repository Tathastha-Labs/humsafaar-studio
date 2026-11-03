
// ── Shared enums (match backend) ─────────────────────────────────────────

// These are the string values the omni-gen API accepts/returns.
// They align with CommonAspectRatio and CommonResolution on the frontend.

// ── Image request / response types ───────────────────────────────────────

// ── Video request / response types ───────────────────────────────────────

// Frontend-supplied hints for cost estimation only. IGNORED by generation —
// the backend measures inputs itself (e.g. ffprobing reference videos) when
// it bills. Seedance 2.5 bills reference-video input seconds, so its cost
// quote needs the combined input duration.
// ── Audio request / response types ───────────────────────────────────────

// Audio model ids (match backend CommonAudioModel).
export type OmniGenAudioModelId =
  | "suno_music"
  | "suno_remix"
  | "suno_sounds"
  | "suno_sample"
  | "seed_audio_1p0";

// Musical keys for audio generation (eg. Suno Sounds).
// NB: There are intentionally no E keys, per product spec.
export type OmniGenMusicalKey =
  | "auto"
  | "c_major"
  | "c_minor"
  | "d_major"
  | "d_minor"
  | "f_major"
  | "f_minor"
  | "g_major"
  | "g_minor"
  | "a_major"
  | "a_minor"
  | "b_major"
  | "b_minor";

/** Shared request body for both the audio cost estimate and audio
 *  generation endpoints. */
export interface OmniGenAudioRequest {
  // REQUIRED (even if marked optional) — prevents duplicate requests.
  idempotency_token?: string | null;
  // REQUIRED (even if marked optional) — which model to use.
  model?: OmniGenAudioModelId | string | null;
  prompt?: string | null;
  // Style/genre direction (Suno's "tags").
  style_prompt?: string | null;
  // The remix/sample source, or Seed Audio @Audio references (up to 3).
  audio_media_tokens?: string[] | null;
  // Seed Audio supports a single reference image; cannot be combined with
  // audio references.
  image_media_tokens?: string[] | null;
  // Whether to keep the original lyrics (Suno Remix).
  keep_lyrics?: boolean | null;
  // Whether to generate instrumental-only audio (Suno Music / Sample).
  is_instrumental?: boolean | null;
  // Whether the sound should loop vs a single hit (Suno Sounds).
  is_loopable?: boolean | null;
  // Beats per minute (Suno Sounds).
  bpm?: number | null;
  // The musical key to use (Suno Sounds).
  musical_key?: OmniGenMusicalKey | null;
  // Output sample rate in Hz (Seed Audio: 8000/16000/24000/32000/44100/48000).
  sample_rate_hz?: number | null;
  // Playback speed multiplier (Seed Audio: 0.5–2.0).
  speed?: number | null;
  // Volume multiplier (Seed Audio: 0.5–2.0).
  volume?: number | null;
  // Pitch shift in semitones (Seed Audio: -12..=12).
  pitch?: number | null;
}

export interface OmniGenAudioGenerateResponse {
  success: boolean;
  inference_job_token: string;
  // All job tokens created by this request (including the primary). One
  // request may create multiple jobs (eg. Suno multi-clip) — poll them all.
  all_job_tokens: string[];
}

// ── Image model info (from GET /v1/omni_gen/models/image) ────────────────

// ── Video model info (from GET /v1/omni_gen/models/video) ────────────────

// ── Audio model info (from GET /v1/omni_gen/models/audio) ────────────────

// Unlike the image/video model infos, absent capabilities are omitted from
// the JSON entirely (serde skip_serializing_if), so every field except
// `model` is optional. Treat undefined as unsupported.
export interface OmniGenAudioModelDetails {
  model: OmniGenAudioModelId | string;
  // ModelCreator enum as a snake_case string, e.g. "suno", "bytedance".
  model_creator?: string | null;
  full_name?: string | null;
  extra_info?: string | null;
  extra_info_short?: string | null;
  text_prompt_supported?: boolean | null;
  // Whether a style/genre prompt (Suno's "tags") is supported.
  style_prompt_supported?: boolean | null;
  audio_references_supported?: boolean | null;
  audio_references_max?: number | null;
  image_references_supported?: boolean | null;
  image_references_max?: number | null;
  // Whether the "keep lyrics" toggle is supported (Suno Remix).
  keep_lyrics_supported?: boolean | null;
  // Whether the instrumental-only toggle is supported (Suno Music / Sample).
  instrumental_toggle_supported?: boolean | null;
  // Whether the loop vs single-hit toggle is supported (Suno Sounds).
  loopable_toggle_supported?: boolean | null;
  bpm_supported?: boolean | null;
  musical_key_supported?: boolean | null;
  sample_rate_hz_options?: number[] | null;
  sample_rate_hz_default?: number | null;
  speed_supported?: boolean | null;
  volume_supported?: boolean | null;
  pitch_supported?: boolean | null;
  is_disabled?: boolean | null;
}

// ── Provider types (shared by image and video model responses) ───────────

// ── Mesh (3D object) request / response types ────────────────────────────

// Shared body for POST /v1/omni_gen/cost/mesh and /v1/omni_gen/generate/mesh.
export interface OmniGenMeshRequest {
  // Required despite being nullable (backend rejects when absent).
  model: string;
  idempotency_token?: string | null;
  prompt?: string | null;
  // Primary/front input image, or the sketch image for sketch-to-3D.
  reference_image_media_tokens?: string[] | null;
  // Multi-view input (one image per side).
  front_image_media_token?: string | null;
  back_image_media_token?: string | null;
  left_image_media_token?: string | null;
  right_image_media_token?: string | null;
  // Input mesh file for mesh-to-mesh models (part splitting, retopology).
  input_mesh_media_token?: string | null;
  // "normal" | "low_poly" | "geometry"
  mesh_output_type?: string | null;
  // "triangle" | "quad"
  polygon_type?: string | null;
  face_count?: number | null;
  enable_pbr?: boolean | null;
  enable_texture?: boolean | null;
  // "standard" | "detailed"
  texture_quality?: string | null;
  geometry_quality?: string | null;
}

export interface OmniGenMeshGenerateResponse {
  success: boolean;
  inference_job_token: string;
  all_job_tokens: string[];
}

// ── Splat (3D world) request / response types ────────────────────────────

// Shared body for POST /v1/omni_gen/cost/splat and /v1/omni_gen/generate/splat.
export interface OmniGenSplatRequest {
  // Required despite being nullable (backend rejects when absent).
  model: string;
  idempotency_token?: string | null;
  prompt?: string | null;
  // Multi-view input of the same world.
  reference_image_media_tokens?: string[] | null;
  reference_video_media_token?: string | null;
  // Whether the single reference image is a 360-degree panorama.
  is_panoramic?: boolean | null;
  disable_recaption?: boolean | null;
}

export interface OmniGenSplatGenerateResponse {
  success: boolean;
  inference_job_token: string;
  all_job_tokens: string[];
}

// ── Helpers ──────────────────────────────────────────────────────────────

// ── API class ────────────────────────────────────────────────────────────
