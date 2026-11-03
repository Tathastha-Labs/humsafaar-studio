import { invoke } from "@tauri-apps/api/core";
import { CommandResult } from "../common/CommandStatus";

// Rust is the source of truth: `EngineStatus` in the comfyui_manager crate.
export type ComfyUiEngineStatus =
  | { state: "unsupported"; reason: string }
  | { state: "not_installed" }
  | {
      state: "installing";
      label: string;
      step_index: number;
      step_count: number;
      detail?: string | null;
      fraction?: number | null;
    }
  | { state: "stopped" }
  | { state: "starting" }
  | { state: "running"; base_url: string }
  | { state: "failed"; message: string };

// Rust is the source of truth: `ModelState` in the comfyui_manager crate.
export type LocalModelState =
  | { state: "not_installed" }
  | { state: "downloading"; downloaded_bytes: number; total_bytes?: number | null }
  | { state: "installed" }
  | { state: "failed"; message: string };

// Rust is the source of truth: `Suitability` and `Assessment` in the comfyui_manager crate.
export type LocalModelFit = "good" | "slow" | "very_slow" | "not_recommended";

export interface LocalModelEntry {
  id: string;
  display_name: string;
  license: string;
  size_bytes: number;
  state: LocalModelState;
  /** What it needs at the least, and what makes it comfortable. */
  minimum_hardware: string;
  recommended_hardware: string;
  /** How well it will run on this computer. */
  assessment: { level: LocalModelFit; message: string };
}

export interface ComfyUiGetStatusSuccess extends CommandResult {
  payload: {
    status: ComfyUiEngineStatus;
    /** A ComfyUI the user runs; the app does not install or start it. */
    is_external: boolean;
    /** What this computer has to run models with. */
    hardware: { summary: string; gpu_kind: "apple_silicon" | "nvidia" | "none" };
    /** Every model the app can download. */
    models: LocalModelEntry[];
    install_folder: string;
    /** The engine stops by itself after this many idle minutes, and starts again when needed. */
    idle_stop_minutes: number;
  };
}

export const ComfyUiGetStatus = async (): Promise<ComfyUiGetStatusSuccess> => {
  return await invoke("comfyui_get_status_command") as ComfyUiGetStatusSuccess;
}

/** Installs the engine and the starter model, then starts it. Returns at once. */
export const ComfyUiInstall = async (): Promise<CommandResult> => {
  return await invoke("comfyui_install_command") as CommandResult;
}

/** Returns at once; follow progress with `ComfyUiGetStatus`. */
export const ComfyUiStart = async (): Promise<CommandResult> => {
  return await invoke("comfyui_start_command") as CommandResult;
}

export const ComfyUiStop = async (): Promise<CommandResult> => {
  return await invoke("comfyui_stop_command") as CommandResult;
}

/** Starts a download and returns at once; follow it with `ComfyUiGetStatus`. */
export const ComfyUiDownloadModel = async (modelId: string): Promise<CommandResult> => {
  return await invoke("comfyui_download_model_command", {
    request: { model_id: modelId },
  }) as CommandResult;
}

export const ComfyUiDeleteModel = async (modelId: string): Promise<CommandResult> => {
  return await invoke("comfyui_delete_model_command", {
    request: { model_id: modelId },
  }) as CommandResult;
}
