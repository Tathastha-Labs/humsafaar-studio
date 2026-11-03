import type { MediaItem } from "../../models/assets";
import { AssetType } from "../../enums";

export const demoSkyboxItems: MediaItem[] = [
  {
    version: 1,
    media_id: "SKY::m_1",
    type: AssetType.SKYBOX,
    name: "Empty Skybox",
    thumbnail: "/resources/placeholders/placeholder.png",
  },
  {
    version: 1,
    media_id: "SKY::Default",
    type: AssetType.SKYBOX,
    name: "Day Skybox",
    thumbnail: "/resources/skybox/day/px.webp",
  },
  {
    version: 1,
    media_id: "SKY::m_0",
    type: AssetType.SKYBOX,
    name: "Night Skybox",
    thumbnail: "/resources/placeholders/placeholder.png",
  },
  {
    version: 1,
    media_id: "SKY::m_3",
    type: AssetType.SKYBOX,
    name: "Gray Skybox",
    thumbnail: "/resources/placeholders/placeholder.png",
  },
  {
    version: 1,
    media_id: "SKY::m_2",
    type: AssetType.SKYBOX,
    name: "Black Skybox",
    thumbnail: "/resources/skybox/black.jpg",
  },
];
