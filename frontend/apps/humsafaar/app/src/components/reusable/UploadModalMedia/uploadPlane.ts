import { UploaderStates } from "~/enums";
import { UploaderState } from "~/models";
import { saveLocalFile } from "@storyteller/api";

/** Add a picture or video to the library so it can be placed in a scene as a plane. */
export const uploadPlane = async ({
  assetFile,
  progressCallback,
}: {
  title: string;
  assetFile: File;
  progressCallback: (newState: UploaderState) => void;
}) => {
  progressCallback({ status: UploaderStates.uploadingAsset });
  try {
    await saveLocalFile(assetFile);
  } catch (error) {
    progressCallback({
      status: UploaderStates.assetError,
      errorMessage: error instanceof Error ? error.message : "Could not save the file.",
    });
    return;
  }
  progressCallback({ status: UploaderStates.success });
};

/** A plane from a file already in the library needs nothing more to be saved. */
export const uploadPlaneFromMediaToken = async ({
  progressCallback,
}: {
  title: string;
  mediaToken: string;
  progressCallback: (newState: UploaderState) => void;
}) => {
  progressCallback({ status: UploaderStates.uploadingAsset });
  progressCallback({ status: UploaderStates.success });
};
