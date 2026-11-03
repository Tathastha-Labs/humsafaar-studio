import { invoke } from "@tauri-apps/api/core";
import type { MediaFile } from "./models/MediaFile.js";
import { saveLocalFile, toBase64, listLocalMedia, toMediaFile } from "./LocalLibrary.js";
import { loadOrganization, nowIso, updateOrganization, type SceneRecord } from "./LocalOrganization.js";

// Saved scenes. A scene is a text file in the media folder (so it is kept with everything else
// and can be backed up by copying the folder); its title and cover picture are listed here.

const SCENE_EXTENSION = "scene";
const UNTITLED = "Untitled Scene";

interface ReadTextResult {
  payload?: { text?: string };
}

/**
 * Save a scene. With a token it saves over that scene, otherwise it makes a new one.
 * Returns the scene's token, or an empty string if it could not be saved.
 */
export async function saveScene(args: {
  json: string;
  title: string;
  token?: string;
  cover?: Blob | null;
}): Promise<string> {
  const title = args.title.trim() || UNTITLED;
  try {
    const known = args.token ? loadOrganization().scenes[args.token] : undefined;
    let token: string;
    if (args.token && known) {
      await invoke("local_media_replace_command", {
        request: { media_file_token: args.token, base64_data: await toBase64(new Blob([args.json])) },
      });
      token = args.token;
    } else {
      token = await saveLocalFile(new File([args.json], `scene.${SCENE_EXTENSION}`, { type: "application/json" }));
    }

    const cover = args.cover ? await saveLocalFile(new File([args.cover], "cover.png", { type: "image/png" })) : null;
    updateOrganization((state) => {
      const now = nowIso();
      const record: SceneRecord = state.scenes[token] ?? { token, title, cover: null, created_at: now, updated_at: now };
      record.title = title;
      record.updated_at = now;
      if (cover) record.cover = cover;
      state.scenes[token] = record;
    });
    return token;
  } catch {
    return "";
  }
}

/** A saved scene's contents, and its title. */
export async function loadScene(token: string): Promise<{ scene: unknown; title: string }> {
  const result = (await invoke("local_media_read_text_command", {
    request: { media_file_token: token },
  })) as ReadTextResult;
  const text = result?.payload?.text;
  if (text === undefined) throw new Error("Failed to load scene");
  const title = loadOrganization().scenes[token]?.title ?? UNTITLED;
  return { scene: JSON.parse(text), title };
}

export function renameScene(token: string, title: string): boolean {
  const name = title.trim();
  if (!name) return false;
  return updateOrganization((state) => {
    const record = state.scenes[token];
    if (!record) return false;
    record.title = name;
    record.updated_at = nowIso();
    return true;
  });
}

export function sceneTitle(token: string): string | undefined {
  return loadOrganization().scenes[token]?.title;
}

/** Every saved scene, most recently saved first, shaped like a library file. */
export async function listScenes(): Promise<MediaFile[]> {
  const state = loadOrganization();
  const covers = new Map((await listLocalMedia().catch(() => [])).map((entry) => [entry.token, entry.url]));
  // Latest-made first, so scenes saved in the same instant still list newest first.
  return Object.values(state.scenes)
    .reverse()
    .sort((a, b) => b.updated_at.localeCompare(a.updated_at))
    .map((record) => {
      const file = toMediaFile({
        token: record.token,
        url: "",
        media_class: "dimensional",
        origin_category: "upload",
        created_at: record.created_at,
        size_bytes: 0,
        maybe_model_type: null,
      });
      const cover = record.cover ? covers.get(record.cover) : undefined;
      return {
        ...file,
        media_type: "scene_json" as unknown as MediaFile["media_type"],
        media_class: null,
        maybe_title: record.title,
        public_bucket_path: "",
        public_bucket_url: "",
        media_links: { cdn_url: cover ?? "", thumbnail_template: "" },
        cover_image: {
          maybe_cover_image_public_bucket_path: cover ?? null,
          maybe_links: cover ? { cdn_url: cover } : null,
          default_cover: { image_index: 0, color_index: 0 },
        } as unknown as MediaFile["cover_image"],
        updated_at: record.updated_at,
      };
    });
}
