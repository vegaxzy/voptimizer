import { invoke } from "@tauri-apps/api/core";

export interface CleanCategory {
  id: string;
  name: string;
  description: string;
  size_mb: number;
  file_count: number;
  exists: boolean;
  requires_admin: boolean;
  irreversible: boolean;
  default_selected: boolean;
}

export interface DebloatResult {
  success: boolean;
  categories_cleaned: number;
  total_mb_freed: number;
  errors: string[];
  message: string;
}

/** Measures each cleanable category (read-only — never deletes). */
export async function scanCleanup(): Promise<CleanCategory[]> {
  return invoke<CleanCategory[]>("scan_cleanup");
}

/** Cleans the selected categories and returns how much was freed. */
export async function cleanCleanup(ids: string[]): Promise<DebloatResult> {
  return invoke<DebloatResult>("clean_cleanup", { ids });
}

// ── Bloatware (UWP) remover ─────────────────────────────────────────────────

export interface AppxPackage {
  id: string;
  name: string;
  publisher: string;
  category: "bloat" | "app" | "system";
  removable: boolean;
  recommended: boolean;
  note: string;
}

/** Lists installed Store/UWP apps, classified (read-only). */
export async function listAppx(): Promise<AppxPackage[]> {
  return invoke<AppxPackage[]>("list_appx");
}

/** Removes the selected apps (per-user; protected packages are refused). */
export async function removeAppx(ids: string[]): Promise<DebloatResult> {
  return invoke<DebloatResult>("remove_appx", { ids });
}
