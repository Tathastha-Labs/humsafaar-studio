import type { ApiResponse } from "./ApiManager.js";
import type { Prompts } from "./models/Prompts.js";
import { saveLocalFile } from "./LocalLibrary.js";

/**
 * Prompts and scene snapshots on this computer. A prompt is remembered by the task that used it
 * (see the app's task metadata), so there are no prompt records to look up by token, and the
 * lookups answer that none exist. Snapshots are saved to the media library.
 */
export class PromptsApi {
  public async GetPromptsByToken(_args: { token: string }): Promise<ApiResponse<Prompts>> {
    return { success: false, errorMessage: "Prompts are not kept as separate records." };
  }

  public async BatchGetPrompts(_args: { tokens: string[] }): Promise<ApiResponse<Prompts[]>> {
    return { success: true, data: [] };
  }

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
      return { success: false, errorMessage: error instanceof Error ? error.message : "Failed to generate snapshot." };
    }
  }
}
