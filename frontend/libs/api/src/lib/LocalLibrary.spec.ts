import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { MediaFilesApi } from "./MediaFilesApi";
import { FoldersApi } from "./FoldersApi";
import { TagsApi } from "./TagsApi";
import { resetOrganizationCache } from "./LocalOrganization";
import { FilterMediaClasses } from "./enums/QueryFilters";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const file = (token: string, media_class: string, origin_category: string, created_at: string) => ({
  token, url: `asset://localhost/${token}`, media_class, origin_category, created_at, size_bytes: 1, maybe_model_type: null,
});

const FILES = [
  file("mf_local_g1", "image", "inference", "2026-01-03T00:00:00Z"),
  file("mf_local_u2", "image", "upload", "2026-01-02T00:00:00Z"),
  file("mf_local_g3", "video", "inference", "2026-01-01T00:00:00Z"),
  file("mf_local_g4", "dimensional", "inference", "2026-01-01T00:00:00Z"),
];

beforeEach(() => {
  const store = new Map<string, string>();
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => void store.set(key, value),
  });
  vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
  resetOrganizationCache();
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockResolvedValue({ payload: { files: FILES } });
});

describe("the library", () => {
  it("lists newest first and filters by kind", async () => {
    const images = await new MediaFilesApi().ListUserMediaFiles({ filter_media_classes: [FilterMediaClasses.IMAGE] });
    expect(images.data?.map((f) => f.token)).toEqual(["mf_local_g1", "mf_local_u2"]);
  });

  it("can leave uploads out, or show only uploads", async () => {
    const api = new MediaFilesApi();
    const results = await api.ListUserMediaFiles({ include_user_uploads: false });
    expect(results.data?.every((f) => f.origin_category === "inference")).toBe(true);
    const uploads = await api.ListUserMediaFiles({ user_uploads_only: true });
    expect(uploads.data?.map((f) => f.token)).toEqual(["mf_local_u2"]);
  });

  it("a 3D file matches the old and new 3D filters", async () => {
    const api = new MediaFilesApi();
    for (const kind of [FilterMediaClasses.DIMENSIONAL, FilterMediaClasses.MESH]) {
      const result = await api.ListUserMediaFiles({ filter_media_classes: [kind] });
      expect(result.data?.map((f) => f.token)).toEqual(["mf_local_g4"]);
    }
  });

  it("pages the results", async () => {
    const page = await new MediaFilesApi().ListUserMediaFiles({ page_size: 3, page_index: 1 });
    expect(page.data?.map((f) => f.token)).toEqual(["mf_local_g4"]);
    expect(page.pagination).toEqual({ current: 1, total_page_count: 2 });
  });

  it("finds one file by token and says so when it is missing", async () => {
    const api = new MediaFilesApi();
    expect((await api.GetMediaFileByToken({ mediaFileToken: "mf_local_g3" })).data?.media_links.cdn_url).toBe("asset://localhost/mf_local_g3");
    expect((await api.GetMediaFileByToken({ mediaFileToken: "mf_local_nope" })).success).toBe(false);
  });

  it("is empty rather than failing when the list cannot be read", async () => {
    vi.mocked(invoke).mockRejectedValue(new Error("boom"));
    expect((await new MediaFilesApi().ListUserMediaFiles({})).data).toEqual([]);
  });
});

describe("folders", () => {
  it("creates, fills and empties a folder", async () => {
    const api = new FoldersApi();
    const folder = (await api.CreateFolder({ name: " Trip " })).data!;
    expect(folder.name).toBe("Trip");

    await api.AddMediaFiles({ folderToken: folder.token, mediaFileTokens: ["mf_local_g1", "mf_local_nope"] });
    const inside = await api.ListFolderMediaFiles({ folderToken: folder.token });
    expect(inside.data?.map((f) => f.token)).toEqual(["mf_local_g1"]);

    const loose = await api.ListMediaFilesWithoutFolder();
    expect(loose.data?.map((f) => f.token)).not.toContain("mf_local_g1");

    await api.RemoveMediaFiles({ folderToken: folder.token, mediaFileTokens: ["mf_local_g1"] });
    expect((await api.ListFolderMediaFiles({ folderToken: folder.token })).data).toEqual([]);
  });

  it("moves files between folders", async () => {
    const api = new FoldersApi();
    const a = (await api.CreateFolder({ name: "A" })).data!;
    const b = (await api.CreateFolder({ name: "B" })).data!;
    await api.AddMediaFiles({ folderToken: a.token, mediaFileTokens: ["mf_local_g1"] });
    const moved = await api.MoveMediaFiles({ folderToken: b.token, sourceFolderToken: a.token, mediaFileTokens: ["mf_local_g1"] });
    expect(moved.data).toEqual({ accepted_media_file_tokens: ["mf_local_g1"], added_to_destination_count: 1, removed_from_source_count: 1 });
    expect((await api.ListFolderMediaFiles({ folderToken: a.token })).data).toEqual([]);
  });

  it("keeps folder changes after a restart", async () => {
    const api = new FoldersApi();
    const folder = (await api.CreateFolder({ name: "Keep" })).data!;
    await api.SetStar({ folderToken: folder.token, hasStar: true });
    resetOrganizationCache();
    const again = (await new FoldersApi().GetFolder({ folderToken: folder.token })).data!;
    expect(again.has_star).toBe(true);
  });

  it("never puts a folder inside its own descendant", async () => {
    const api = new FoldersApi();
    const parent = (await api.CreateFolder({ name: "P" })).data!;
    const child = (await api.CreateFolder({ name: "C", maybe_parent_folder_token: parent.token })).data!;
    const result = await api.AddSubfolders({ folderToken: child.token, subfolderTokens: [parent.token] });
    expect(result.data).toEqual([]);
  });

  it("an orphaned child is flagged when its parent is deleted", async () => {
    const api = new FoldersApi();
    const parent = (await api.CreateFolder({ name: "P" })).data!;
    const child = (await api.CreateFolder({ name: "C", maybe_parent_folder_token: parent.token })).data!;
    await api.DeleteFolder({ folderToken: parent.token });
    expect((await api.GetFolder({ folderToken: child.token })).data?.is_orphaned).toBe(true);
  });

  it("refuses a blank name", async () => {
    expect((await new FoldersApi().CreateFolder({ name: "  " })).success).toBe(false);
  });
});

describe("tags", () => {
  it("tags are the same word whatever the capitals, and count their uses", async () => {
    const api = new TagsApi();
    await api.AddMediaFileTags({ mediaFileToken: "mf_local_g1", tags: ["Cat"] });
    await api.AddMediaFileTags({ mediaFileToken: "mf_local_u2", tags: ["cat", "  ", "dog"] });
    const tags = (await api.ListTags()).data!;
    expect(tags.map((t) => [t.tag_value_lowercase, t.use_count])).toEqual([["cat", 2], ["dog", 1]]);
  });

  it("finds the files with a tag, and the untagged ones", async () => {
    const api = new TagsApi();
    await api.SetMediaFileTags({ mediaFileToken: "mf_local_g1", tags: ["sky"] });
    const tag = (await api.ListTags()).data![0];
    expect((await api.ListMediaFilesWithTag({ tagToken: tag.tag_token })).data?.map((f) => f.token)).toEqual(["mf_local_g1"]);
    expect((await api.ListUntaggedMediaFiles()).data?.map((f) => f.token)).not.toContain("mf_local_g1");
  });

  it("setting tags replaces the old ones, and clearing removes them", async () => {
    const api = new TagsApi();
    await api.SetMediaFileTags({ mediaFileToken: "mf_local_g1", tags: ["a", "b"] });
    const set = await api.SetMediaFileTags({ mediaFileToken: "mf_local_g1", tags: ["c"] });
    expect(set.data?.removed_count).toBe(2);
    await api.ClearMediaFileTags({ mediaFileToken: "mf_local_g1" });
    expect((await api.ListMediaFileTags({ mediaFileToken: "mf_local_g1" })).data).toEqual([]);
  });

  it("renaming onto an existing tag merges them", async () => {
    const api = new TagsApi();
    await api.AddMediaFileTags({ mediaFileToken: "mf_local_g1", tags: ["one"] });
    await api.AddMediaFileTags({ mediaFileToken: "mf_local_u2", tags: ["two"] });
    const [one, two] = (await api.ListTags()).data!;
    await api.RenameTag({ tagToken: one.tag_token, newTagValue: "TWO" });
    const left = (await api.ListTags()).data!;
    expect(left).toHaveLength(1);
    expect(left[0].use_count).toBe(2);
    expect(two.tag_token).toBe(left[0].tag_token);
  });

  it("ignores files that are not in the library", async () => {
    const result = await new TagsApi().BulkAddTags({ mediaFileTokens: ["mf_local_nope"], tags: ["x"] });
    expect(result.data?.accepted_media_file_tokens).toEqual([]);
  });
});
