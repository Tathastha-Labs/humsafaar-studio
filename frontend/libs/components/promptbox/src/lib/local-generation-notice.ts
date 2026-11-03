import { GenerationProvider } from "@storyteller/api-enums";
import { toast } from "@storyteller/ui-toaster";

const STARTING_MESSAGE =
  "Starting on your computer. This can take a few minutes, and the first run takes longer while the engine warms up.";

/** Longer than any realistic wait, so it is dismissed by the caller and not by a timer. */
const NOTICE_DURATION_MS = 10 * 60 * 1000;

/**
 * Local generation is far slower than a hosted service, and the first run also starts the
 * engine. Shows a loading notice saying so, and returns a function that removes it.
 * Does nothing for hosted providers.
 */
export const showLocalStartNotice = (
  provider: GenerationProvider | string | undefined,
): (() => void) => {
  if (provider !== GenerationProvider.ComfyUi) {
    return () => {};
  }
  const id = toast.loading(STARTING_MESSAGE, { duration: NOTICE_DURATION_MS });
  return () => toast.dismiss(id);
};
