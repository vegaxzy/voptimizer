use crate::backup;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;

// ── Result type ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExperimentalOpResult {
    pub success: bool,
    pub message: String,
    pub error: Option<String>,
}

impl ExperimentalOpResult {
    fn ok(msg: impl Into<String>) -> Self {
        Self { success: true, message: msg.into(), error: None }
    }
    fn fail(msg: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { success: false, message: msg.into(), error: Some(detail.into()) }
    }
}

// ── Persistent state ───────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Default, Debug)]
struct ExpState {
    onedrive_path: Option<String>,
    widgets_original: Option<u32>,
    applied: HashMap<String, bool>,
}

fn state_path() -> std::path::PathBuf {
    backup::get_app_data_dir().join("exp_state.json")
}

fn load_state() -> ExpState {
    backup::read_json_metadata(&state_path())
}

fn save_state(state: &ExpState) {
    let _ = backup::write_json_metadata(&state_path(), state);
}

// ── Path helpers ───────────────────────────────────────────────────────────

fn sys_root() -> String {
    std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string())
}

fn sc_path() -> String {
    format!("{}\\System32\\sc.exe", sys_root())
}

fn ps_path() -> String {
    format!(
        "{}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
        sys_root()
    )
}

// ── OneDrive startup ───────────────────────────────────────────────────────

pub fn disable_onedrive_startup_impl() -> ExperimentalOpResult {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        KEY_READ | KEY_WRITE,
    ) {
        Ok(k) => k,
        Err(e) => return ExperimentalOpResult::fail("Cannot open Run registry key", e.to_string()),
    };

    let path: String = match key.get_value("OneDrive") {
        Ok(v) => v,
        Err(_) => {
            return ExperimentalOpResult::fail(
                "OneDrive startup entry not found",
                "OneDrive may not be installed or is already disabled",
            )
        }
    };

    let mut state = load_state();
    state.onedrive_path = Some(path);
    save_state(&state);

    match key.delete_value("OneDrive") {
        Ok(_) => {
            let mut state = load_state();
            state.applied.insert("disable-onedrive-startup".into(), true);
            save_state(&state);
            ExperimentalOpResult::ok("OneDrive startup entry removed")
        }
        Err(e) => ExperimentalOpResult::fail(
            "Failed to remove OneDrive startup entry",
            e.to_string(),
        ),
    }
}

pub fn revert_onedrive_startup_impl() -> ExperimentalOpResult {
    use winreg::enums::*;
    use winreg::RegKey;

    let state = load_state();
    let path = match state.onedrive_path {
        Some(p) => p,
        None => {
            return ExperimentalOpResult::fail(
                "No saved OneDrive path found",
                "Disable OneDrive startup via VOptimizer before trying to revert",
            )
        }
    };

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        KEY_WRITE,
    ) {
        Ok(k) => k,
        Err(e) => return ExperimentalOpResult::fail("Cannot open Run registry key", e.to_string()),
    };

    match key.set_value("OneDrive", &path) {
        Ok(_) => {
            let mut state = load_state();
            state.applied.remove("disable-onedrive-startup");
            save_state(&state);
            ExperimentalOpResult::ok("OneDrive startup entry restored")
        }
        Err(e) => {
            ExperimentalOpResult::fail("Failed to restore OneDrive startup entry", e.to_string())
        }
    }
}

// ── Widgets ────────────────────────────────────────────────────────────────

pub fn disable_widgets_impl() -> ExperimentalOpResult {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced",
        KEY_READ | KEY_WRITE,
    ) {
        Ok(k) => k,
        Err(e) => {
            return ExperimentalOpResult::fail(
                "Cannot open Explorer Advanced registry key",
                e.to_string(),
            )
        }
    };

    let original: u32 = key.get_value("TaskbarDa").unwrap_or(1u32);

    let mut state = load_state();
    state.widgets_original = Some(original);
    save_state(&state);

    match key.set_value("TaskbarDa", &0u32) {
        Ok(_) => {
            let mut state = load_state();
            state.applied.insert("disable-widgets".into(), true);
            save_state(&state);
            ExperimentalOpResult::ok(
                "Widgets hidden from taskbar. Sign out or restart Explorer for the change to take effect.",
            )
        }
        Err(e) => ExperimentalOpResult::fail("Failed to disable Widgets", e.to_string()),
    }
}

pub fn revert_widgets_impl() -> ExperimentalOpResult {
    use winreg::enums::*;
    use winreg::RegKey;

    let state = load_state();
    let original = state.widgets_original.unwrap_or(1u32);

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced",
        KEY_WRITE,
    ) {
        Ok(k) => k,
        Err(e) => {
            return ExperimentalOpResult::fail(
                "Cannot open Explorer Advanced registry key",
                e.to_string(),
            )
        }
    };

    match key.set_value("TaskbarDa", &original) {
        Ok(_) => {
            let mut state = load_state();
            state.applied.remove("disable-widgets");
            save_state(&state);
            ExperimentalOpResult::ok(
                "Widgets restored to taskbar. Sign out or restart Explorer for the change to take effect.",
            )
        }
        Err(e) => ExperimentalOpResult::fail("Failed to restore Widgets", e.to_string()),
    }
}

// ── SysMain service ────────────────────────────────────────────────────────

pub fn disable_sysmain_impl() -> ExperimentalOpResult {
    let sc = sc_path();

    let out = Command::new(&sc)
        .args(["config", "SysMain", "start=", "disabled"])
        .output();

    match out {
        Err(e) => return ExperimentalOpResult::fail("Failed to run sc.exe", e.to_string()),
        Ok(o) if !o.status.success() => {
            let stderr = String::from_utf8_lossy(&o.stderr).into_owned();
            let stdout = String::from_utf8_lossy(&o.stdout).into_owned();
            let detail = if stderr.trim().is_empty() { stdout } else { stderr };
            return ExperimentalOpResult::fail(
                "Failed to disable SysMain — administrator privileges required",
                detail.trim().to_string(),
            );
        }
        Ok(_) => {}
    }

    // Stop the running service (ignore failure — may already be stopped)
    let _ = Command::new(&sc).args(["stop", "SysMain"]).output();

    let mut state = load_state();
    state.applied.insert("disable-sysmain".into(), true);
    save_state(&state);

    ExperimentalOpResult::ok("SysMain (Superfetch) service disabled and stopped")
}

pub fn revert_sysmain_impl() -> ExperimentalOpResult {
    let sc = sc_path();

    let out = Command::new(&sc)
        .args(["config", "SysMain", "start=", "auto"])
        .output();

    match out {
        Err(e) => return ExperimentalOpResult::fail("Failed to run sc.exe", e.to_string()),
        Ok(o) if !o.status.success() => {
            let stderr = String::from_utf8_lossy(&o.stderr).into_owned();
            let stdout = String::from_utf8_lossy(&o.stdout).into_owned();
            let detail = if stderr.trim().is_empty() { stdout } else { stderr };
            return ExperimentalOpResult::fail(
                "Failed to re-enable SysMain — administrator privileges required",
                detail.trim().to_string(),
            );
        }
        Ok(_) => {}
    }

    // Start the service (ignore failure — may need a reboot)
    let _ = Command::new(&sc).args(["start", "SysMain"]).output();

    let mut state = load_state();
    state.applied.remove("disable-sysmain");
    save_state(&state);

    ExperimentalOpResult::ok("SysMain (Superfetch) service re-enabled")
}

// ── NVIDIA telemetry ───────────────────────────────────────────────────────

pub fn detect_nvidia_impl() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    hklm.open_subkey("SOFTWARE\\NVIDIA Corporation").is_ok()
        || hklm
            .open_subkey("SOFTWARE\\WOW6432Node\\NVIDIA Corporation")
            .is_ok()
}

pub fn disable_nvidia_telemetry_impl() -> ExperimentalOpResult {
    let ps = ps_path();
    let script = concat!(
        "$tasks = Get-ScheduledTask 2>$null | Where-Object {",
        " $_.TaskName -like '*NvTm*' -or",
        " $_.TaskName -like '*NvDriver*' -or",
        " $_.TaskName -like '*NvNode*' };",
        "if ($tasks) {",
        " $tasks | Disable-ScheduledTask | Out-Null;",
        " Write-Output \"Disabled $($tasks.Count) NVIDIA telemetry task(s)\"",
        "} else {",
        " Write-Output 'No NVIDIA telemetry tasks found'",
        "}"
    );

    match Command::new(&ps)
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output()
    {
        Err(e) => ExperimentalOpResult::fail("Failed to run PowerShell", e.to_string()),
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
            if !stderr.is_empty() && !o.status.success() {
                ExperimentalOpResult::fail("PowerShell returned an error", stderr)
            } else {
                let mut state = load_state();
                state.applied.insert("disable-nvidia-telemetry".into(), true);
                save_state(&state);
                ExperimentalOpResult::ok(if stdout.is_empty() {
                    "NVIDIA telemetry tasks processed".to_string()
                } else {
                    stdout
                })
            }
        }
    }
}

pub fn revert_nvidia_telemetry_impl() -> ExperimentalOpResult {
    let ps = ps_path();
    let script = concat!(
        "$tasks = Get-ScheduledTask 2>$null | Where-Object {",
        " $_.TaskName -like '*NvTm*' -or",
        " $_.TaskName -like '*NvDriver*' -or",
        " $_.TaskName -like '*NvNode*' };",
        "if ($tasks) {",
        " $tasks | Enable-ScheduledTask | Out-Null;",
        " Write-Output \"Re-enabled $($tasks.Count) NVIDIA telemetry task(s)\"",
        "} else {",
        " Write-Output 'No NVIDIA telemetry tasks found'",
        "}"
    );

    match Command::new(&ps)
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output()
    {
        Err(e) => ExperimentalOpResult::fail("Failed to run PowerShell", e.to_string()),
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
            if !stderr.is_empty() && !o.status.success() {
                ExperimentalOpResult::fail("PowerShell returned an error", stderr)
            } else {
                let mut state = load_state();
                state.applied.remove("disable-nvidia-telemetry");
                save_state(&state);
                ExperimentalOpResult::ok(if stdout.is_empty() {
                    "NVIDIA telemetry tasks processed".to_string()
                } else {
                    stdout
                })
            }
        }
    }
}

// ── Dispatch ───────────────────────────────────────────────────────────────

pub fn apply_impl(tweak_id: &str) -> ExperimentalOpResult {
    match tweak_id {
        "disable-onedrive-startup" => disable_onedrive_startup_impl(),
        "disable-widgets" => disable_widgets_impl(),
        "disable-sysmain" => disable_sysmain_impl(),
        "disable-nvidia-telemetry" => disable_nvidia_telemetry_impl(),
        _ => ExperimentalOpResult::fail(
            format!("'{}' is a placeholder — not yet implemented", tweak_id),
            "placeholder",
        ),
    }
}

pub fn revert_impl(tweak_id: &str) -> ExperimentalOpResult {
    match tweak_id {
        "disable-onedrive-startup" => revert_onedrive_startup_impl(),
        "disable-widgets" => revert_widgets_impl(),
        "disable-sysmain" => revert_sysmain_impl(),
        "disable-nvidia-telemetry" => revert_nvidia_telemetry_impl(),
        _ => ExperimentalOpResult::fail(
            format!("'{}' cannot be reverted — not yet implemented", tweak_id),
            "placeholder",
        ),
    }
}

pub fn get_status_impl(tweak_id: &str) -> bool {
    load_state()
        .applied
        .get(tweak_id)
        .copied()
        .unwrap_or(false)
}

pub fn get_all_statuses_impl(ids: &[String]) -> HashMap<String, bool> {
    let state = load_state();
    ids.iter()
        .map(|id| {
            let applied = state.applied.get(id.as_str()).copied().unwrap_or(false);
            (id.clone(), applied)
        })
        .collect()
}
