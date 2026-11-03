import { beforeEach, expect, it, vi } from "vitest";
import { sceneOutputAdapter } from "./sceneOutputAdapter";

const mocks = vi.hoisted(() => ({
  save: vi.fn(),
  getFile: vi.fn(),
  image: { value: null as any },
  mediaId: { value: null as string | null },
  visible: { value: false },
  previous: { value: null },
  next: { value: null },
}));

vi.mock("@storyteller/api", () => ({
  saveLocalFile: mocks.save,
  MediaFilesApi: class {
    GetMediaFileByToken = mocks.getFile;
  },
}));
vi.mock("@storyteller/ui-gallery-modal", () => ({
  galleryModalLightboxImage: mocks.image,
  galleryModalLightboxMediaId: mocks.mediaId,
  galleryModalLightboxVisible: mocks.visible,
  galleryModalLightboxNavPrev: mocks.previous,
  galleryModalLightboxNavNext: mocks.next,
}));

beforeEach(() => {
  mocks.save.mockReset();
  mocks.getFile.mockReset();
  mocks.image.value = null;
  mocks.mediaId.value = null;
  mocks.visible.value = false;
});

it.each([
  { kind: "video" as const, fileName: "scene-recording.mp4", mime: "video/mp4" },
  { kind: "image" as const, fileName: "scene-capture.png", mime: "image/png" },
])("saves a $kind in the media library", async ({ kind, fileName, mime }) => {
  mocks.save.mockResolvedValue("mf_local_gscene");
  const blob = new Blob(["rendered scene"], { type: mime });
  const token = await sceneOutputAdapter.uploadMedia({ kind, blob, fileName });
  expect(token).toBe("mf_local_gscene");
  const saved = mocks.save.mock.calls[0][0] as File;
  expect(saved.name).toBe(fileName);
  expect(saved.type).toBe(mime);
  expect(saved.size).toBe(blob.size);
});

it("says so when the recording could not be saved", async () => {
  mocks.save.mockRejectedValue(new Error("disk full"));
  await expect(sceneOutputAdapter.uploadMedia({
    kind: "video", blob: new Blob(["video"]), fileName: "recording.mp4",
  })).rejects.toThrow("disk full");
  expect(mocks.visible.value).toBe(false);
});

it("opens a saved video in the lightbox using its local URL", async () => {
  mocks.getFile.mockResolvedValue({
    success: true,
    data: {
      token: "mf_local_gscene",
      maybe_title: "My recording",
      created_at: "2026-09-11T12:00:00Z",
      media_links: { cdn_url: "asset://localhost/recording.mp4", thumbnail_template: "" },
    },
  });
  await sceneOutputAdapter.openMediaLightbox("mf_local_gscene", "video");
  expect(mocks.getFile).toHaveBeenCalledWith({ mediaFileToken: "mf_local_gscene" });
  expect(mocks.image.value).toMatchObject({
    id: "mf_local_gscene", label: "My recording", mediaClass: "video",
    fullImage: "asset://localhost/recording.mp4",
  });
  expect(mocks.mediaId.value).toBe("mf_local_gscene");
  expect(mocks.visible.value).toBe(true);
});

it("reports a failed preview lookup so the recording can be retained", async () => {
  mocks.getFile.mockResolvedValue({ success: false });
  await expect(sceneOutputAdapter.openMediaLightbox("mf_local_gscene", "video"))
    .rejects.toThrow("Saved, but the preview could not be loaded");
  expect(mocks.visible.value).toBe(false);
});
