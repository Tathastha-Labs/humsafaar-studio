import { useEffect, useState } from "react";
import { CircleHelpIcon } from "lucide-react";
import { OpenUrl, ProviderListCredentials } from "@storyteller/tauri-api";
import { GenerationProvider } from "@storyteller/api-enums";
import { Tooltip } from "@storyteller/ui-tooltip";

// Providers a model can run on that bill the user's own account, with the Rust credential name and where to sign up.
const KEY_INFO: Partial<Record<GenerationProvider, { credential: string; name: string; url: string }>> = {
  [GenerationProvider.Fal]: { credential: "fal_api_key", name: "Fal", url: "https://fal.ai/dashboard/keys" },
  [GenerationProvider.Replicate]: { credential: "replicate_api_key", name: "Replicate", url: "https://replicate.com/account/api-tokens" },
  [GenerationProvider.Grok]: { credential: "grok_api_key", name: "xAI", url: "https://console.x.ai" },
  [GenerationProvider.GmiCloud]: { credential: "gmi_cloud_api_key", name: "GMI Cloud", url: "https://console.gmicloud.ai" },
  [GenerationProvider.Gemini]: { credential: "gemini_api_key", name: "Gemini", url: "https://aistudio.google.com/apikey" },
  [GenerationProvider.WorldLabs]: { credential: "world_labs_api_key", name: "World Labs", url: "https://platform.worldlabs.ai" },
};

/** Credential names that have a saved key. Null until the app has answered; refreshed when the window regains focus. */
export const useSavedKeys = (): Set<string> | null => {
  const [saved, setSaved] = useState<Set<string> | null>(null);
  useEffect(() => {
    const load = () =>
      ProviderListCredentials()
        .then((r) => setSaved(new Set(r.payload.providers.filter((p) => p.has_credentials).map((p) => p.provider_credential))))
        .catch(() => setSaved(null));
    load();
    window.addEventListener("focus", load);
    return () => window.removeEventListener("focus", load);
  }, []);
  return saved;
};

/** A "?" for a model whose provider needs an API key the user has not saved yet; clicking opens the sign-up page. */
export const missingKeyHelp = (provider: GenerationProvider | undefined, saved: Set<string> | null) => {
  const info = provider ? KEY_INFO[provider] : undefined;
  if (!info || !saved || saved.has(info.credential)) return undefined;
  return (
    <Tooltip content={`Needs a ${info.name} API key. Click to get one, then paste it in Settings.`} position="top" className="z-50">
      <button
        type="button"
        aria-label={`Get a ${info.name} API key`}
        className="mr-1 text-base-fg/60 hover:text-base-fg"
        onClick={(e) => {
          e.stopPropagation();
          OpenUrl(info.url);
        }}
      >
        <CircleHelpIcon size={16} />
      </button>
    </Tooltip>
  );
};
