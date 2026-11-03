import { MediaFilesApi } from "./MediaFilesApi.js";
import {
  FilterMediaClasses,
  FilterMediaType,
  FilterEngineCategories,
} from "./enums/QueryFilters.js";

interface ListUserMediaQuery {
  username?: string;
  filter_media_classes?: FilterMediaClasses[];
  filter_media_type?: FilterMediaType[];
  filter_engine_categories?: FilterEngineCategories[];
  user_uploads_only?: boolean;
  include_user_uploads?: boolean;
  page_index?: number;
  page_size?: number;
}

export class GalleryModalApi extends MediaFilesApi {
  public async listUserMediaFiles(query: ListUserMediaQuery) {
    return await this.ListUserMediaFiles(query);
  }
}
