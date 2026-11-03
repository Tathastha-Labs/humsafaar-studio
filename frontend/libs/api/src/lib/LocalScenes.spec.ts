import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { loadScene, saveScene, renameScene, listScenes } from "./LocalScenes";
import { MediaFilesApi } from "./MediaFilesApi";
import { FilterEngineCategories, FilterMediaType } from "./enums/QueryFilters";
import { resetOrganizationCache } from "./LocalOrganization";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// A stand-in for the media folder: the same commands, backed by a map.
const files = new Map<string, { extension: string; text: string; media_class?: string }>();
let counter = 0;

const decode = (base64: string) => Buffer.from(base64, "base64").toString("utf8");

beforeEach(() => {
  files.clear();
  counter = 0;
  const store = new Map<string, string>();
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => void store.set(key, value),
  });
  vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
  resetOrganizationCache();

  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation(async (command: string, args?: any) => {
    const request = args?.request;
    switch (command) {
      case "local_media_import_command": {
        const token = `mf_local_u${++counter}`;
        const extension = request.extension as string;
        files.set(token, { extension, text: decode(request.base64_data), media_class: extension === "png" ? "image" : undefined });
        return { payload: { media_file_token: token, url: `asset://localhost/${token}` } };
      }
      case "local_media_replace_command": {
        const file = files.get(request.media_file_token);
        if (!file) throw new Error("missing");
        file.text = decode(request.base64_data);
        return { payload: {} };
      }
      case "local_media_read_text_command": {
        const file = files.get(request.media_file_token);
        if (!file) throw new Error("missing");
        return { payload: { text: file.text } };
      }
      case "local_media_list_command":
        return {
          payload: {
            files: [...files.entries()]
              .filter(([, f]) => f.media_class)
              .map(([token, f]) => ({
                token, url: `asset://localhost/${token}`, media_class: f.media_class,
                origin_category: "upload", created_at: "2026-01-01T00:00:00Z", size_bytes: 1, maybe_model_type: null,
              })),
          },
        };
      default:
        throw new Error(`unexpected command ${command}`);
    }
  });
});

describe("scenes", () => {
  it("saves a new scene and loads it back with its title", async () => {
    const token = await saveScene({ json: '{"objects":[1,2]}', title: "My Scene" });
    expect(token).toMatch(/^mf_local_u/);
    const loaded = await loadScene(token);
    expect(loaded).toEqual({ scene: { objects: [1, 2] }, title: "My Scene" });
  });

  it("saving again with the token keeps the same scene and changes it", async () => {
    const token = await saveScene({ json: '{"v":1}', title: "A" });
    const again = await saveScene({ json: '{"v":2}', title: "A2", token });
    expect(again).toBe(token);
    expect((await loadScene(token)).scene).toEqual({ v: 2 });
    expect((await listScenes()).map((s) => s.maybe_title)).toEqual(["A2"]);
  });

  it("an empty title becomes Untitled Scene", async () => {
    const token = await saveScene({ json: "{}", title: "   " });
    expect((await loadScene(token)).title).toBe("Untitled Scene");
  });

  it("renames a scene, and refuses a blank name", async () => {
    const token = await saveScene({ json: "{}", title: "Old" });
    expect(renameScene(token, " New ")).toBe(true);
    expect((await loadScene(token)).title).toBe("New");
    expect(renameScene(token, "  ")).toBe(false);
    expect(renameScene("mf_local_unope", "x")).toBe(false);
  });

  it("a cover picture is saved and shown for the scene", async () => {
    const token = await saveScene({ json: "{}", title: "Pic", cover: new Blob(["png"], { type: "image/png" }) });
    const [scene] = await listScenes();
    expect(scene.token).toBe(token);
    expect(scene.media_links.cdn_url).toMatch(/^asset:\/\/localhost\/mf_local_u/);
  });

  it("scenes do not appear among pictures, and loading a missing scene fails", async () => {
    await saveScene({ json: "{}", title: "S" });
    const images = await new MediaFilesApi().ListUserMediaFiles({});
    expect(images.data).toEqual([]);
    await expect(loadScene("mf_local_unope")).rejects.toThrow();
  });

  it("returns an empty token when saving fails", async () => {
    vi.mocked(invoke).mockRejectedValue(new Error("disk full"));
    expect(await saveScene({ json: "{}", title: "x" })).toBe("");
  });
});

describe("what the scene editor asks the library for", () => {
  it("scenes come back for the scene-file filter, newest first", async () => {
    await saveScene({ json: "{}", title: "First" });
    await saveScene({ json: "{}", title: "Second" });
    const result = await new MediaFilesApi().ListUserMediaFiles({ filter_media_type: [FilterMediaType.SCENE_JSON] });
    expect(result.data?.map((s) => s.maybe_title)).toEqual(["Second", "First"]);
  });

  it("image planes ask for pictures and 3D categories ask only for models", async () => {
    files.set("mf_local_ua", { extension: "png", text: "", media_class: "image" });
    files.set("mf_local_ub", { extension: "glb", text: "", media_class: "dimensional" });
    const api = new MediaFilesApi();
    const planes = await api.ListUserMediaFiles({ filter_engine_categories: [FilterEngineCategories.IMAGE_PLANE] });
    expect(planes.data?.map((f) => f.token)).toEqual(["mf_local_ua"]);
    const objects = await api.ListUserMediaFiles({ filter_engine_categories: [FilterEngineCategories.OBJECT] });
    expect(objects.data?.map((f) => f.token)).toEqual(["mf_local_ub"]);
    const models = await api.ListUserMediaFiles({ filter_media_type: [FilterMediaType.GLB] });
    expect(models.data?.map((f) => f.token)).toEqual(["mf_local_ub"]);
  });

  it("a file's bucket path is its full URL, so the empty CDN origin joins up", async () => {
    files.set("mf_local_ua", { extension: "png", text: "", media_class: "image" });
    const file = (await new MediaFilesApi().GetMediaFileByToken({ mediaFileToken: "mf_local_ua" })).data!;
    expect(file.public_bucket_path).toBe("asset://localhost/mf_local_ua");
  });
});
