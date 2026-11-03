// Shared model store: the single source of truth for the model dropdowns.
//
// The lists are the ones in `@storyteller/model-list`. There is no hosted listing to reconcile
// with any more, so the store simply holds them.

import { create } from "zustand";
import {
  ImageModel,
  VideoModel,
  IMAGE_MODELS,
  VIDEO_MODELS,
  ALL_MODELS_LIST,
} from "@storyteller/model-list";

export interface ModelsStoreState {
  imageModels: ImageModel[];
  videoModels: VideoModel[];
}

export const useModelsStore = create<ModelsStoreState>(() => ({
  imageModels: IMAGE_MODELS,
  videoModels: VIDEO_MODELS,
}));

// Hook selectors for React consumers.
export const useImageModels = (): ImageModel[] => useModelsStore((s) => s.imageModels);
export const useVideoModels = (): VideoModel[] => useModelsStore((s) => s.videoModels);

// History/recreate looks models up by either id.
export function findDesktopModel(modelId: string) {
  const { imageModels, videoModels } = useModelsStore.getState();
  return [...imageModels, ...videoModels, ...ALL_MODELS_LIST]
    .find((model) => model.tauriId === modelId || model.id === modelId);
}
