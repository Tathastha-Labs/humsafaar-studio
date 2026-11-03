import type { ApiResponse } from "./ApiManager.js";
import type { FilterEngineCategories } from "./enums/QueryFilters.js";
import type { Visibility } from "./enums/Visibility.js";
import type { EIntermediateFile } from "./enums/EIntermediateFile.js";
import { UploaderStates, UploaderState } from "@storyteller/common";
import { saveLocalFile } from "./LocalLibrary.js";

// Uploading means saving into the media library on this computer. The argument shapes are the ones
// the pages were written against, so options that only mattered to a hosted account (visibility,
// titles, whether a file is an intermediate) are accepted and not needed.

interface UploadArgs {
  blob: Blob;
  fileName: string;
  uuid?: string;
  maybe_title?: string;
  maybe_visibility?: Visibility;
  is_intermediate_system_file?: EIntermediateFile;
}

export class MediaUploadApi {
  public async UploadAudio(args: UploadArgs): Promise<ApiResponse<string>> {
    return this.save(args.blob, args.fileName);
  }

  public async UploadImage(args: UploadArgs): Promise<ApiResponse<string>> {
    return this.save(args.blob, args.fileName);
  }

  public async UploadNewVideo(
    args: UploadArgs & { maybe_style_name?: string; maybe_scene_source_media_file_token?: string },
  ): Promise<ApiResponse<string>> {
    return this.save(args.blob, args.fileName);
  }

  /** A model, picture plane or other file for a scene. The category is not recorded. */
  public async UploadNewEngineAsset(args: {
    file: File;
    fileName: string;
    uuid?: string;
    engine_category: FilterEngineCategories;
    maybe_animation_type?: string;
    maybe_duration_millis?: number;
    maybe_title?: string;
    maybe_visibility?: Visibility;
  }): Promise<ApiResponse<string>> {
    return this.save(args.file, args.fileName);
  }

  private async save(blob: Blob, fileName: string): Promise<ApiResponse<string>> {
    try {
      const file = new File([blob], fileName, { type: blob.type });
      return { success: true, data: await saveLocalFile(file) };
    } catch (error) {
      return { success: false, errorMessage: error instanceof Error ? error.message : "Could not save the file." };
    }
  }
}

export type UploadMediaFn = (args: {
  title: string;
  assetFile: File;
  progressCallback: (newState: UploaderState) => void;
}) => Promise<void>;

/** Save a file for a page that wants its token, reporting progress the way the pages expect. */
const saveForPage = (failureMessage: string): UploadMediaFn =>
  async ({ assetFile, progressCallback }) => {
    progressCallback({ status: UploaderStates.uploadingImage });
    try {
      progressCallback({ status: UploaderStates.success, data: await saveLocalFile(assetFile) });
    } catch (error) {
      progressCallback({
        status: UploaderStates.imageCreateError,
        errorMessage: error instanceof Error ? error.message : failureMessage,
      });
    }
  };

export const UploadImageMedia: UploadMediaFn = saveForPage("Could not save the image.");
export const UploadVideoMedia: UploadMediaFn = saveForPage("Could not save the video.");
export const UploadAudioMedia: UploadMediaFn = saveForPage("Could not save the audio.");
