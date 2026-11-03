import { useCallback, useEffect, useState } from "react";
import { LoaderCircleIcon } from "lucide-react";
import {
  ComfyUiDeleteModel,
  ComfyUiDownloadModel,
  ComfyUiGetStatus,
  ComfyUiInstall,
  ComfyUiStart,
  ComfyUiStop,
  type ComfyUiEngineStatus,
  type ComfyUiGetStatusSuccess,
  type LocalModelEntry,
  type LocalModelFit,
} from "@storyteller/tauri-api";
import { Button } from "@storyteller/ui-button";

type EngineInfo = ComfyUiGetStatusSuccess["payload"];

/** Installed together with the engine. */
const STARTER_MODEL_ID = "stable_diffusion_1p5";
const ENGINE_SIZE_BYTES = 1_800_000_000;

const POLL_BUSY_MS = 1000;
const POLL_IDLE_MS = 5000;

const isBusy = (info: EngineInfo) =>
  info.status.state === "installing" ||
  info.status.state === "starting" ||
  info.models.some((model) => model.state.state === "downloading");

const formatGigabytes = (bytes: number) => `${(bytes / 1_000_000_000).toFixed(1)} GB`;

export const LocalModelsSettingsPane = () => {
  const [info, setInfo] = useState<EngineInfo | undefined>(undefined);
  const [error, setError] = useState<string | undefined>(undefined);

  const refresh = useCallback(async () => {
    try {
      const result = await ComfyUiGetStatus();
      setInfo(result.payload);
      setError(undefined);
    } catch (e) {
      console.error("Error reading the local engine status", e);
      setError("Could not read the local engine status.");
    }
  }, []);

  // Check often while something is happening, and rarely otherwise.
  useEffect(() => {
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout> | undefined;

    const tick = async () => {
      await refresh();
      if (!cancelled) {
        timer = setTimeout(tick, POLL_IDLE_MS);
      }
    };
    void tick();

    return () => {
      cancelled = true;
      if (timer) clearTimeout(timer);
    };
  }, [refresh]);

  useEffect(() => {
    if (!info || !isBusy(info)) return;
    const timer = setInterval(() => void refresh(), POLL_BUSY_MS);
    return () => clearInterval(timer);
  }, [info, refresh]);

  const act = async (action: () => Promise<unknown>, failure: string) => {
    setError(undefined);
    try {
      await action();
    } catch (e) {
      console.error(failure, e);
      setError(failure);
    }
    await refresh();
  };

  return (
    <div className="flex flex-col gap-5 pt-3 text-base-fg">
      <p className="text-sm text-base-fg/70">
        Run image, video and music models on this computer. It is free, private and works
        offline once installed.
      </p>

      {!info ? (
        <LoaderCircleIcon className="animate-spin text-base-fg/60" />
      ) : (
        <>
          <ComputerBlock info={info} />
          <StatusBlock info={info} act={act} />
          <ModelsBlock info={info} act={act} />
        </>
      )}

      {error && <p className="text-sm text-red">{error}</p>}
    </div>
  );
};

const ComputerBlock = ({ info }: { info: EngineInfo }) => (
  <div className="flex flex-col gap-0.5">
    <p className="font-mono text-[11px] font-semibold uppercase tracking-[0.12em]">
      This computer
    </p>
    <p className="text-sm font-medium text-base-fg/80">{info.hardware.summary}</p>
  </div>
);

const StatusBlock = ({
  info,
  act,
}: {
  info: EngineInfo;
  act: (action: () => Promise<unknown>, failure: string) => Promise<void>;
}) => {
  const { status } = info;
  const starter = info.models.find((model) => model.id === STARTER_MODEL_ID);
  const sizeText = formatGigabytes((starter?.size_bytes ?? 0) + ENGINE_SIZE_BYTES);

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between gap-3">
        <div className="flex min-w-0 flex-col gap-0.5">
          <p className="font-mono text-[11px] font-semibold uppercase tracking-[0.12em]">
            Local engine
          </p>
          <p className="text-sm font-medium text-base-fg/80">{statusText(status, info.is_external)}</p>
        </div>

        {!info.is_external && status.state === "not_installed" && (
          <Button
            variant="primary"
            className="h-9 shrink-0 px-3"
            onClick={() => act(ComfyUiInstall, "Could not start the install.")}
          >
            Install
          </Button>
        )}
        {!info.is_external && status.state === "stopped" && (
          <Button
            variant="secondary"
            className="h-9 shrink-0 px-3"
            onClick={() => act(ComfyUiStart, "Could not start the engine.")}
          >
            Start
          </Button>
        )}
        {!info.is_external && status.state === "running" && (
          <Button
            variant="secondary"
            className="h-9 shrink-0 px-3"
            onClick={() => act(ComfyUiStop, "Could not stop the engine.")}
          >
            Stop
          </Button>
        )}
        {!info.is_external && status.state === "failed" && (
          <Button
            variant="primary"
            className="h-9 shrink-0 px-3"
            onClick={() => act(ComfyUiInstall, "Could not start the install.")}
          >
            Try again
          </Button>
        )}
      </div>

      {!info.is_external && status.state === "not_installed" && (
        <p className="text-xs text-base-fg/50">
          Downloads about {sizeText}: the engine, then {starter?.display_name ?? "a starter model"}.
          You can keep using the app while it installs.
        </p>
      )}

      {status.state === "installing" && <InstallProgress status={status} />}

      {status.state === "failed" && (
        <p className="whitespace-pre-wrap break-words font-mono text-xs text-red">
          {status.message}
        </p>
      )}

      {!info.is_external && status.state === "running" && (
        <p className="text-xs text-base-fg/50">
          It stops by itself after {info.idle_stop_minutes} minutes without use, and starts again when
          you next make something locally.
        </p>
      )}

      {!info.is_external && (
        <p className="break-all text-xs text-base-fg/40">Stored in {info.install_folder}</p>
      )}
    </div>
  );
};

const InstallProgress = ({
  status,
}: {
  status: Extract<ComfyUiEngineStatus, { state: "installing" }>;
}) => {
  const percent = status.fraction != null ? Math.round(status.fraction * 100) : undefined;

  return (
    <div className="flex flex-col gap-1.5">
      <p className="text-xs text-base-fg/70">
        Step {status.step_index + 1} of {status.step_count}: {status.label}
      </p>
      <div className="h-1.5 w-full overflow-hidden rounded-[2px] bg-white/10">
        <div
          className={percent == null ? "h-full w-1/3 animate-pulse bg-primary" : "h-full bg-primary transition-all"}
          style={percent == null ? undefined : { width: `${percent}%` }}
        />
      </div>
      {status.detail && (
        <p className="truncate font-mono text-[11px] text-base-fg/40">{status.detail}</p>
      )}
    </div>
  );
};

const ModelsBlock = ({
  info,
  act,
}: {
  info: EngineInfo;
  act: (action: () => Promise<unknown>, failure: string) => Promise<void>;
}) => (
  <div className="flex flex-col gap-4">
    <hr className="border-ui-panel-border" />
    <p className="font-mono text-[11px] font-semibold uppercase tracking-[0.12em]">
      Models
    </p>
    {info.models.map((model) => (
      <ModelRow key={model.id} model={model} act={act} />
    ))}
  </div>
);

const ModelRow = ({
  model,
  act,
}: {
  model: LocalModelEntry;
  act: (action: () => Promise<unknown>, failure: string) => Promise<void>;
}) => {
  const { state } = model;
  const discouraged =
    model.assessment.level === "very_slow" || model.assessment.level === "not_recommended";
  const percent =
    state.state === "downloading" && state.total_bytes
      ? Math.min(100, Math.round((state.downloaded_bytes / state.total_bytes) * 100))
      : undefined;

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between gap-3">
        <div className="flex min-w-0 flex-col gap-0.5">
          <p className="text-sm font-medium">{model.display_name}</p>
          <p className="text-xs text-base-fg/50">
            {formatGigabytes(model.size_bytes)} · {model.license}
          </p>
        </div>

        {state.state === "not_installed" && (
          <Button
            variant="secondary"
            className="h-9 shrink-0 px-3"
            onClick={() => act(() => ComfyUiDownloadModel(model.id), `Could not start downloading ${model.display_name}.`)}
          >
            {discouraged ? "Download anyway" : "Download"}
          </Button>
        )}
        {state.state === "installed" && (
          <Button
            variant="destructive"
            className="h-9 shrink-0 px-3"
            onClick={() => act(() => ComfyUiDeleteModel(model.id), `Could not remove ${model.display_name}.`)}
          >
            Remove
          </Button>
        )}
        {state.state === "failed" && (
          <Button
            variant="primary"
            className="h-9 shrink-0 px-3"
            onClick={() => act(() => ComfyUiDownloadModel(model.id), `Could not start downloading ${model.display_name}.`)}
          >
            Try again
          </Button>
        )}
        {state.state === "downloading" && (
          <span className="shrink-0 font-mono text-xs text-base-fg/60">
            {percent != null ? `${percent}%` : "Downloading"}
          </span>
        )}
      </div>

      <FitNotice model={model} />

      {state.state === "downloading" && (
        <div className="h-1.5 w-full overflow-hidden rounded-[2px] bg-white/10">
          <div
            className={percent == null ? "h-full w-1/3 animate-pulse bg-primary" : "h-full bg-primary transition-all"}
            style={percent == null ? undefined : { width: `${percent}%` }}
          />
        </div>
      )}
      {state.state === "failed" && (
        <p className="whitespace-pre-wrap break-words font-mono text-xs text-red">{state.message}</p>
      )}
    </div>
  );
};

const FIT_TITLES: Record<LocalModelFit, string> = {
  good: "Runs well on this computer",
  slow: "Will be slow on this computer",
  very_slow: "Will be very slow on this computer",
  not_recommended: "Not recommended for this computer",
};

const FIT_STYLES: Record<LocalModelFit, string> = {
  good: "border-green-500/30 text-green-400",
  slow: "border-amber-400/40 text-amber-300",
  very_slow: "border-amber-400/40 text-amber-300",
  not_recommended: "border-red/50 text-red",
};

const FitNotice = ({ model }: { model: LocalModelEntry }) => {
  const { level, message } = model.assessment;
  return (
    <div className="flex flex-col gap-1.5">
      <div className={`flex flex-col gap-0.5 rounded-[3px] border px-3 py-2 ${FIT_STYLES[level]}`}>
        <p className="text-xs font-semibold">{FIT_TITLES[level]}</p>
        {level !== "good" && <p className="text-xs text-base-fg/70">{message}</p>}
      </div>
      <p className="text-[11px] text-base-fg/40">Minimum: {model.minimum_hardware}</p>
      <p className="text-[11px] text-base-fg/40">Comfortable: {model.recommended_hardware}</p>
    </div>
  );
};

const statusText = (status: ComfyUiEngineStatus, isExternal: boolean): string => {
  if (isExternal && status.state !== "unsupported") {
    return "Using a ComfyUI you run yourself";
  }
  switch (status.state) {
    case "unsupported":
      return `Not available on this computer. ${status.reason}`;
    case "not_installed":
      return "Not installed";
    case "installing":
      return "Installing";
    case "stopped":
      return "Installed, not running";
    case "starting":
      return "Starting";
    case "running":
      return "Running";
    case "failed":
      return "Something went wrong";
  }
};
