import { useRef, useState } from "react";
import { GenerateImage, GenerateVideo, type GenerateImageRequest, type GenerateVideoRequest } from "@storyteller/tauri-api";
import { defaultModelForPage, ModelPage, useSelectedImageModel, useSelectedProviderForModel, useSelectedVideoModel } from "@storyteller/ui-model-selector";
import { usePromptImageStore } from "@storyteller/ui-promptbox";
import { useTextToImageGenerationCompleteEvent, useVideoGenerationCompleteEvent } from "@storyteller/tauri-events";
import { videoResolutionValue, type ImageModel, type VideoModel } from "@storyteller/model-list";

export type ShotMedia = {
  imagePending?: boolean;
  image?: { token: string; url: string };
  videoPending?: boolean;
  video?: { url: string };
};

const newId = () => (crypto.randomUUID ? crypto.randomUUID() : Math.random().toString(36).slice(2));

/**
 * Pictures and clips for each shot, made with whatever models are picked on Create Image and Create Video.
 * Results come back through the same completion events those pages use, matched by a per-request id.
 */
export const useShotMedia = () => {
  const [media, setMedia] = useState<Record<number, ShotMedia>>({});
  const waiting = useRef(new Map<string, { shot: number; kind: "image" | "video" }>());

  // Until a model is picked on Create Image / Create Video, use the same default those pages start with.
  const imageModel = useSelectedImageModel(ModelPage.TextToImage) ?? (defaultModelForPage([], ModelPage.TextToImage) as ImageModel | undefined);
  const imageProvider = useSelectedProviderForModel(ModelPage.TextToImage, imageModel?.id) ?? imageModel?.getProviders()[0];
  const videoModel = useSelectedVideoModel(ModelPage.ImageToVideo) ?? (defaultModelForPage([], ModelPage.ImageToVideo) as VideoModel | undefined);
  const videoProvider = useSelectedProviderForModel(ModelPage.ImageToVideo, videoModel?.id) ?? videoModel?.getProviders()[0];

  const patch = (shot: number, change: ShotMedia) => setMedia((m) => ({ ...m, [shot]: { ...m[shot], ...change } }));

  useTextToImageGenerationCompleteEvent(async (event) => {
    const hit = waiting.current.get(event.maybe_frontend_subscriber_id ?? "");
    const first = event.generated_images[0];
    if (!hit || hit.kind !== "image") return;
    waiting.current.delete(event.maybe_frontend_subscriber_id ?? "");
    patch(hit.shot, { imagePending: false, image: first ? { token: first.media_token, url: first.cdn_url } : undefined });
  });

  useVideoGenerationCompleteEvent(async (event) => {
    const hit = waiting.current.get(event.maybe_frontend_subscriber_id ?? "");
    if (!hit || hit.kind !== "video") return;
    waiting.current.delete(event.maybe_frontend_subscriber_id ?? "");
    patch(hit.shot, { videoPending: false, video: event.generated_video ? { url: event.generated_video.cdn_url } : undefined });
  });

  const generateImage = async (shot: number, prompt: string) => {
    if (!imageModel) throw "No image model is available.";
    const id = newId();
    const store = usePromptImageStore.getState();
    const request: GenerateImageRequest = {
      prompt,
      model: imageModel,
      batch_size: 1,
      frontend_caller: "mini_app",
      frontend_subscriber_id: id,
    };
    if (imageProvider) request.provider = imageProvider;
    if (imageModel.supportsNewAspectRatio()) request.aspect_ratio = store.commonAspectRatio;
    if (imageModel.supportsNewResolution()) request.resolution = store.commonResolution;
    if (imageModel.supportsQuality()) request.quality = imageModel.resolveQuality(store.commonQuality);

    waiting.current.set(id, { shot, kind: "image" });
    patch(shot, { imagePending: true });
    try {
      await GenerateImage(request);
    } catch (err) {
      waiting.current.delete(id);
      patch(shot, { imagePending: false });
      throw err;
    }
  };

  /** Starts a clip for each of `which` (default all) that has a picture. Returns how many had none. */
  const generateVideos = async (shots: string[], which = shots.map((_, i) => i)) => {
    if (!videoModel) throw "No video model is available.";
    let skipped = 0;
    for (const i of which) {
      const picture = media[i]?.image;
      if (!picture) {
        skipped++;
        continue;
      }
      const id = newId();
      const request: GenerateVideoRequest = {
        prompt: shots[i],
        model: videoModel,
        start_frame_image_media_token: picture.token,
        frontend_caller: "mini_app",
        frontend_subscriber_id: id,
      };
      if (videoProvider) request.provider = videoProvider;
      if (videoModel.defaultDuration != null) request.duration_seconds = videoModel.defaultDuration;
      if (videoModel.resolutionOptions?.length) {
        request.resolution = videoResolutionValue(videoModel.defaultResolution ?? videoModel.resolutionOptions[0]);
      }
      if (videoModel.supportsCommonAspectRatio) {
        const size = videoModel.sizeOptions.find((o) => o.tauriValue === videoModel.defaultAspectRatio) ?? videoModel.sizeOptions[0];
        if (size) request.aspect_ratio = size.tauriValue;
      }
      waiting.current.set(id, { shot: i, kind: "video" });
      patch(i, { videoPending: true });
      try {
        await GenerateVideo(request);
      } catch (err) {
        waiting.current.delete(id);
        patch(i, { videoPending: false });
        throw err;
      }
    }
    return skipped;
  };

  /** Moves media and in-flight requests to new shot positions; null drops them. */
  const remap = (to: (shot: number) => number | null) => {
    setMedia((m) => Object.fromEntries(Object.entries(m).flatMap(([k, v]) => {
      const n = to(Number(k));
      return n === null ? [] : [[n, v]];
    })));
    for (const [id, hit] of waiting.current) {
      const n = to(hit.shot);
      if (n === null) waiting.current.delete(id);
      else hit.shot = n;
    }
  };

  const reset = () => {
    waiting.current.clear();
    setMedia({});
  };

  return { media, generateImage, generateVideos, remap, reset, imageModel, imageProvider, videoModel };
};
