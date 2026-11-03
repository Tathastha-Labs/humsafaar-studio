import { invoke } from "@tauri-apps/api/core";
import { CommandResult } from "../common/CommandStatus";

// Rust is the source of truth: `ProviderCredentialKey` (snake_case).
export type ProviderApiKeyId =
  | "fal_api_key"
  | "replicate_api_key"
  | "open_ai_api_key"
  | "anthropic_api_key"
  | "hugging_face_api_key"
  | "gemini_api_key"
  | "gmi_cloud_api_key"
  | "grok_api_key"
  | "groq_api_key"
  | "world_labs_api_key";

export interface ProviderCredentialEntry {
  provider_credential: string;
  credential_type: "api_key" | "web_login";
  has_credentials: boolean;
  maybe_details?: {
    /** For API keys: the first few characters, then asterisks. The full key is never sent to the UI. */
    maybe_key_start?: string | null;
    maybe_email_address?: string | null;
    maybe_username?: string | null;
  } | null;
}

export interface ProviderListCredentialsSuccess extends CommandResult {
  payload: {
    providers: ProviderCredentialEntry[];
  };
}

export const ProviderListCredentials = async (): Promise<ProviderListCredentialsSuccess> => {
  return await invoke("provider_list_command") as ProviderListCredentialsSuccess;
}

export const ProviderSetApiKey = async (
  providerCredential: ProviderApiKeyId,
  apiKey: string,
): Promise<CommandResult> => {
  return await invoke("provider_set_api_key_command", {
    request: {
      provider_credential: providerCredential,
      api_key: apiKey,
    },
  }) as CommandResult;
}

export const ProviderClearCredentials = async (
  providerCredential: ProviderApiKeyId,
): Promise<CommandResult> => {
  return await invoke("provider_clear_command", {
    request: {
      provider_credential: providerCredential,
    },
  }) as CommandResult;
}
