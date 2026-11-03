import type { ApiResponse } from "./ApiManager.js";
import { saveLocalFile } from "./LocalLibrary.js";

export class EngineApi {
  /** Save a picture of the scene. Returns the new file's token. */
  public async uploadSceneSnapshot({
    screenshot,
  }: {
    screenshot: File;
    sceneMediaToken?: string;
  }): Promise<ApiResponse<string>> {
    try {
      return { success: true, data: await saveLocalFile(screenshot) };
    } catch (error) {
      return { success: false, errorMessage: error instanceof Error ? error.message : "Failed to save the snapshot." };
    }
  }
}
