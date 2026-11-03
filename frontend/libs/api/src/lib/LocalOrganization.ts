import type {
  FolderInfo,
  FolderMediaFileListItem,
  FolderThumbnail,
} from "./models/Folder.js";
import type { LocalMediaEntry } from "./LocalLibrary.js";

// Folders and tags, kept on this computer. They only say which files belong together; the files
// themselves live in the media library. The data is a small JSON document in the webview's
// storage, so it survives restarts.

const STORAGE_KEY = "humsafaar.library.v1";
const COVER_SLOTS = 4;

export interface FolderRecord {
  token: string;
  name: string;
  parent: string | null;
  color: string | null;
  star: boolean;
  cover: string | null;
  created_at: string;
  updated_at: string;
}

export interface TagRecord {
  token: string;
  value: string;
}

export interface SceneRecord {
  token: string;
  title: string;
  /** Media token of the picture shown for the scene. */
  cover: string | null;
  created_at: string;
  updated_at: string;
}

export interface OrganizationState {
  folders: Record<string, FolderRecord>;
  /** Folder token to its files, oldest first, each with when it was added. */
  folderFiles: Record<string, { token: string; added: string }[]>;
  tags: Record<string, TagRecord>;
  /** Media token to the tags on it. */
  fileTags: Record<string, string[]>;
  /** Saved scenes. The scene itself is a file in the media folder; this holds its title and cover. */
  scenes: Record<string, SceneRecord>;
}

const emptyState = (): OrganizationState => ({ folders: {}, folderFiles: {}, tags: {}, fileTags: {}, scenes: {} });

let memory: OrganizationState | null = null;

export function loadOrganization(): OrganizationState {
  if (memory) return memory;
  try {
    const text = localStorage.getItem(STORAGE_KEY);
    memory = text ? { ...emptyState(), ...(JSON.parse(text) as Partial<OrganizationState>) } : emptyState();
  } catch {
    memory = emptyState();
  }
  return memory;
}

/** Run a change and save it. Returns what the change returned. */
export function updateOrganization<T>(change: (state: OrganizationState) => T): T {
  const state = loadOrganization();
  const result = change(state);
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // Storage can be unavailable or full; the change still holds for this session.
  }
  return result;
}

/** Forget the in-memory copy so the next read comes from storage. Used by tests. */
export function resetOrganizationCache(): void {
  memory = null;
}

export const newToken = (prefix: string): string => `${prefix}_${crypto.randomUUID().replace(/-/g, "")}`;
export const nowIso = (): string => new Date().toISOString();

// ── Shapes ──

function thumbnailOf(entry: LocalMediaEntry): FolderThumbnail {
  return {
    token: entry.token,
    media_class: entry.media_class,
    media_type: entry.media_class === "dimensional" ? "glb" : entry.media_class,
    cdn_url: entry.url,
    maybe_thumbnail_template: null,
  };
}

export function toFolderInfo(record: FolderRecord, state: OrganizationState, files: Map<string, LocalMediaEntry>): FolderInfo {
  const members = (state.folderFiles[record.token] ?? []).filter((m) => files.has(m.token));
  const recent = members.slice(-COVER_SLOTS).reverse();
  const cover = record.cover ? files.get(record.cover) : undefined;
  return {
    token: record.token,
    name: record.name,
    owner_user_token: "local",
    maybe_parent_folder_token: record.parent,
    last_media_thumbnails: recent.map((m) => thumbnailOf(files.get(m.token) as LocalMediaEntry)),
    maybe_custom_cover_thumbnail: cover ? thumbnailOf(cover) : null,
    maybe_color_code: record.color,
    has_star: record.star,
    is_orphaned: record.parent !== null && !state.folders[record.parent],
    created_at: record.created_at,
    updated_at: record.updated_at,
  };
}

export function toFolderMediaItem(entry: LocalMediaEntry, added: string): FolderMediaFileListItem {
  return {
    token: entry.token,
    media_class: entry.media_class,
    media_type: entry.media_class === "dimensional" ? "glb" : entry.media_class,
    maybe_prompt_token: null,
    maybe_batch_token: null,
    media_links: { cdn_url: entry.url, maybe_thumbnail_template: null },
    maybe_title: null,
    creator_set_visibility: "private",
    is_user_upload: entry.origin_category === "upload",
    created_at: entry.created_at,
    updated_at: entry.created_at,
    added_to_folder_at: added,
  };
}

/** Which folders a file is in. */
export function foldersOfFile(state: OrganizationState, token: string): string[] {
  return Object.keys(state.folderFiles).filter((folder) => state.folderFiles[folder].some((m) => m.token === token));
}
