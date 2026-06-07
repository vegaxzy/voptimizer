import { invoke } from "@tauri-apps/api/core";

// ── Types (mirror the Rust structs in system_info.rs exactly) ───────────────

export type DriverStatus = "ok" | "outdated" | "unknown";

export interface DriverEntry {
  name: string;
  version: string;
  date: string;
  status: DriverStatus;
  note: string;
}

/**
 * Slow, static system facts (hardware / OS / BIOS / drivers). These rarely or
 * never change while the app is open, so they are fetched once and cached.
 * The live-metric fields exist on the struct for backward compatibility but are
 * NOT authoritative here — read those from `SystemLive` instead.
 */
export interface SystemStatic {
  // CPU
  cpu_name: string;
  cpu_cores: number;
  cpu_threads: number;
  // GPU
  gpu_name: string;
  gpu_vram_gb: number;
  gpu_driver_version: string;
  // RAM
  ram_total_mb: number;
  ram_type: string;
  ram_speed_mhz: number;
  // Motherboard
  motherboard: string;
  // Storage (system drive)
  storage_name: string;
  storage_partition: string;
  storage_gb: number;
  storage_free_gb: number;
  storage_media_type: string;
  storage_bus_type: string;
  storage_type: string;
  storage_health: string;
  // OS
  os_name: string;
  os_build: string;
  os_version_tag: string;
  os_install_date: string;
  os_architecture: string;
  os_locale: string;
  os_hostname: string;
  // BIOS / UEFI
  bios_vendor: string;
  bios_version: string;
  bios_release_date: string;
  bios_mode: string;
  bios_secure_boot: boolean;
  bios_age_days: number;
  // Drivers
  drivers: DriverEntry[];
  // (these live fields are present on the struct but ignored by the UI)
  cpu_pct?: number;
  ram_used_mb?: number;
  disk_used_gb?: number;
  disk_total_gb?: number;
  uptime_secs?: number;
}

/** Fast, live metrics — measured via Win32 FFI, safe to poll frequently. */
export interface SystemLive {
  cpu_pct: number;
  ram_used_mb: number;
  ram_total_mb: number;
  disk_used_gb: number;
  disk_total_gb: number;
  uptime_secs: number;
}

/** A merged view the System Overview UI renders from static + live. */
export type SystemOverview = SystemStatic & SystemLive;

// ── Invoke wrappers ─────────────────────────────────────────────────────────

/** Static hardware/OS/BIOS/driver info (slow — cache it). */
export async function getSystemStatic(): Promise<SystemStatic> {
  return invoke<SystemStatic>("get_system_static");
}

/** Live CPU/RAM/disk/uptime metrics (fast — poll it). */
export async function getSystemLive(): Promise<SystemLive> {
  return invoke<SystemLive>("get_system_live");
}

/** Combined snapshot (kept for compatibility; the UI prefers static + live). */
export async function getSystemOverview(): Promise<SystemOverview> {
  return invoke<SystemOverview>("get_system_overview");
}
