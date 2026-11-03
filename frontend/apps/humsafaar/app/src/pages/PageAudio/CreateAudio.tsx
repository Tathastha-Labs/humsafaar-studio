import { GenerateAudio } from "@storyteller/tauri-api";
import type { OmniGenAudioModelDetails, OmniGenAudioRequest } from "@storyteller/api";
import { GenerationProvider } from "@storyteller/api-enums";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { PromptBoxAudio } from "@storyteller/ui-promptbox";
import { missingKeyHelp, useSavedKeys } from "@storyteller/ui-model-selector";
import {
  UploadAudioMedia,
  UploadImageMedia,
  FilterMediaClasses,
} from "@storyteller/api";
import {
  galleryModalLightboxImage,
  galleryModalLightboxMediaId,
  galleryModalLightboxVisible,
  galleryModalLightboxNavPrev,
  galleryModalLightboxNavNext,
} from "@storyteller/ui-gallery-modal";
import { HelpMenuButton } from "@storyteller/ui-help-menu";
import {
  useGalleryData,
  useLastViewedGenerationStore,
  type GalleryItem,
} from "@storyteller/ui-generation-list";
import { DesktopCreatePageShell } from "~/components/generation-feed/DesktopCreatePageShell";
import { useDesktopGenerationFeed } from "~/components/generation-feed/useDesktopGenerationFeed";
import { DesktopGenerationGallery } from "~/components/generation-feed/DesktopGenerationGallery";

export const LOCAL_MUSIC_MODEL_ID = "ace_step_1p5_turbo";

// Music runs on the local engine; hosted audio models return once a cloud provider is wired in.
const LOCAL_MUSIC_MODEL: OmniGenAudioModelDetails = {
  model: LOCAL_MUSIC_MODEL_ID,
  model_creator: "ace_step",
  full_name: "ACE-Step 1.5 (Local)",
  extra_info:
    "Open-source music model that runs on this computer. Makes about 30 seconds of music. Free, but slow: expect around a minute or more on Apple Silicon or a good NVIDIA card.",
  extra_info_short: "Local, free, about 30s of music",
  text_prompt_supported: true,
  style_prompt_supported: true,
  instrumental_toggle_supported: true,
  bpm_supported: true,
  musical_key_supported: true,
};

export const FAL_MUSIC_MODEL_ID = "ace_step_fal";

// The same open model on Fal's servers: needs a Fal key, costs a little, and makes longer songs fast.
const FAL_MUSIC_MODEL: OmniGenAudioModelDetails = {
  model: FAL_MUSIC_MODEL_ID,
  model_creator: "ace_step",
  full_name: "ACE-Step (Fal)",
  extra_info:
    "The ACE-Step music model run on Fal with your own Fal key. Makes about a minute of music and finishes in seconds. Fal charges your account.",
  extra_info_short: "Cloud, about 60s of music, needs a Fal key",
  text_prompt_supported: true,
  style_prompt_supported: true,
  instrumental_toggle_supported: true,
};

export const REPLICATE_MUSIC_MODEL_ID = "minimax_music_replicate";

// Sings the lyrics, so it needs some. There is no instrumental mode.
const REPLICATE_MUSIC_MODEL: OmniGenAudioModelDetails = {
  model: REPLICATE_MUSIC_MODEL_ID,
  model_creator: "minimax",
  full_name: "MiniMax Music 2.5 (Replicate)",
  extra_info:
    "Songs with sung vocals, made on Replicate with your own Replicate key. Write the lyrics in the prompt box and the style in the style box. Replicate charges your account.",
  extra_info_short: "Cloud, sung lyrics, needs a Replicate key",
  text_prompt_supported: true,
  style_prompt_supported: true,
};

const LOCAL_MUSIC_MODELS = [LOCAL_MUSIC_MODEL, FAL_MUSIC_MODEL, REPLICATE_MUSIC_MODEL];

const generateAudio = async (request: OmniGenAudioRequest) => {
  const provider =
    request.model === LOCAL_MUSIC_MODEL_ID
      ? GenerationProvider.ComfyUi
      : request.model === FAL_MUSIC_MODEL_ID
        ? GenerationProvider.Fal
        : request.model === REPLICATE_MUSIC_MODEL_ID
          ? GenerationProvider.Replicate
          : undefined;
  return (await GenerateAudio({ ...request, provider })).payload;
};

const MODEL_PROVIDER: Record<string, GenerationProvider> = {
  [FAL_MUSIC_MODEL_ID]: GenerationProvider.Fal,
  [REPLICATE_MUSIC_MODEL_ID]: GenerationProvider.Replicate,
};

const AUDIO_FILTER = [FilterMediaClasses.AUDIO];

const CreateAudio = () => {
  const savedKeys = useSavedKeys();
  const modelTrailing = useCallback((id: string) => missingKeyHelp(MODEL_PROVIDER[id], savedKeys), [savedKeys]);
  const models = LOCAL_MUSIC_MODELS;
  const promptContentRef = useRef<HTMLDivElement>(null);
  const [promptHeight, setPromptHeight] = useState<number>(138);

  // Track the promptbox height so the feed can pad past it.
  useEffect(() => {
    const el = promptContentRef.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    const update = () => setPromptHeight(el.offsetHeight);
    update();
    const ro = new ResizeObserver(() => update());
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  // Tauri enqueues and persists audio jobs; the shared session poller and
  // library provide the same generation feed used by the webapp.
  const feed = useDesktopGenerationFeed({ mediaType: "audio" });
  const gallery = useGalleryData({
    filterMediaClasses: AUDIO_FILTER,
    excludeUploads: true,
  });

  // Content only — while the gallery is still loading the shell keeps the
  // hero + background up as a splash and fades them out when items land.
  const hasContent =
    feed.inProgress.length > 0 ||
    feed.failed.length > 0 ||
    feed.newlyCompleted.length > 0 ||
    gallery.items.length > 0;

  const newlyCompletedTokens = useMemo(
    () => new Set(feed.newlyCompleted.map((item) => item.id)),
    [feed.newlyCompleted],
  );

  // Flat, time-sorted completed list driving lightbox prev/next navigation.
  const flatCompleted = useMemo(() => {
    const seen = new Set<string>();
    const merged: GalleryItem[] = [];
    for (const item of feed.newlyCompleted) {
      if (!seen.has(item.id)) {
        seen.add(item.id);
        merged.push(item);
      }
    }
    for (const item of gallery.items) {
      if (!seen.has(item.id)) {
        seen.add(item.id);
        merged.push(item);
      }
    }
    merged.sort(
      (a, b) =>
        new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime(),
    );
    return merged;
  }, [feed.newlyCompleted, gallery.items]);

  const flatCompletedRef = useRef(flatCompleted);
  flatCompletedRef.current = flatCompleted;

  // Open a completed row in the global lightbox (rendered by TopBar's
  // gallery modal); prev/next walk the merged feed order.
  const lastViewedId = useLastViewedGenerationStore((s) => s.id);

  const openInLightbox = useCallback((item: GalleryItem) => {
    useLastViewedGenerationStore.getState().setId(item.id);
    const list = flatCompletedRef.current;
    const index = list.findIndex((i) => i.id === item.id);
    galleryModalLightboxNavPrev.value =
      index > 0 ? () => openInLightbox(list[index - 1]) : null;
    galleryModalLightboxNavNext.value =
      index >= 0 && index < list.length - 1
        ? () => openInLightbox(list[index + 1])
        : null;
    galleryModalLightboxMediaId.value = item.id;
    galleryModalLightboxImage.value = {
      id: item.id,
      label: item.label,
      thumbnail: item.thumbnail,
      fullImage: item.fullImage,
      createdAt: item.createdAt,
      mediaClass: item.mediaClass,
    };
    galleryModalLightboxVisible.value = true;
  }, []);

  return (
    <DesktopCreatePageShell
      hasContent={hasContent}
      emptyStateTitle="Create Audio"
      emptyStateSubtitle="Describe a song, a sound, or a sample"
      bottomOffset={promptHeight + 40}
      listContent={
        <DesktopGenerationGallery
          inProgressJobs={feed.inProgress}
          failedJobs={feed.failed}
          onDismissFailed={feed.dismissFailed}
          newlyCompletedItems={feed.newlyCompleted}
          galleryItems={gallery.items}
          newlyCompletedTokens={newlyCompletedTokens}
          hasMore={gallery.hasMore}
          isLoading={gallery.isLoading}
          isInitialLoading={gallery.isInitialLoading}
          onLoadMore={gallery.loadMore}
          onGalleryItemClick={openInLightbox}
          lastViewedId={lastViewedId}
        />
      }
      promptBox={
        <div className="fixed bottom-4 left-1/2 z-20 w-full max-w-5xl -translate-x-1/2 px-2 sm:px-4">
          <div ref={promptContentRef}>
            <PromptBoxAudio
              generateAudio={generateAudio}
              models={models}
              modelTrailing={modelTrailing}
              uploadAudio={UploadAudioMedia}
              uploadImage={UploadImageMedia}
              onEnqueuePressed={async () => {
                // Nudge the shared jobs poller so the pending cards appear
                // immediately (one per created job token).
                window.dispatchEvent(new Event("task-queue-update"));
              }}
            />
          </div>
        </div>
      }
      bottomRight={<HelpMenuButton />}
    />
  );
};

export default CreateAudio;
