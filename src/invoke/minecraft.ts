import { invoke } from "@tauri-apps/api/core";

export interface SystemInfo {
  cpu_name: string;
  gpu_name: string;
  gpu_vendor: string;
  ram_total_mb: number;
  ram_used_mb: number;
  power_plan_name: string;
  power_plan_guid: string;
  gamedvr_enabled: boolean;
  game_bar_capture_enabled: boolean;
  animations_enabled: boolean;
  edge_background_enabled: boolean;
  edge_startup_boost_enabled: boolean;
  startup_app_count: number;
  minecraft_running: boolean;
  nvidia_detected: boolean;
  amd_detected: boolean;
}

export interface ProcessInfo {
  pid: number;
  name: string;
  memory_mb: number;
  is_safe_to_kill: boolean;
}

export interface PingResult {
  success: boolean;
  host: string;
  latency_ms: number | null;
  error: string | null;
}

export interface PresetResult {
  tweak_id: string;
  success: boolean;
  message: string;
}

export interface DnsInfo {
  servers: string[];
  hostname: string;
}

export async function getSystemInfo(): Promise<SystemInfo> {
  return invoke<SystemInfo>("get_system_info");
}

export async function listProcesses(): Promise<ProcessInfo[]> {
  return invoke<ProcessInfo[]>("list_processes");
}

export async function killProcess(pid: number, name: string): Promise<{ success: boolean; message: string; error: string | null }> {
  return invoke("kill_process", { pid, name });
}

export async function flushDns(): Promise<{ success: boolean; message: string; error: string | null }> {
  return invoke("flush_dns");
}

export async function getDnsInfo(): Promise<DnsInfo> {
  return invoke<DnsInfo>("get_dns_info");
}

export async function pingHost(host: string): Promise<PingResult> {
  return invoke<PingResult>("ping_host", { host });
}

export async function applyMinecraftPreset(tweakIds: string[]): Promise<PresetResult[]> {
  return invoke<PresetResult[]>("apply_minecraft_preset", { tweakIds });
}
