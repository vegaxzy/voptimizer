use std::collections::HashMap;

mod admin;
mod backup;
mod minecraft;
mod startup;
mod tools;
mod tweaks_impl;
mod util;

use backup::{BackupEntry, BackupOpResult, HistoryEntry, RestorePointStatus};
use minecraft::{DnsInfo, PingResult, PresetResult, ProcessInfo, SystemInfo};
use startup::{StartupApp, StartupOpResult};
use tools::{
    BenchmarkComparison, BenchmarkStateResult, CleanResult, GameSessionStatus, MinecraftMonitor,
    OverlayInfo, ProcessLoad, ShaderCacheEntry, SystemSnapshot,
};
use tweaks_impl::TweakOpResult;

// ── Tweak commands ─────────────────────────────────────────────────────────

#[tauri::command]
async fn apply_tweak(tweak_id: String) -> Result<TweakOpResult, String> {
    let result = tweaks_impl::apply_impl(&tweak_id);
    backup::record_history(HistoryEntry {
        id: backup::new_id(),
        timestamp: backup::now_ms(),
        action: "apply_tweak".to_string(),
        category: "tweak".to_string(),
        target: tweak_id,
        success: result.success,
        message: result.message.clone(),
        ..Default::default()
    });
    Ok(result)
}

#[tauri::command]
async fn revert_tweak(tweak_id: String) -> Result<TweakOpResult, String> {
    let result = tweaks_impl::revert_impl(&tweak_id);
    backup::record_history(HistoryEntry {
        id: backup::new_id(),
        timestamp: backup::now_ms(),
        action: "revert_tweak".to_string(),
        category: "tweak".to_string(),
        target: tweak_id,
        success: result.success,
        message: result.message.clone(),
        ..Default::default()
    });
    Ok(result)
}

#[tauri::command]
async fn check_tweak_status(tweak_id: String) -> Result<bool, String> {
    Ok(tweaks_impl::check_status_impl(&tweak_id))
}

#[tauri::command]
async fn check_all_tweak_statuses(tweak_ids: Vec<String>) -> Result<HashMap<String, bool>, String> {
    Ok(tweaks_impl::check_all_statuses_impl(&tweak_ids))
}

#[tauri::command]
async fn detect_nvidia() -> Result<bool, String> {
    Ok(tweaks_impl::detect_nvidia_impl())
}

#[tauri::command]
async fn detect_amd() -> Result<bool, String> {
    Ok(minecraft::detect_amd_impl())
}

/// Opens a native Windows file-picker (via PowerShell/WinForms) and returns
/// the selected exe path, or `null` if the user cancelled.
#[tauri::command]
async fn pick_exe_file() -> Option<String> {
    tweaks_impl::pick_exe_file_pub()
}

/// Applies the "Disable Fullscreen Optimizations" compatibility flag to a
/// specific executable path. The path is obtained via `pick_exe_file`.
#[tauri::command]
async fn apply_exe_fullscreen_opt(path: String) -> Result<TweakOpResult, String> {
    let result = tweaks_impl::apply_exe_fsopt_pub(&path);
    backup::record_history(HistoryEntry {
        id: backup::new_id(),
        timestamp: backup::now_ms(),
        action: "apply_tweak".to_string(),
        category: "tweak".to_string(),
        target: format!("disable-fullscreen-optimizations-selected-exe:{}", path),
        success: result.success,
        message: result.message.clone(),
        ..Default::default()
    });
    Ok(result)
}

// ── Startup Apps commands ──────────────────────────────────────────────────

#[tauri::command]
async fn list_startup_apps() -> Result<Vec<StartupApp>, String> {
    Ok(startup::list_impl())
}

#[tauri::command]
async fn disable_startup_app(id: String) -> Result<StartupOpResult, String> {
    let result = startup::disable_impl(id.clone());
    let target = id.splitn(2, ':').nth(1).unwrap_or(&id).to_string();
    backup::record_history(HistoryEntry {
        id: backup::new_id(),
        timestamp: backup::now_ms(),
        action: "disable_startup".to_string(),
        category: "startup".to_string(),
        target,
        success: result.success,
        message: result.message.clone(),
        ..Default::default()
    });
    Ok(result)
}

#[tauri::command]
async fn enable_startup_app(id: String) -> Result<StartupOpResult, String> {
    let result = startup::enable_impl(id.clone());
    let target = id.splitn(2, ':').nth(1).unwrap_or(&id).to_string();
    backup::record_history(HistoryEntry {
        id: backup::new_id(),
        timestamp: backup::now_ms(),
        action: "enable_startup".to_string(),
        category: "startup".to_string(),
        target,
        success: result.success,
        message: result.message.clone(),
        ..Default::default()
    });
    Ok(result)
}

// ── Backup commands ────────────────────────────────────────────────────────

#[tauri::command]
async fn list_backups() -> Result<Vec<BackupEntry>, String> {
    Ok(backup::list_backups_impl())
}

#[tauri::command]
async fn create_registry_backup(
    label: String,
    registry_key: String,
) -> Result<BackupOpResult, String> {
    let result = backup::create_registry_backup_impl(label.clone(), registry_key.clone());
    backup::record_history(HistoryEntry {
        id: backup::new_id(),
        timestamp: backup::now_ms(),
        action: "create_backup".to_string(),
        category: "backup".to_string(),
        target: format!("{} ({})", label, registry_key),
        success: result.success,
        message: result.message.clone(),
        ..Default::default()
    });
    Ok(result)
}

#[tauri::command]
async fn restore_registry_file(id: String) -> Result<BackupOpResult, String> {
    let result = backup::restore_registry_file_impl(id);
    let target = result
        .data
        .as_ref()
        .map(|e| e.label.clone())
        .unwrap_or_else(|| "unknown".to_string());
    backup::record_history(HistoryEntry {
        id: backup::new_id(),
        timestamp: backup::now_ms(),
        action: "restore_backup".to_string(),
        category: "backup".to_string(),
        target,
        success: result.success,
        message: result.message.clone(),
        ..Default::default()
    });
    Ok(result)
}

#[tauri::command]
async fn delete_backup(id: String) -> Result<BackupOpResult, String> {
    let result = backup::delete_backup_impl(id);
    backup::record_history(HistoryEntry {
        id: backup::new_id(),
        timestamp: backup::now_ms(),
        action: "delete_backup".to_string(),
        category: "backup".to_string(),
        target: result.message.clone(),
        success: result.success,
        message: result.message.clone(),
        ..Default::default()
    });
    Ok(result)
}

#[tauri::command]
async fn check_restore_point_status() -> Result<RestorePointStatus, String> {
    Ok(backup::check_restore_point_status_impl())
}

#[tauri::command]
async fn create_restore_point(description: String) -> Result<BackupOpResult, String> {
    let result = backup::create_restore_point_impl(description.clone());
    backup::record_history(HistoryEntry {
        id: backup::new_id(),
        timestamp: backup::now_ms(),
        action: "create_restore_point".to_string(),
        category: "backup".to_string(),
        target: description,
        success: result.success,
        message: result.message.clone(),
        ..Default::default()
    });
    Ok(result)
}

#[tauri::command]
async fn list_history() -> Result<Vec<HistoryEntry>, String> {
    Ok(backup::list_history_impl())
}

#[tauri::command]
async fn clear_history() -> Result<BackupOpResult, String> {
    Ok(backup::clear_history_impl())
}

// ── Admin commands ─────────────────────────────────────────────────────────

#[tauri::command]
async fn is_running_as_admin() -> Result<bool, String> {
    Ok(admin::is_admin_impl())
}

#[tauri::command]
async fn restart_as_admin(app: tauri::AppHandle) -> Result<(), String> {
    admin::restart_as_admin_impl()?;
    app.exit(0);
    Ok(())
}

// ── Minecraft commands ─────────────────────────────────────────────────────

#[tauri::command]
async fn get_system_info() -> Result<SystemInfo, String> {
    Ok(minecraft::get_system_info_impl())
}

#[tauri::command]
async fn list_processes() -> Result<Vec<ProcessInfo>, String> {
    Ok(minecraft::list_processes_impl())
}

#[tauri::command]
async fn kill_process(pid: u32, name: String) -> Result<TweakOpResult, String> {
    let result = minecraft::kill_process_impl(pid, &name);
    backup::record_history(HistoryEntry {
        id: backup::new_id(),
        timestamp: backup::now_ms(),
        action: "kill_process".to_string(),
        category: "minecraft".to_string(),
        target: format!("{} (PID {})", name, pid),
        success: result.success,
        message: result.message.clone(),
        ..Default::default()
    });
    Ok(result)
}

#[tauri::command]
async fn flush_dns() -> Result<TweakOpResult, String> {
    let result = minecraft::flush_dns_impl();
    backup::record_history(HistoryEntry {
        id: backup::new_id(),
        timestamp: backup::now_ms(),
        action: "flush_dns".to_string(),
        category: "minecraft".to_string(),
        target: "DNS cache".to_string(),
        success: result.success,
        message: result.message.clone(),
        ..Default::default()
    });
    Ok(result)
}

#[tauri::command]
async fn get_dns_info() -> Result<DnsInfo, String> {
    Ok(minecraft::get_dns_info_impl())
}

#[tauri::command]
async fn ping_host(host: String) -> Result<PingResult, String> {
    Ok(minecraft::ping_host_impl(&host))
}

#[tauri::command]
async fn apply_minecraft_preset(tweak_ids: Vec<String>) -> Result<Vec<PresetResult>, String> {
    let results = minecraft::apply_minecraft_preset_impl(&tweak_ids);
    for r in &results {
        backup::record_history(HistoryEntry {
            id: backup::new_id(),
            timestamp: backup::now_ms(),
            action: "apply_tweak".to_string(),
            category: "minecraft_preset".to_string(),
            target: r.tweak_id.clone(),
            success: r.success,
            message: r.message.clone(),
            ..Default::default()
        });
    }
    Ok(results)
}

// ── Gaming Tools commands ──────────────────────────────────────────────────

#[tauri::command]
async fn detect_overlays() -> Result<Vec<OverlayInfo>, String> {
    Ok(tools::detect_overlays_impl())
}

#[tauri::command]
async fn scan_background_load() -> Result<Vec<ProcessLoad>, String> {
    Ok(tools::scan_background_load_impl())
}

#[tauri::command]
async fn get_shader_caches() -> Result<Vec<ShaderCacheEntry>, String> {
    Ok(tools::get_shader_caches_impl())
}

#[tauri::command]
async fn clean_shader_caches(ids: Vec<String>) -> Result<CleanResult, String> {
    let result = tools::clean_shader_caches_impl(ids.clone());
    backup::record_history(HistoryEntry {
        id: backup::new_id(),
        timestamp: backup::now_ms(),
        action: "clean_shader_caches".to_string(),
        category: "tools".to_string(),
        target: ids.join(", "),
        success: result.success,
        message: result.message.clone(),
        ..Default::default()
    });
    Ok(result)
}

#[tauri::command]
async fn start_game_session() -> Result<TweakOpResult, String> {
    let result = tools::start_game_session_impl();
    backup::record_history(HistoryEntry {
        id: backup::new_id(),
        timestamp: backup::now_ms(),
        action: "start_game_session".to_string(),
        category: "tools".to_string(),
        target: "game_session".to_string(),
        success: result.success,
        message: result.message.clone(),
        ..Default::default()
    });
    Ok(result)
}

#[tauri::command]
async fn end_game_session() -> Result<TweakOpResult, String> {
    let result = tools::end_game_session_impl();
    backup::record_history(HistoryEntry {
        id: backup::new_id(),
        timestamp: backup::now_ms(),
        action: "end_game_session".to_string(),
        category: "tools".to_string(),
        target: "game_session".to_string(),
        success: result.success,
        message: result.message.clone(),
        ..Default::default()
    });
    Ok(result)
}

#[tauri::command]
async fn get_game_session_status() -> Result<GameSessionStatus, String> {
    Ok(tools::get_game_session_status_impl())
}

#[tauri::command]
async fn get_minecraft_monitor() -> Result<MinecraftMonitor, String> {
    Ok(tools::get_minecraft_monitor_impl())
}

#[tauri::command]
async fn take_snapshot(label: String, slot: String) -> Result<SystemSnapshot, String> {
    Ok(tools::take_snapshot_impl(label, slot))
}

#[tauri::command]
async fn get_benchmark_state() -> Result<BenchmarkStateResult, String> {
    Ok(tools::get_benchmark_state_impl())
}

#[tauri::command]
async fn get_benchmark_comparison() -> Result<Option<BenchmarkComparison>, String> {
    Ok(tools::get_benchmark_comparison_impl())
}

#[tauri::command]
async fn clear_benchmark() -> Result<(), String> {
    tools::clear_benchmark_impl();
    Ok(())
}

// ── App entry ──────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            // Admin
            is_running_as_admin,
            restart_as_admin,
            // Tweaks
            apply_tweak,
            revert_tweak,
            check_tweak_status,
            check_all_tweak_statuses,
            detect_nvidia,
            detect_amd,
            pick_exe_file,
            apply_exe_fullscreen_opt,
            // Startup
            list_startup_apps,
            disable_startup_app,
            enable_startup_app,
            // Backup
            list_backups,
            create_registry_backup,
            restore_registry_file,
            delete_backup,
            check_restore_point_status,
            create_restore_point,
            list_history,
            clear_history,
            // Minecraft
            get_system_info,
            list_processes,
            kill_process,
            flush_dns,
            get_dns_info,
            ping_host,
            apply_minecraft_preset,
            // Gaming Tools
            detect_overlays,
            scan_background_load,
            get_shader_caches,
            clean_shader_caches,
            start_game_session,
            end_game_session,
            get_game_session_status,
            get_minecraft_monitor,
            take_snapshot,
            get_benchmark_state,
            get_benchmark_comparison,
            clear_benchmark,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
