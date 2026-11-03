import type { ApiResponse } from "./ApiManager.js";
import type { FolderInfo, FolderMediaFileListItem } from "./models/Folder.js";
import { listLocalMedia, type LocalMediaEntry } from "./LocalLibrary.js";
import {
  foldersOfFile,
  loadOrganization,
  newToken,
  nowIso,
  toFolderInfo,
  toFolderMediaItem,
  updateOrganization,
  type FolderRecord,
} from "./LocalOrganization.js";

// ─── Request types ────────────────────────────────────────────────────────────

export interface CreateFolderRequest {
  name: string;
  /** Omit (or null) to create a root-level folder. */
  maybe_parent_folder_token?: string | null;
  maybe_color_code?: string | null;
}

/** Paging for the list calls. Everything is on this computer, so a list comes back whole. */
export interface ListFoldersQuery {
  cursor?: string;
  limit?: number;
}

export interface FolderCursor {
  maybe_cursor?: string | null;
}

export interface MoveMediaFilesResult {
  accepted_media_file_tokens: string[];
  added_to_destination_count: number;
  removed_from_source_count: number;
}

const NO_CURSOR: FolderCursor = { maybe_cursor: null };
const MISSING = "That folder does not exist.";

// ─── API ───────────────────────────────────────────────────────────────────────

/**
 * Folders, kept on this computer. Folders nest (a folder lives under at most one parent) and a
 * file can be in any number of folders. Every call is idempotent.
 */
export class FoldersApi {
  // ── Create / read / delete ──

  public async CreateFolder(params: CreateFolderRequest): Promise<ApiResponse<FolderInfo>> {
    const name = params.name.trim();
    if (!name) return { success: false, errorMessage: "Give the folder a name." };
    const parent = params.maybe_parent_folder_token ?? null;

    const record = updateOrganization((state): FolderRecord | null => {
      if (parent && !state.folders[parent]) return null;
      const now = nowIso();
      const created: FolderRecord = {
        token: newToken("folder"), name, parent, color: params.maybe_color_code ?? null,
        star: false, cover: null, created_at: now, updated_at: now,
      };
      state.folders[created.token] = created;
      state.folderFiles[created.token] = [];
      return created;
    });
    if (!record) return { success: false, errorMessage: MISSING };
    return { success: true, data: await this.info(record.token) };
  }

  public async GetFolder({ folderToken }: { folderToken: string }): Promise<ApiResponse<FolderInfo>> {
    if (!loadOrganization().folders[folderToken]) return { success: false, errorMessage: MISSING };
    return { success: true, data: await this.info(folderToken) };
  }

  /** Delete a folder. Its sub-folders stay and become orphaned; the files stay in the library. */
  public async DeleteFolder({ folderToken }: { folderToken: string }): Promise<ApiResponse<undefined>> {
    updateOrganization((state) => {
      delete state.folders[folderToken];
      delete state.folderFiles[folderToken];
    });
    return { success: true };
  }

  public async ListAllFolders(_query: ListFoldersQuery = {}): Promise<ApiResponse<FolderInfo[], FolderCursor>> {
    const state = loadOrganization();
    const files = await this.files();
    const folders = Object.values(state.folders)
      .sort((a, b) => a.created_at.localeCompare(b.created_at))
      .map((record) => toFolderInfo(record, state, files));
    return { success: true, data: folders, pagination: NO_CURSOR };
  }

  // ── Look ──

  public async SetColorCode({ folderToken, colorCode }: { folderToken: string; colorCode: string | null }): Promise<ApiResponse<undefined>> {
    return this.change(folderToken, (record) => { record.color = colorCode; });
  }

  public async SetCoverImage({ folderToken, mediaFileToken }: { folderToken: string; mediaFileToken: string | null }): Promise<ApiResponse<string | null>> {
    const result = this.change(folderToken, (record) => { record.cover = mediaFileToken; });
    return { ...result, data: mediaFileToken };
  }

  public async RenameFolder({ folderToken, newName }: { folderToken: string; newName: string }): Promise<ApiResponse<undefined>> {
    const name = newName.trim();
    if (!name) return { success: false, errorMessage: "Give the folder a name." };
    return this.change(folderToken, (record) => { record.name = name; });
  }

  public async SetStar({ folderToken, hasStar }: { folderToken: string; hasStar: boolean }): Promise<ApiResponse<undefined>> {
    return this.change(folderToken, (record) => { record.star = hasStar; });
  }

  // ── Nesting ──

  public async ListSubfolders({ folderToken }: { folderToken: string; query?: ListFoldersQuery }): Promise<ApiResponse<FolderInfo[], FolderCursor>> {
    const state = loadOrganization();
    const files = await this.files();
    const children = Object.values(state.folders)
      .filter((record) => record.parent === folderToken)
      .map((record) => toFolderInfo(record, state, files));
    return { success: true, data: children, pagination: NO_CURSOR };
  }

  public async AddSubfolders({ folderToken, subfolderTokens }: { folderToken: string; subfolderTokens: string[] }): Promise<ApiResponse<string[]>> {
    const accepted = updateOrganization((state) => {
      if (!state.folders[folderToken]) return [];
      return subfolderTokens.filter((token) => {
        const child = state.folders[token];
        if (!child || token === folderToken || this.isAncestor(state.folders, token, folderToken)) return false;
        child.parent = folderToken;
        child.updated_at = nowIso();
        return true;
      });
    });
    return { success: true, data: accepted };
  }

  public async RemoveSubfolders({ folderToken, subfolderTokens }: { folderToken: string; subfolderTokens: string[] }): Promise<ApiResponse<number>> {
    const removed = updateOrganization((state) => {
      let count = 0;
      for (const token of subfolderTokens) {
        const child = state.folders[token];
        if (child && child.parent === folderToken) {
          child.parent = null;
          child.updated_at = nowIso();
          count += 1;
        }
      }
      return count;
    });
    return { success: true, data: removed };
  }

  // ── Files ──

  public async ListFolderMediaFiles({ folderToken }: { folderToken: string; query?: ListFoldersQuery }): Promise<ApiResponse<FolderMediaFileListItem[], FolderCursor>> {
    const files = await this.files();
    const members = (loadOrganization().folderFiles[folderToken] ?? []).filter((m) => files.has(m.token));
    const items = members.slice().reverse().map((m) => toFolderMediaItem(files.get(m.token) as LocalMediaEntry, m.added));
    return { success: true, data: items, pagination: NO_CURSOR };
  }

  public async ListMediaFilesWithoutFolder(
    args: { cursor?: string; limit?: number; filterMediaClass?: string } = {},
  ): Promise<ApiResponse<FolderMediaFileListItem[], FolderCursor>> {
    const state = loadOrganization();
    const items = (await listLocalMedia())
      .filter((entry) => foldersOfFile(state, entry.token).length === 0)
      .filter((entry) => !args.filterMediaClass || entry.media_class === args.filterMediaClass
        || (args.filterMediaClass === "mesh" && entry.media_class === "dimensional"))
      .map((entry) => toFolderMediaItem(entry, entry.created_at));
    return { success: true, data: items, pagination: NO_CURSOR };
  }

  public async AddMediaFiles({ folderToken, mediaFileTokens }: { folderToken: string; mediaFileTokens: string[] }): Promise<ApiResponse<string[]>> {
    const files = await this.files();
    const accepted = updateOrganization((state) => {
      if (!state.folders[folderToken]) return [];
      const members = state.folderFiles[folderToken] ?? (state.folderFiles[folderToken] = []);
      const accepted = mediaFileTokens.filter((token) => files.has(token));
      for (const token of accepted) {
        if (!members.some((m) => m.token === token)) members.push({ token, added: nowIso() });
      }
      return accepted;
    });
    return { success: true, data: accepted };
  }

  public async MoveMediaFiles({ folderToken, sourceFolderToken, mediaFileTokens }: { folderToken: string; sourceFolderToken: string; mediaFileTokens: string[] }): Promise<ApiResponse<MoveMediaFilesResult>> {
    const files = await this.files();
    const result = updateOrganization((state): MoveMediaFilesResult => {
      const accepted = state.folders[folderToken] ? mediaFileTokens.filter((token) => files.has(token)) : [];
      const destination = state.folderFiles[folderToken] ?? (state.folderFiles[folderToken] = []);
      let added = 0;
      for (const token of accepted) {
        if (!destination.some((m) => m.token === token)) {
          destination.push({ token, added: nowIso() });
          added += 1;
        }
      }
      const source = state.folderFiles[sourceFolderToken] ?? [];
      const kept = source.filter((m) => !accepted.includes(m.token));
      const removed = source.length - kept.length;
      if (accepted.length > 0 && sourceFolderToken !== folderToken) state.folderFiles[sourceFolderToken] = kept;
      return {
        accepted_media_file_tokens: accepted,
        added_to_destination_count: added,
        removed_from_source_count: sourceFolderToken !== folderToken ? removed : 0,
      };
    });
    return { success: true, data: result };
  }

  public async RemoveMediaFiles({ folderToken, mediaFileTokens }: { folderToken: string; mediaFileTokens: string[] }): Promise<ApiResponse<number>> {
    const removed = updateOrganization((state) => {
      const members = state.folderFiles[folderToken] ?? [];
      const kept = members.filter((m) => !mediaFileTokens.includes(m.token));
      state.folderFiles[folderToken] = kept;
      return members.length - kept.length;
    });
    return { success: true, data: removed };
  }

  // ── Helpers ──

  private async files(): Promise<Map<string, LocalMediaEntry>> {
    try {
      return new Map((await listLocalMedia()).map((entry) => [entry.token, entry]));
    } catch {
      return new Map();
    }
  }

  private async info(folderToken: string): Promise<FolderInfo> {
    const state = loadOrganization();
    return toFolderInfo(state.folders[folderToken], state, await this.files());
  }

  private change(folderToken: string, apply: (record: FolderRecord) => void): ApiResponse<undefined> {
    const found = updateOrganization((state) => {
      const record = state.folders[folderToken];
      if (!record) return false;
      apply(record);
      record.updated_at = nowIso();
      return true;
    });
    return found ? { success: true } : { success: false, errorMessage: MISSING };
  }

  /** Whether `ancestor` is above `folder`, so a folder is never put inside its own descendant. */
  private isAncestor(folders: Record<string, FolderRecord>, ancestor: string, folder: string): boolean {
    let current: string | null = folders[folder]?.parent ?? null;
    const seen = new Set<string>();
    while (current && !seen.has(current)) {
      if (current === ancestor) return true;
      seen.add(current);
      current = folders[current]?.parent ?? null;
    }
    return false;
  }
}
