import { useCallback, useEffect, useState } from "react";
import { LoaderCircleIcon } from "lucide-react";
import {
  OpenUrl,
  ProviderClearCredentials,
  ProviderListCredentials,
  ProviderSetApiKey,
  type ProviderApiKeyId,
} from "@storyteller/tauri-api";
import { Button } from "@storyteller/ui-button";
import { Input } from "@storyteller/ui-input";

interface ApiKeyProvider {
  id: ProviderApiKeyId;
  name: string;
  description: string;
  getKeyUrl: string;
  /** What a saved key is used for today. Absent means no feature reads it yet. */
  usedFor?: string;
  /** What it is meant for, shown while no feature reads it. */
  plannedFor?: string;
}

const API_KEY_PROVIDERS: ApiKeyProvider[] = [
  {
    id: "fal_api_key",
    name: "Fal",
    description:
      "One key for many hosted models: FLUX, Seedream, Nano Banana, Kling, Veo, Hunyuan 3D and more.",
    getKeyUrl: "https://fal.ai/dashboard/keys",
    usedFor: "Images, video, 3D and music",
  },
  {
    id: "open_ai_api_key",
    name: "OpenAI",
    description: "GPT Image models and GPT models for writing scripts.",
    getKeyUrl: "https://platform.openai.com/api-keys",
    usedFor: "Script writing",
  },
  {
    id: "anthropic_api_key",
    name: "Anthropic",
    description: "Claude models for writing scripts and shot lists.",
    getKeyUrl: "https://console.anthropic.com/settings/keys",
    plannedFor: "Script writing",
  },
  {
    id: "replicate_api_key",
    name: "Replicate",
    description: "Hosted models for video, 3D and music.",
    getKeyUrl: "https://replicate.com/account/api-tokens",
    usedFor: "Video, 3D and music",
  },
  {
    id: "gemini_api_key",
    name: "Google Gemini",
    description: "Gemini models for writing scripts and Google's Nano Banana image models.",
    getKeyUrl: "https://aistudio.google.com/apikey",
    usedFor: "Script writing and Nano Banana images",
  },
  {
    id: "gmi_cloud_api_key",
    name: "GMI Cloud",
    description: "Hosted GPU inference. Used for Seedance 2.0 video.",
    getKeyUrl: "https://console.gmicloud.ai",
    usedFor: "Seedance 2.0 video",
  },
  {
    id: "groq_api_key",
    name: "Groq",
    description: "Fast open language, speech-to-text and voice models (not xAI's Grok). The voice model needs you to accept its terms once in the Groq console.",
    getKeyUrl: "https://console.groq.com/keys",
    usedFor: "Script writing, transcription and voices",
  },
  {
    id: "grok_api_key",
    name: "xAI Grok",
    description: "Grok Imagine for image and video generation through the xAI API.",
    getKeyUrl: "https://console.x.ai",
    usedFor: "Images and video",
  },
  {
    id: "world_labs_api_key",
    name: "World Labs",
    description: "Marble, for generating explorable 3D worlds from images or text.",
    getKeyUrl: "https://platform.worldlabs.ai",
    usedFor: "3D worlds",
  },
  {
    id: "hugging_face_api_key",
    name: "Hugging Face",
    description: "Needed to download gated models for local generation.",
    getKeyUrl: "https://huggingface.co/settings/tokens",
    plannedFor: "Downloading local models",
  },
];

export const ApiKeysSettingsPane = () => {
  // Masked key start for each saved key; absent means no key is saved.
  const [savedKeys, setSavedKeys] = useState<Record<string, string>>({});
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const result = await ProviderListCredentials();
      const saved: Record<string, string> = {};
      for (const entry of result.payload.providers) {
        if (entry.credential_type === "api_key" && entry.has_credentials) {
          saved[entry.provider_credential] =
            entry.maybe_details?.maybe_key_start ?? "********";
        }
      }
      setSavedKeys(saved);
      setLoadError(false);
    } catch (e) {
      console.error("Error loading API keys", e);
      setLoadError(true);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <div className="flex flex-col gap-5 pt-3 text-base-fg">
      <p className="text-sm text-base-fg/70">
        Keys are stored on this computer and only sent to the provider they
        belong to. Add one to use that provider's models anywhere in the app.
      </p>

      {isLoading ? (
        <LoaderCircleIcon className="animate-spin text-base-fg/60" />
      ) : (
        API_KEY_PROVIDERS.map((provider, index) => (
          <div key={provider.id} className="flex flex-col gap-5">
            {index > 0 && <hr className="border-ui-panel-border" />}
            <ApiKeyBlock
              provider={provider}
              savedKeyStart={savedKeys[provider.id]}
              onChanged={refresh}
            />
          </div>
        ))
      )}

      {loadError && (
        <p className="text-sm text-red">Could not read your saved keys.</p>
      )}
    </div>
  );
};

const ApiKeyBlock = ({
  provider,
  savedKeyStart,
  onChanged,
}: {
  provider: ApiKeyProvider;
  savedKeyStart: string | undefined;
  onChanged: () => Promise<void>;
}) => {
  const [draft, setDraft] = useState("");
  const [isBusy, setIsBusy] = useState(false);
  const [error, setError] = useState<string | undefined>(undefined);

  const run = async (action: () => Promise<unknown>, failure: string) => {
    setIsBusy(true);
    setError(undefined);
    try {
      await action();
      await onChanged();
    } catch (e) {
      console.error(failure, e);
      setError(failure);
    } finally {
      setIsBusy(false);
    }
  };

  const save = () =>
    run(async () => {
      await ProviderSetApiKey(provider.id, draft.trim());
      setDraft("");
    }, "Could not save the key.");

  const remove = () =>
    run(() => ProviderClearCredentials(provider.id), "Could not remove the key.");

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 flex-col gap-0.5">
          <p className="font-mono text-[11px] font-semibold uppercase tracking-[0.12em]">
            {provider.name}
          </p>
          <p className="text-sm text-base-fg/70">{provider.description}</p>
          <p className="text-xs text-base-fg/50">
            {provider.usedFor
              ? `In use: ${provider.usedFor}`
              : `Saved keys are not used yet. Planned: ${provider.plannedFor}`}
          </p>
        </div>
        <button
          type="button"
          className="shrink-0 text-xs text-blue-400 underline hover:text-blue-300"
          onClick={() => void OpenUrl(provider.getKeyUrl)}
        >
          Get a key
        </button>
      </div>

      {savedKeyStart ? (
        <div className="flex items-center justify-between gap-3">
          <p className="truncate font-mono text-sm text-base-fg/80">
            {savedKeyStart}
          </p>
          <Button
            variant="destructive"
            className="h-9 shrink-0 px-3"
            onClick={remove}
            disabled={isBusy}
          >
            Remove
          </Button>
        </div>
      ) : (
        <div className="flex items-center gap-2">
          <Input
            className="grow"
            type="password"
            autoComplete="off"
            spellCheck={false}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            placeholder={`Paste your ${provider.name} key`}
          />
          <Button
            variant="primary"
            className="h-10 shrink-0 px-4"
            onClick={save}
            disabled={isBusy || draft.trim().length === 0}
          >
            {isBusy ? <LoaderCircleIcon className="animate-spin text-sm" /> : "Save"}
          </Button>
        </div>
      )}

      {error && <p className="text-sm text-red">{error}</p>}
    </div>
  );
};
