import { invoke } from "@tauri-apps/api/core";

export async function isRunningAsAdmin(): Promise<boolean> {
  return invoke<boolean>("is_running_as_admin");
}

export async function restartAsAdmin(): Promise<void> {
  return invoke<void>("restart_as_admin");
}
