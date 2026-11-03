import { MediaFilesApi, saveLocalFile } from "@storyteller/api";
import type { PageSceneAdapter } from "@storyteller/ui-pagescene";
import {
  type GalleryItem,
  galleryModalLightboxImage,
  galleryModalLightboxMediaId,
  galleryModalLightboxNavNext,
  galleryModalLightboxNavPrev,
  galleryModalLightboxVisible,
} from "@storyteller/ui-gallery-modal";

export const sceneOutputAdapter: Pick<
  PageSceneAdapter,
  "uploadMedia" | "openMediaLightbox"
> = {
  /** Captures and recordings are library outputs, so they are saved in the media library. */
  async uploadMedia({ blob, fileName }) {
    try {
      return await saveLocalFile(new File([blob], fileName, { type: blob.type }));
    } catch (error) {
      throw new Error(error instanceof Error ? error.message : "Could not save the recording or capture.");
    }
  },

  async openMediaLightbox(token, kind) {
    const response = await new MediaFilesApi().GetMediaFileByToken({
      mediaFileToken: token,
    });
    const file = response.data;
    if (!response.success || !file?.media_links?.cdn_url) {
      throw new Error("Saved, but the preview could not be loaded. Retry to open it.");
    }
    const item: GalleryItem = {
      id: token,
      label: file.maybe_title || (kind === "video" ? "Scene recording" : "Scene capture"),
      thumbnail: null,
      thumbnailUrlTemplate: file.media_links.thumbnail_template,
      fullImage: file.media_links.cdn_url,
      createdAt: file.created_at,
      mediaClass: kind,
      isUpload: true,
    };
    galleryModalLightboxNavPrev.value = null;
    galleryModalLightboxNavNext.value = null;
    galleryModalLightboxMediaId.value = token;
    galleryModalLightboxImage.value = item;
    galleryModalLightboxVisible.value = true;
  },
};
