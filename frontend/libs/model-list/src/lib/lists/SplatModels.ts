import { SplatModel } from "../classes/SplatModel.js";
import { ModelCreator } from "../classes/metadata/ModelCreator.js";
import { GenerationProvider } from "@storyteller/api-enums";

// TODO: Some of the model configs, such as generation counts, are authoritatively controlled in `legacy/Models.ts`

export const SPLAT_MODELS : SplatModel [] = [
  new SplatModel({
    id: "marble_1p0_draft",
    tauriId: "marble_1p0_draft",
    fullName: "Marble 1.0 Draft",
    category: "gaussian",
    creator: ModelCreator.WorldLabs,
    selectorName: "Marble Draft",
    selectorDescription: "Fast generation, good for quick drafts",
    selectorBadges: ["~1 min."],
    providers: [GenerationProvider.WorldLabs],
    progressBarTime: 60000,
  }),
  new SplatModel({
    id: "marble_1p0",
    tauriId: "marble_1p0",
    fullName: "Marble 1.0",
    category: "gaussian",
    creator: ModelCreator.WorldLabs,
    selectorName: "Marble 1.0",
    selectorDescription: "High quality, best for final renders",
    selectorBadges: ["~5 min."],
    providers: [GenerationProvider.WorldLabs],
    progressBarTime: 300000,
  }),
  new SplatModel({
    id: "marble_1p1",
    tauriId: "marble_1p1",
    fullName: "Marble 1.1",
    category: "gaussian",
    creator: ModelCreator.WorldLabs,
    selectorName: "Marble 1.1",
    selectorDescription: "Newer model with improved detail",
    selectorBadges: ["~5 min."],
    providers: [GenerationProvider.WorldLabs],
    progressBarTime: 300000,
  }),
  new SplatModel({
    id: "marble_1p1_plus",
    tauriId: "marble_1p1_plus",
    fullName: "Marble 1.1 Plus",
    category: "gaussian",
    creator: ModelCreator.WorldLabs,
    selectorName: "Marble 1.1 Plus",
    selectorDescription: "Highest quality World Labs model",
    selectorBadges: ["~5 min."],
    providers: [GenerationProvider.WorldLabs],
    progressBarTime: 300000,
  }),
];

export const SPLAT_MODELS_BY_ID: Map<string, SplatModel> = new Map(
  SPLAT_MODELS.map((model) => [model.id, model]),
);

if (SPLAT_MODELS_BY_ID.size !== SPLAT_MODELS.length) {
  throw new Error("All splat models must have unique IDs");
}
