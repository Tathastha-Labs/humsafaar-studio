import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { GenerationProvider } from "@storyteller/api-enums";
import { ModelPage } from "./model-pages";
import {
  getSelectedProviderForModel,
  useClassyModelSelectorStore,
  useSelectedProviderForModel,
} from "./classy-model-selector-store";

const RETIRED_SELECTIONS = [
  { page: ModelPage.TextToImage, modelId: "gpt_image_1", provider: GenerationProvider.Sora },
  { page: ModelPage.ImageToVideo, modelId: "sora_2", provider: GenerationProvider.Sora },
];

afterEach(() => {
  cleanup();
  useClassyModelSelectorStore.setState({ selectedModels: {}, selectedProviders: {} });
});

it.each(RETIRED_SELECTIONS)(
  "uses Humsafaar Studio for stale $provider selections of $modelId in generation and pricing",
  ({ page, modelId, provider }) => {
    useClassyModelSelectorStore.setState({
      selectedProviders: { [page]: { [modelId]: provider } },
    });
    const { result } = renderHook(() => useSelectedProviderForModel(page, modelId));
    expect(result.current).toBe(GenerationProvider.Humsafaar);
    expect(getSelectedProviderForModel(page, modelId)).toBe(GenerationProvider.Humsafaar);

    act(() => useClassyModelSelectorStore.getState().setSelectedProvider(page, modelId, provider));
    expect(useClassyModelSelectorStore.getState().selectedProviders[page]?.[modelId])
      .toBe(GenerationProvider.Humsafaar);
  },
);

it("keeps a World Labs selection, since 3D worlds run on the user's own World Labs key", () => {
  const page = ModelPage.ImageTo3DWorld;
  act(() => useClassyModelSelectorStore.getState().setSelectedProvider(page, "marble_1p0", GenerationProvider.WorldLabs));
  expect(getSelectedProviderForModel(page, "marble_1p0")).toBe(GenerationProvider.WorldLabs);
});

it("preserves other provider selections and missing defaults", () => {
  const page = ModelPage.TextToImage;
  const modelId = "midjourney";
  const { result } = renderHook(() => useSelectedProviderForModel(page, modelId));
  expect(result.current).toBeUndefined();
  act(() => useClassyModelSelectorStore.getState().setSelectedProvider(page, modelId, GenerationProvider.Midjourney));
  expect(result.current).toBe(GenerationProvider.Midjourney);
  expect(getSelectedProviderForModel(page, modelId)).toBe(GenerationProvider.Midjourney);
});
