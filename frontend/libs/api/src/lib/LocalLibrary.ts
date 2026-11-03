import { invoke } from "@tauri-apps/api/core";
import type { MediaFile } from "./models/MediaFile.js";
import { FilterMediaClasses } from "./enums/QueryFilters.js";

// The library is everything saved on this computer. Rust lists it (`local_media_list_command`);
// this turns each entry into the `MediaFile` shape the pages already read.

/** One file as Rust reports it. */
export interface LocalMediaEntry {
  token: string;
  url: string;
  media_class: "image" | "video" | "audio" | "dimensional";
  origin_category: "upload" | "inference";
  created_at: string;
  size_bytes: number;
  maybe_model_type: string | null;
}

interface LocalMediaListResult {
  payload?: { files?: LocalMediaEntry[] };
}

const isTauri = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Every file in the library, newest first. Empty outside the desktop app. */
export async function listLocalMedia(): Promise<LocalMediaEntry[]> {
  if (!isTauri()) return [];
  const result = (await invoke("local_media_list_command")) as LocalMediaListResult;
  return result?.payload?.files ?? [];
}

/** Whether a file counts under a class filter. A 3D file matches the old and new 3D classes. */
export function matchesClassFilter(
  entry: LocalMediaEntry,
  filters: FilterMediaClasses[] | undefined,
): boolean {
  if (!filters || filters.length === 0) return true;
  if (entry.media_class === "dimensional") {
    return filters.includes(FilterMediaClasses.DIMENSIONAL) || filters.includes(FilterMediaClasses.MESH);
  }
  return filters.includes(entry.media_class as unknown as FilterMediaClasses);
}

/** Entries as the `MediaFile` shape. Fields that only a hosted account had are left empty. */
export function toMediaFile(entry: LocalMediaEntry): MediaFile {
  return {
    token: entry.token,
    media_type: (entry.media_class === "dimensional" ? "glb" : entry.media_class) as unknown as MediaFile["media_type"],
    media_class: entry.media_class as unknown as MediaFile["media_class"],
    maybe_animation_type: null,
    maybe_engine_extension: null,
    maybe_batch_token: "",
    maybe_original_filename: null,
    maybe_creator_user: null,
    maybe_prompt_token: null,
    origin: {
      origin_category: entry.origin_category,
      product_category: entry.media_class,
      maybe_model: entry.maybe_model_type ? { title: entry.maybe_model_type } : null,
    },
    origin_category: entry.origin_category,
    origin_product_category: entry.media_class,
    maybe_origin_model_type: entry.maybe_model_type,
    maybe_origin_model_token: null,
    maybe_duration_millis: null,
    maybe_style_name: null,
    // Local files have no bucket; the full URL stands in, and the CDN origin is empty, so it joins up.
    public_bucket_path: entry.url,
    public_bucket_url: entry.url,
    // A local file is its own thumbnail: the picture itself, or a video that plays its first frame.
    media_links: {
      cdn_url: entry.url,
      thumbnail_template: "",
      ...(entry.media_class === "image" ? { maybe_thumbnail_template: entry.url } : {}),
      ...(entry.media_class === "video" ? { maybe_video_previews: { animated: entry.url } } : {}),
    } as MediaFile["media_links"],
    cover_image: {
      maybe_cover_image_public_bucket_path: null,
      default_cover: { image_index: 0, color_index: 0 },
    },
    creator_set_visibility: "private",
    maybe_title: null,
    maybe_text_transcript: null,
    stats: { positive_rating_count: 0, bookmark_count: 0 },
    maybe_model_weight_info: undefined as unknown as MediaFile["maybe_model_weight_info"],
    created_at: entry.created_at,
    updated_at: entry.created_at,
  };
}

const fileExtension = (file: File): string => {
  const dot = file.name.lastIndexOf(".");
  const fromName = dot >= 0 ? file.name.slice(dot + 1).toLowerCase() : "";
  if (/^[a-z0-9]{1,5}$/.test(fromName)) return fromName;
  return file.type.split("/")[1] || "png";
};

export const toBase64 = async (file: Blob): Promise<string> => {
  const bytes = new Uint8Array(await file.arrayBuffer());
  const chunkSize = 0x8000;
  let binary = "";
  for (let i = 0; i < bytes.length; i += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunkSize));
  }
  return btoa(binary);
};

/** Saves a file in the app's own media folder and returns its token. */
export const saveLocalFile = async (file: File): Promise<string> => {
  const result = (await invoke("local_media_import_command", {
    request: { base64_data: await toBase64(file), extension: fileExtension(file) },
  })) as { payload?: { media_file_token?: string } };

  const token = result.payload?.media_file_token;
  if (!token) throw new Error("The app did not return a token for the saved file.");
  return token;
};

