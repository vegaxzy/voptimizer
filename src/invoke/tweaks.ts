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
