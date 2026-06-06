import { invoke } from "@tauri-apps/api/core";
import type { StartupApp, StartupOpResult } from "../types/startup";

export async function listStartupApps(): Promise<StartupApp[]> {
  return invoke<StartupApp[]>("list_startup_apps");
}

export async function disableStartupApp(id: string): Promise<StartupOpResult> {
  return invoke<StartupOpResult>("disable_startup_app", { id });
}

export async function enableStartupApp(id: string): Promise<StartupOpResult> {
  return invoke<StartupOpResult>("enable_startup_app", { id });
}
