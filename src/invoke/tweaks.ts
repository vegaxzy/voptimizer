import { invoke } from "@tauri-apps/api/core";
import type { TweakOpResult } from "../types";

export async function applyTweak(tweakId: string): Promise<TweakOpResult> {
  return invoke<TweakOpResult>("apply_tweak", { tweakId });
}

export async function revertTweak(tweakId: string): Promise<TweakOpResult> {
  return invoke<TweakOpResult>("revert_tweak", { tweakId });
}

export async function checkTweakStatus(tweakId: string): Promise<boolean> {
  return invoke<boolean>("check_tweak_status", { tweakId });
}

export async function checkAllTweakStatuses(
  tweakIds: string[]
): Promise<Record<string, boolean>> {
  return invoke<Record<string, boolean>>("check_all_tweak_statuses", { tweakIds });
}

export async function detectNvidia(): Promise<boolean> {
  return invoke<boolean>("detect_nvidia");
}

export async function detectAmd(): Promise<boolean> {
  return invoke<boolean>("detect_amd");
}

/** Opens a native file-picker. Returns the selected exe path or null if cancelled. */
export async function pickExeFile(): Promise<string | null> {
  return invoke<string | null>("pick_exe_file");
}

/** Applies DISABLEDXMAXIMIZEDWINDOWEDMODE to the given exe path. */
export async function applyExeFullscreenOpt(path: string): Promise<TweakOpResult> {
  return invoke<TweakOpResult>("apply_exe_fullscreen_opt", { path });
}

/** Sets a High process-priority profile (IFEO PerfOptions) for the given exe. */
export async function applyExePriority(path: string): Promise<TweakOpResult> {
  return invoke<TweakOpResult>("apply_exe_priority", { path });
}

/** Sets the high-performance GPU preference (HKCU) for the given exe. */
export async function applyExeGpuPref(path: string): Promise<TweakOpResult> {
  return invoke<TweakOpResult>("apply_exe_gpu_pref", { path });
}

/** Maps a per-exe tweak id → its apply invoke. Used by the file-picker flow. */
export const PER_EXE_APPLY: Record<string, (path: string) => Promise<TweakOpResult>> = {
  "disable-fullscreen-optimizations-selected-exe": applyExeFullscreenOpt,
  "set-game-priority-selected-exe": applyExePriority,
  "prefer-high-perf-gpu-selected-exe": applyExeGpuPref,
};
