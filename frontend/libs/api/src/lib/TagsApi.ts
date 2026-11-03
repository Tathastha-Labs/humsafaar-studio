import type { ApiResponse } from "./ApiManager.js";
import type { MediaLinks, MediaFileCoverImageDetails } from "./models/Folder.js";
import { listLocalMedia, type LocalMediaEntry } from "./LocalLibrary.js";
import { loadOrganization, newToken, updateOrganization } from "./LocalOrganization.js";

// ─── Types ────────────────────────────────────────────────────────────────────

export interface TagDetails {
  tag_token: string;
  tag_value: string;
  tag_value_lowercase: string;
  use_count: number;
}

export interface TagMediaFileListItem {
  token: string;
  media_class: string;
  media_type: string;
  maybe_prompt_token?: string | null;
  maybe_batch_token?: string | null;
  media_links: MediaLinks;
  cover_image?: MediaFileCoverImageDetails;
  maybe_title?: string | null;
  maybe_original_filename?: string | null;
  maybe_frame_width?: number | null;
  maybe_frame_height?: number | null;
  maybe_duration_millis?: number | null;
  creator_set_visibility?: string;
  is_user_upload?: boolean;
  created_at: string;
  updated_at: string;
}

export interface MediaFileTagsEntry {
  media_file_token: string;
  tags: TagDetails[];
}

export interface TagsCursor {
  maybe_cursor?: string | null;
}

/** Paging for the list calls. Everything is on this computer, so a list comes back whole. */
export interface ListTagsQuery {
  cursor?: string;
  limit?: number;
}

const NO_CURSOR: TagsCursor = { maybe_cursor: null };

// ─── API ───────────────────────────────────────────────────────────────────────

/**
 * Tags, kept on this computer. A tag is a word; a file can have any number of them. Looking a tag
 * up ignores capital letters, so "Cat" and "cat" are the same tag.
 */
export class TagsApi {
  public async ListTags(_query: ListTagsQuery = {}): Promise<ApiResponse<TagDetails[], TagsCursor>> {
    const state = loadOrganization();
    const tags = Object.values(state.tags)
      .map((record) => this.details(state, record.token))
      .filter((tag): tag is TagDetails => tag !== null && tag.use_count > 0)
      .sort((a, b) => a.tag_value_lowercase.localeCompare(b.tag_value_lowercase));
    return { success: true, data: tags, pagination: NO_CURSOR };
  }

  public async RenameTag({ tagToken, newTagValue }: { tagToken: string; newTagValue: string }): Promise<ApiResponse<TagDetails>> {
    const value = newTagValue.trim();
    if (!value) return { success: false, errorMessage: "Give the tag a name." };
    const renamed = updateOrganization((state) => {
      const record = state.tags[tagToken];
      if (!record) return false;
      const existing = Object.values(state.tags).find((t) => t.token !== tagToken && t.value.toLowerCase() === value.toLowerCase());
      if (existing) {
        // Renaming onto an existing tag merges the two.
        for (const file of Object.keys(state.fileTags)) {
          const tokens = state.fileTags[file];
          if (tokens.includes(tagToken)) {
            state.fileTags[file] = Array.from(new Set(tokens.map((t) => (t === tagToken ? existing.token : t))));
          }
        }
        delete state.tags[tagToken];
      } else {
        record.value = value;
      }
      return true;
    });
    if (!renamed) return { success: false, errorMessage: "That tag does not exist." };
    const state = loadOrganization();
    const target = Object.values(state.tags).find((t) => t.value.toLowerCase() === value.toLowerCase());
    return { success: true, data: target ? (this.details(state, target.token) ?? undefined) : undefined };
  }

  public async DeleteTag({ tagToken }: { tagToken: string }): Promise<ApiResponse<{ removed_link_count: number }>> {
    const removed = updateOrganization((state) => {
      let count = 0;
      for (const file of Object.keys(state.fileTags)) {
        const kept = state.fileTags[file].filter((t) => t !== tagToken);
        count += state.fileTags[file].length - kept.length;
        state.fileTags[file] = kept;
      }
      delete state.tags[tagToken];
      return count;
    });
    return { success: true, data: { removed_link_count: removed } };
  }

  public async BulkAddTags({ mediaFileTokens, tags }: { mediaFileTokens: string[]; tags: string[] }): Promise<
    ApiResponse<{ accepted_media_file_tokens: string[]; tags: TagDetails[] }>
  > {
    const accepted = await this.knownTokens(mediaFileTokens);
    const added = updateOrganization((state) => {
      const tokens = tags.map((value) => this.tagFor(state, value)).filter((t): t is string => t !== null);
      for (const file of accepted) {
        const own = state.fileTags[file] ?? (state.fileTags[file] = []);
        for (const token of tokens) if (!own.includes(token)) own.push(token);
      }
      return tokens;
    });
    return { success: true, data: { accepted_media_file_tokens: accepted, tags: this.detailsFor(added) } };
  }

  public async BulkSetTags({ mediaFileTokens, tags }: { mediaFileTokens: string[]; tags: string[] }): Promise<
    ApiResponse<{ accepted_media_file_tokens: string[]; tags: TagDetails[]; removed_count: number }>
  > {
    const accepted = await this.knownTokens(mediaFileTokens);
    const { tokens, removed } = updateOrganization((state) => {
      const tokens = tags.map((value) => this.tagFor(state, value)).filter((t): t is string => t !== null);
      let removed = 0;
      for (const file of accepted) {
        const before = state.fileTags[file] ?? [];
        removed += before.filter((t) => !tokens.includes(t)).length;
        state.fileTags[file] = [...tokens];
      }
      return { tokens, removed };
    });
    return { success: true, data: { accepted_media_file_tokens: accepted, tags: this.detailsFor(tokens), removed_count: removed } };
  }

  public async ListMediaFileTags({ mediaFileToken }: { mediaFileToken: string }): Promise<ApiResponse<TagDetails[]>> {
    return { success: true, data: this.detailsFor(loadOrganization().fileTags[mediaFileToken] ?? []) };
  }

  public async AddMediaFileTags({ mediaFileToken, tags }: { mediaFileToken: string; tags: string[] }): Promise<ApiResponse<TagDetails[]>> {
    const added = await this.BulkAddTags({ mediaFileTokens: [mediaFileToken], tags });
    return { success: added.success, data: this.detailsFor(loadOrganization().fileTags[mediaFileToken] ?? []) };
  }

  public async SetMediaFileTags({ mediaFileToken, tags }: { mediaFileToken: string; tags: string[] }): Promise<ApiResponse<{ tags: TagDetails[]; removed_count: number }>> {
    const result = await this.BulkSetTags({ mediaFileTokens: [mediaFileToken], tags });
    return { success: result.success, data: { tags: result.data?.tags ?? [], removed_count: result.data?.removed_count ?? 0 } };
  }

  public async ClearMediaFileTags({ mediaFileToken }: { mediaFileToken: string }): Promise<ApiResponse<{ removed_count: number }>> {
    const removed = updateOrganization((state) => {
      const count = (state.fileTags[mediaFileToken] ?? []).length;
      state.fileTags[mediaFileToken] = [];
      return count;
    });
    return { success: true, data: { removed_count: removed } };
  }

  public async ListTaggedMediaFiles(_query: ListTagsQuery = {}): Promise<ApiResponse<TagMediaFileListItem[], TagsCursor>> {
    const state = loadOrganization();
    return this.items((entry) => (state.fileTags[entry.token] ?? []).length > 0);
  }

  public async ListUntaggedMediaFiles(_query: ListTagsQuery = {}): Promise<ApiResponse<TagMediaFileListItem[], TagsCursor>> {
    const state = loadOrganization();
    return this.items((entry) => (state.fileTags[entry.token] ?? []).length === 0);
  }

  public async ListMediaFilesWithTag({ tagToken }: { tagToken: string; cursor?: string; limit?: number }): Promise<ApiResponse<TagMediaFileListItem[], TagsCursor>> {
    const state = loadOrganization();
    return this.items((entry) => (state.fileTags[entry.token] ?? []).includes(tagToken));
  }

  public async BulkListMediaFileTags({ mediaFileTokens }: { mediaFileTokens: string[] }): Promise<ApiResponse<MediaFileTagsEntry[]>> {
    const state = loadOrganization();
    return {
      success: true,
      data: mediaFileTokens.map((token) => ({
        media_file_token: token,
        tags: this.detailsFor(state.fileTags[token] ?? []),
      })),
    };
  }

  // ── Helpers ──

  /** The token of the tag with this word, made if there is none. Blank words are ignored. */
  private tagFor(state: ReturnType<typeof loadOrganization>, value: string): string | null {
    const word = value.trim();
    if (!word) return null;
    const existing = Object.values(state.tags).find((t) => t.value.toLowerCase() === word.toLowerCase());
    if (existing) return existing.token;
    const token = newToken("tag");
    state.tags[token] = { token, value: word };
    return token;
  }

  private details(state: ReturnType<typeof loadOrganization>, tagToken: string): TagDetails | null {
    const record = state.tags[tagToken];
    if (!record) return null;
    const uses = Object.values(state.fileTags).filter((tokens) => tokens.includes(tagToken)).length;
    return { tag_token: tagToken, tag_value: record.value, tag_value_lowercase: record.value.toLowerCase(), use_count: uses };
  }

  private detailsFor(tagTokens: string[]): TagDetails[] {
    const state = loadOrganization();
    return tagTokens.map((token) => this.details(state, token)).filter((t): t is TagDetails => t !== null);
  }

  private async knownTokens(tokens: string[]): Promise<string[]> {
    const known = new Set((await this.entries()).map((entry) => entry.token));
    return tokens.filter((token) => known.has(token));
  }

  private async items(include: (entry: LocalMediaEntry) => boolean): Promise<ApiResponse<TagMediaFileListItem[], TagsCursor>> {
    const items = (await this.entries()).filter(include).map((entry): TagMediaFileListItem => ({
      token: entry.token,
      media_class: entry.media_class,
      media_type: entry.media_class === "dimensional" ? "glb" : entry.media_class,
      media_links: { cdn_url: entry.url, maybe_thumbnail_template: null },
      creator_set_visibility: "private",
      is_user_upload: entry.origin_category === "upload",
      created_at: entry.created_at,
      updated_at: entry.created_at,
    }));
    return { success: true, data: items, pagination: NO_CURSOR };
  }

  private async entries(): Promise<LocalMediaEntry[]> {
    try {
      return await listLocalMedia();
    } catch {
      return [];
    }
  }
}
