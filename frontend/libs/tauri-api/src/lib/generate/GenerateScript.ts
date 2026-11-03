import { invoke } from "@tauri-apps/api/core";

export interface GenerateScriptRequest {
  prompt: string;
  system_prompt?: string;
}

// Resolves to the written text. Rejects with a plain-language message that says what to fix.
export const GenerateScript = async (request: GenerateScriptRequest): Promise<string> => {
  const result = (await invoke("generate_script_command", { request })) as { payload: { text: string } };
  return result.payload.text;
};
