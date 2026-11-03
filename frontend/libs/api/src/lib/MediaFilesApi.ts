import { invoke } from "@tauri-apps/api/core";
import { MediaFile } from "./models/MediaFile.js";
import { Pagination, PaginationInfinite } from "./models/Pagination.js";
import { ApiResponse } from "./ApiManager.js";
import { Visibility } from "./enums/Visibility.js";
import {
  FilterEngineCategories,
  FilterMediaClasses,
  FilterMediaType,
} from "./enums/QueryFilters.js";
import { listScenes } from "./LocalScenes.js";
import {
  listLocalMedia,
  matchesClassFilter,
  toMediaFile,
  type LocalMediaEntry,
} from "./LocalLibrary.js";

// The media library, kept on this computer. The method names and response shapes are the ones the
// pages were written against, so they work unchanged; things only a hosted account had (featured
// items, titles, cover images, visibility) answer with an empty or "not available" result.

const DEFAULT_PAGE_SIZE = 40;

interface ListMediaQuery {
  sort_ascending?: boolean;
  page_size?: number;
  cursor?: string;
  cursor_is_reversed?: boolean;
  filter_media_classes?: FilterMediaClasses[];
  filter_media_type?: FilterMediaType[];
  filter_engine_categories?: FilterEngineCategories[];
}

interface ListUserMediaQuery {
  username?: string;
  sort_ascending?: boolean;
  page_size?: number;
  page_index?: number;
  filter_media_classes?: FilterMediaClasses[];
  filter_media_type?: FilterMediaType[];
  filter_engine_categories?: FilterEngineCategories[];
  user_uploads_only?: boolean;
  include_user_uploads?: boolean;
}

interface SearchMediaQuery {
  search_term: string;
  filter_media_classes?: FilterMediaClasses[];
  filter_media_type?: FilterMediaType[];
  filter_engine_categories?: FilterEngineCategories[];
}

// Pagination-only query for the by-class session lists (mesh / splat).
interface ListSessionMediaByClassQuery {
  sort_ascending?: boolean;
  page_size?: number;
  cursor?: string;
  cursor_is_reversed?: boolean;
}

/** Which stored kinds an engine category (as the scene editor asks for them) can mean. */
function kindsForCategories(categories: FilterEngineCategories[] | undefined): Set<LocalMediaEntry["media_class"]> | null {
  if (!categories || categories.length === 0) return null;
  const kinds = new Set<LocalMediaEntry["media_class"]>();
  for (const category of categories) {
    switch (category) {
      case FilterEngineCategories.IMAGE_PLANE: kinds.add("image"); break;
      case FilterEngineCategories.VIDEO_PLANE: kinds.add("video"); break;
      case FilterEngineCategories.AUDIO: kinds.add("audio"); break;
      case FilterEngineCategories.SPLAT:
      case FilterEngineCategories.SCENE: break;
      // Characters, objects, sets and the rest are all 3D models here; they are not told apart.
      default: kinds.add("dimensional");
    }
  }
  return kinds;
}

const NOT_AVAILABLE = "That is not available because everything is stored on this computer.";

export class MediaFilesApi {
  public async DeleteMediaFileByToken({
    mediaFileToken,
  }: {
    mediaFileToken: string;
    asMod?: boolean;
    setDelete?: boolean;
  }): Promise<ApiResponse<MediaFile>> {
    try {
      await invoke("media_file_delete_command", {
        request: { media_file_token: mediaFileToken },
      });
      return { success: true };
    } catch (err) {
      return { success: false, errorMessage: String(err) };
    }
  }

  public async ListMediaFilesByTokens({
    mediaTokens,
  }: {
    mediaTokens: string[];
  }): Promise<ApiResponse<MediaFile[]>> {
    if (mediaTokens.length === 0) {
      return { success: true, data: [] };
    }
    const wanted = new Set(mediaTokens);
    const files = (await this.entries()).filter((entry) => wanted.has(entry.token));
    return { success: true, data: files.map(toMediaFile) };
  }

  /** Batches are not tracked on this computer; every result is its own item. */
  public async GetMediaFilesByBatchToken(_args: {
    batchToken: string;
  }): Promise<ApiResponse<MediaFile[]>> {
    return { success: true, data: [] };
  }

  public async GetMediaFileByToken({
    mediaFileToken,
  }: {
    mediaFileToken: string;
  }): Promise<ApiResponse<MediaFile>> {
    const entry = (await this.entries()).find((item) => item.token === mediaFileToken);
    if (!entry) {
      return { success: false, errorMessage: "That file is not on this computer." };
    }
    return { success: true, data: toMediaFile(entry) };
  }

  public async ListMediaFiles(
    query: ListMediaQuery,
  ): Promise<ApiResponse<MediaFile[], PaginationInfinite>> {
    const files = (await this.entries())
      .filter((entry) => matchesClassFilter(entry, query.filter_media_classes))
      .slice(0, query.page_size ?? DEFAULT_PAGE_SIZE);
    return {
      success: true,
      data: files.map(toMediaFile),
      pagination: { cursor_is_reversed: false },
    };
  }

  /** There is no featured gallery. */
  public async ListFeaturedMediaFiles(
    _query: ListMediaQuery,
  ): Promise<ApiResponse<MediaFile[], PaginationInfinite>> {
    return { success: true, data: [], pagination: { cursor_is_reversed: false } };
  }

  public async ListSessionMeshMediaFiles(
    query: ListSessionMediaByClassQuery = {},
  ): Promise<ApiResponse<MediaFile[], PaginationInfinite>> {
    const files = (await this.entries())
      .filter((entry) => entry.media_class === "dimensional")
      .slice(0, query.page_size ?? DEFAULT_PAGE_SIZE);
    return {
      success: true,
      data: files.map(toMediaFile),
      pagination: { cursor_is_reversed: false },
    };
  }

  /** 3D worlds are not made here. */
  public async ListSessionSplatMediaFiles(
    _query: ListSessionMediaByClassQuery = {},
  ): Promise<ApiResponse<MediaFile[], PaginationInfinite>> {
    return { success: true, data: [], pagination: { cursor_is_reversed: false } };
  }

  public async ListUserMediaFiles(
    query: ListUserMediaQuery,
  ): Promise<ApiResponse<MediaFile[], Pagination>> {
    const pageSize = query.page_size ?? DEFAULT_PAGE_SIZE;
    const pageIndex = query.page_index ?? 0;

    if (query.filter_media_type?.includes(FilterMediaType.SCENE_JSON)
      || (query.filter_engine_categories?.length === 1 && query.filter_engine_categories[0] === FilterEngineCategories.SCENE)) {
      const scenes = await listScenes();
      const page = scenes.slice(pageIndex * pageSize, (pageIndex + 1) * pageSize);
      return {
        success: true,
        data: page,
        pagination: { current: pageIndex, total_page_count: Math.max(1, Math.ceil(scenes.length / pageSize)) },
      };
    }
    const kinds = kindsForCategories(query.filter_engine_categories);
    const wantsModels = (query.filter_media_type ?? []).length > 0;

    const matching = (await this.entries()).filter((entry) => {
      if (!matchesClassFilter(entry, query.filter_media_classes)) return false;
      if (kinds && !kinds.has(entry.media_class)) return false;
      if (wantsModels && entry.media_class !== "dimensional") return false;
      if (query.user_uploads_only) return entry.origin_category === "upload";
      if (query.include_user_uploads === false) return entry.origin_category !== "upload";
      return true;
    });
    if (query.sort_ascending) matching.reverse();

    const page = matching.slice(pageIndex * pageSize, (pageIndex + 1) * pageSize);
    return {
      success: true,
      data: page.map(toMediaFile),
      pagination: {
        current: pageIndex,
        total_page_count: Math.max(1, Math.ceil(matching.length / pageSize)),
      },
    };
  }

  /** There is no featured gallery to search. */
  public async SearchFeaturedMediaFiles(
    _query: SearchMediaQuery,
  ): Promise<ApiResponse<MediaFile[], Pagination>> {
    return { success: true, data: [], pagination: { current: 0, total_page_count: 1 } };
  }

  /** Files have no titles here, so a search matches the model that made them. */
  public async SearchUserMediaFiles(
    query: SearchMediaQuery,
  ): Promise<ApiResponse<MediaFile[], Pagination>> {
    const term = query.search_term.trim().toLowerCase();
    const files = (await this.entries())
      .filter((entry) => matchesClassFilter(entry, query.filter_media_classes))
      .filter((entry) => !term || (entry.maybe_model_type ?? "").toLowerCase().includes(term));
    return {
      success: true,
      data: files.map(toMediaFile),
      pagination: { current: 0, total_page_count: 1 },
    };
  }

  public async RenameMediaFileByToken(_args: {
    mediaToken: string;
    name: string;
  }): Promise<ApiResponse<undefined>> {
    return { success: false, errorMessage: NOT_AVAILABLE };
  }

  public async UpdateCoverImage(_args: {
    mediaFileToken: string;
    imageToken: string;
  }): Promise<ApiResponse<undefined>> {
    return { success: false, errorMessage: NOT_AVAILABLE };
  }

  /** Everything here is private already. */
  public async UpdateVisibility(_args: {
    mediaFileToken: string;
    visibility: Visibility;
  }): Promise<ApiResponse<undefined>> {
    return { success: true };
  }

  private async entries(): Promise<LocalMediaEntry[]> {
    try {
      return await listLocalMedia();
    } catch {
      return [];
    }
  }
}
