import { invoke } from "@tauri-apps/api/core";
import type { TweakOpResult } from "../types";

// ── Overlay Detector ───────────────────────────────────────────────────────

export interface OverlayInfo {
  id: string;
  name: string;
  process_name: string;
  detected: boolean;
  pid: number | null;
  category: string;
  tip: string;
}

export async function detectOverlays(): Promise<OverlayInfo[]> {
  return invoke<OverlayInfo[]>("detect_overlays");
}

// ── Background Load Scanner ────────────────────────────────────────────────

export interface ProcessLoad {
  pid: number;
  name: string;
  ram_mb: number;
  cpu_s: number;
  is_gaming_impact: boolean;
  impact_reason: string;
}

export async function scanBackgroundLoad(): Promise<ProcessLoad[]> {
  return invoke<ProcessLoad[]>("scan_background_load");
}

// ── Shader Cache Cleaner ───────────────────────────────────────────────────

export interface ShaderCacheEntry {
  id: string;
  name: string;
  path: string;
  size_mb: number;
  exists: boolean;
  vendor: string;
}

export interface CleanResult {
  success: boolean;
  cleaned_count: number;
  total_mb_freed: number;
  errors: string[];
  message: string;
}

export async function getShaderCaches(): Promise<ShaderCacheEntry[]> {
  return invoke<ShaderCacheEntry[]>("get_shader_caches");
}

export async function cleanShaderCaches(ids: string[]): Promise<CleanResult> {
  return invoke<CleanResult>("clean_shader_caches", { ids });
}

// ── Game Session Mode ──────────────────────────────────────────────────────

export interface GameSessionStatus {
  active: boolean;
  started_at_ms: number;
  duration_secs: number;
  actions_applied: string[];
}

export async function startGameSession(): Promise<TweakOpResult> {
  return invoke<TweakOpResult>("start_game_session");
}

export async function endGameSession(): Promise<TweakOpResult> {
  return invoke<TweakOpResult>("end_game_session");
}

export async function getGameSessionStatus(): Promise<GameSessionStatus> {
  return invoke<GameSessionStatus>("get_game_session_status");
}

// ── Minecraft Process Monitor ──────────────────────────────────────────────

export interface MinecraftMonitor {
  found: boolean;
  instance_count: number;
  pid: number | null;
  ram_mb: number;
  cpu_s: number;
  window_title: string;
}

export async function getMinecraftMonitor(): Promise<MinecraftMonitor> {
  return invoke<MinecraftMonitor>("get_minecraft_monitor");
}

// ── Benchmark Session Mode ─────────────────────────────────────────────────

export interface SystemSnapshot {
  label: string;
  timestamp_ms: number;
  ram_used_mb: number;
  ram_total_mb: number;
  process_count: number;
  uptime_secs: number;
  power_plan_name: string;
}

export interface BenchmarkComparison {
  before: SystemSnapshot;
  after: SystemSnapshot;
  duration_secs: number;
  ram_delta_mb: number;
  process_delta: number;
}

export interface BenchmarkStateResult {
  before: SystemSnapshot | null;
  after: SystemSnapshot | null;
}

/** `slot` = `"before"` | `"after"` */
export async function takeSnapshot(label: string, slot: string): Promise<SystemSnapshot> {
  return invoke<SystemSnapshot>("take_snapshot", { label, slot });
}

export async function getBenchmarkState(): Promise<BenchmarkStateResult> {
  return invoke<BenchmarkStateResult>("get_benchmark_state");
}

export async function getBenchmarkComparison(): Promise<BenchmarkComparison | null> {
  return invoke<BenchmarkComparison | null>("get_benchmark_comparison");
}

export async function clearBenchmark(): Promise<void> {
  return invoke<void>("clear_benchmark");
}
