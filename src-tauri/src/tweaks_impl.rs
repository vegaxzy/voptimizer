use crate::backup;
use crate::util::no_window_cmd;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// â”€â”€ Result type â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TweakOpResult {
    pub success: bool,
    pub message: String,
    pub error: Option<String>,
}

impl TweakOpResult {
    pub fn ok(msg: impl Into<String>) -> Self {
        Self {
            success: true,
            message: msg.into(),
            error: None,
        }
    }
    pub fn fail(msg: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            success: false,
            message: msg.into(),
            error: Some(detail.into()),
        }
    }
}

// â”€â”€ Persistent state â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Serialize, Deserialize, Default, Debug)]
struct ExpState {
    /// Preserved from v0.5.0 for backward-compat with existing exp_state.json
    #[serde(default)]
    onedrive_path: Option<String>,
    #[serde(default)]
    widgets_original: Option<u32>,
    /// Generic DWORD save map â€” key: "tweak_id:value_name"
    #[serde(default)]
    saved_dwords: HashMap<String, u32>,
    /// Generic string save map â€” key: "tweak_id:value_name"
    #[serde(default)]
    saved_strings: HashMap<String, String>,
    /// Previously active power scheme GUID
    #[serde(default)]
    previous_power_scheme: Option<String>,
    /// For tweaks that cannot be status-checked from system state directly
    #[serde(default)]
    applied: HashMap<String, bool>,
}

fn state_path() -> std::path::PathBuf {
    backup::get_app_data_dir().join("exp_state.json")
}
fn load_state() -> ExpState {
    backup::read_json_metadata(&state_path())
}
fn save_state(s: &ExpState) {
    let _ = backup::write_json_metadata(&state_path(), s);
}

// â”€â”€ Path helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn sys_root() -> String {
    std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string())
}
fn sc_exe() -> String {
    format!("{}\\System32\\sc.exe", sys_root())
}
fn ps_exe() -> String {
    format!(
        "{}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
        sys_root()
    )
}
fn powercfg_exe() -> String {
    format!("{}\\System32\\powercfg.exe", sys_root())
}

// â”€â”€ Registry helpers (HKCU) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn apply_hkcu_dword(
    state: &mut ExpState,
    tweak_id: &str,
    key_path: &str,
    value_name: &str,
    new_val: u32,
    default_orig: u32,
    ok_msg: &str,
) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = match hkcu.create_subkey(key_path) {
        Ok(k) => k,
        Err(e) => return TweakOpResult::fail("Cannot open registry key", e.to_string()),
    };
    let save_key = format!("{}:{}", tweak_id, value_name);
    let original: u32 = key.get_value(value_name).unwrap_or(default_orig);
    state.saved_dwords.entry(save_key).or_insert(original);
    match key.set_value(value_name, &new_val) {
        Ok(_) => {
            state.applied.insert(tweak_id.to_string(), true);
            TweakOpResult::ok(ok_msg)
        }
        Err(e) => TweakOpResult::fail("Failed to write registry value", e.to_string()),
    }
}

fn revert_hkcu_dword(
    state: &mut ExpState,
    tweak_id: &str,
    key_path: &str,
    value_name: &str,
    default_orig: u32,
    ok_msg: &str,
) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let save_key = format!("{}:{}", tweak_id, value_name);
    let original = state
        .saved_dwords
        .get(&save_key)
        .copied()
        .unwrap_or(default_orig);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = match hkcu.create_subkey(key_path) {
        Ok(k) => k,
        Err(e) => return TweakOpResult::fail("Cannot open registry key", e.to_string()),
    };
    match key.set_value(value_name, &original) {
        Ok(_) => {
            state.applied.remove(tweak_id);
            TweakOpResult::ok(ok_msg)
        }
        Err(e) => TweakOpResult::fail("Failed to revert registry value", e.to_string()),
    }
}

fn apply_hkcu_dwords(
    state: &mut ExpState,
    tweak_id: &str,
    entries: &[(&str, &str, u32, u32)],
    ok_msg: &str,
) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for (key_path, value_name, new_val, default_orig) in entries {
        let (key, _) = match hkcu.create_subkey(key_path) {
            Ok(k) => k,
            Err(e) => return TweakOpResult::fail("Cannot open registry key", e.to_string()),
        };
        let save_key = format!("{}:{}", tweak_id, value_name);
        let original: u32 = key.get_value(*value_name).unwrap_or(*default_orig);
        state.saved_dwords.entry(save_key).or_insert(original);
        if let Err(e) = key.set_value(*value_name, new_val) {
            return TweakOpResult::fail(format!("Failed to set {}", value_name), e.to_string());
        }
    }
    state.applied.insert(tweak_id.to_string(), true);
    TweakOpResult::ok(ok_msg)
}

fn revert_hkcu_dwords(
    state: &mut ExpState,
    tweak_id: &str,
    entries: &[(&str, &str, u32)],
    ok_msg: &str,
) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for (key_path, value_name, default_orig) in entries {
        let save_key = format!("{}:{}", tweak_id, value_name);
        let original = state
            .saved_dwords
            .get(&save_key)
            .copied()
            .unwrap_or(*default_orig);
        let (key, _) = match hkcu.create_subkey(key_path) {
            Ok(k) => k,
            Err(e) => return TweakOpResult::fail("Cannot open registry key", e.to_string()),
        };
        if let Err(e) = key.set_value(*value_name, &original) {
            return TweakOpResult::fail(format!("Failed to revert {}", value_name), e.to_string());
        }
    }
    state.applied.remove(tweak_id);
    TweakOpResult::ok(ok_msg)
}

fn apply_hkcu_string(
    state: &mut ExpState,
    tweak_id: &str,
    key_path: &str,
    value_name: &str,
    new_val: &str,
    default_orig: &str,
    ok_msg: &str,
) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = match hkcu.create_subkey(key_path) {
        Ok(k) => k,
        Err(e) => return TweakOpResult::fail("Cannot open registry key", e.to_string()),
    };
    let save_key = format!("{}:{}", tweak_id, value_name);
    let original: String = key
        .get_value(value_name)
        .unwrap_or_else(|_| default_orig.to_string());
    state.saved_strings.entry(save_key).or_insert(original);
    match key.set_value(value_name, &new_val) {
        Ok(_) => {
            state.applied.insert(tweak_id.to_string(), true);
            TweakOpResult::ok(ok_msg)
        }
        Err(e) => TweakOpResult::fail("Failed to write registry value", e.to_string()),
    }
}

fn revert_hkcu_string(
    state: &mut ExpState,
    tweak_id: &str,
    key_path: &str,
    value_name: &str,
    default_orig: &str,
    ok_msg: &str,
) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let save_key = format!("{}:{}", tweak_id, value_name);
    let original = state
        .saved_strings
        .get(&save_key)
        .cloned()
        .unwrap_or_else(|| default_orig.to_string());
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = match hkcu.create_subkey(key_path) {
        Ok(k) => k,
        Err(e) => return TweakOpResult::fail("Cannot open registry key", e.to_string()),
    };
    match key.set_value(value_name, &original.as_str()) {
        Ok(_) => {
            state.applied.remove(tweak_id);
            TweakOpResult::ok(ok_msg)
        }
        Err(e) => TweakOpResult::fail("Failed to revert registry value", e.to_string()),
    }
}

// â”€â”€ Registry helpers (HKLM) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn apply_hklm_dword(
    state: &mut ExpState,
    tweak_id: &str,
    key_path: &str,
    value_name: &str,
    new_val: u32,
    default_orig: u32,
    ok_msg: &str,
) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let (key, _) = match hklm.create_subkey(key_path) {
        Ok(k) => k,
        Err(e) => {
            return TweakOpResult::fail(
                "Cannot open registry key â€” administrator privileges required",
                e.to_string(),
            )
        }
    };
    let save_key = format!("{}:{}", tweak_id, value_name);
    let original: u32 = key.get_value(value_name).unwrap_or(default_orig);
    state.saved_dwords.entry(save_key).or_insert(original);
    match key.set_value(value_name, &new_val) {
        Ok(_) => {
            state.applied.insert(tweak_id.to_string(), true);
            TweakOpResult::ok(ok_msg)
        }
        Err(e) => TweakOpResult::fail(
            "Failed to write registry â€” run VOptimizer as administrator",
            e.to_string(),
        ),
    }
}

fn revert_hklm_dword(
    state: &mut ExpState,
    tweak_id: &str,
    key_path: &str,
    value_name: &str,
    default_orig: u32,
    ok_msg: &str,
) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let save_key = format!("{}:{}", tweak_id, value_name);
    let original = state
        .saved_dwords
        .get(&save_key)
        .copied()
        .unwrap_or(default_orig);
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let (key, _) = match hklm.create_subkey(key_path) {
        Ok(k) => k,
        Err(e) => {
            return TweakOpResult::fail(
                "Cannot open registry key â€” administrator privileges required",
                e.to_string(),
            )
        }
    };
    match key.set_value(value_name, &original) {
        Ok(_) => {
            state.applied.remove(tweak_id);
            TweakOpResult::ok(ok_msg)
        }
        Err(e) => TweakOpResult::fail(
            "Failed to revert registry â€” run VOptimizer as administrator",
            e.to_string(),
        ),
    }
}

// â”€â”€ Status check helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn hkcu_dword_eq(key_path: &str, value_name: &str, expected: u32) -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(key_path)
        .ok()
        .and_then(|k| k.get_value(value_name).ok())
        .map(|v: u32| v == expected)
        .unwrap_or(false)
}

fn hkcu_string_eq(key_path: &str, value_name: &str, expected: &str) -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(key_path)
        .ok()
        .and_then(|k| k.get_value(value_name).ok())
        .map(|v: String| v == expected)
        .unwrap_or(false)
}

fn hklm_dword_eq(key_path: &str, value_name: &str, expected: u32) -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(key_path)
        .ok()
        .and_then(|k| k.get_value(value_name).ok())
        .map(|v: u32| v == expected)
        .unwrap_or(false)
}

fn service_start_type(name: &str) -> Option<u32> {
    use winreg::enums::*;
    use winreg::RegKey;
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(format!("SYSTEM\\CurrentControlSet\\Services\\{}", name))
        .ok()?
        .get_value("Start")
        .ok()
}

fn hkcu_run_value_exists(value_name: &str) -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .ok()
        .map(|k| k.get_raw_value(value_name).is_ok())
        .unwrap_or(false)
}

/// Parse the first UUID-shaped token from arbitrary text.
/// Locale-independent â€” works regardless of the Windows UI language.
fn parse_first_guid(text: &str) -> Option<String> {
    for token in text.split_whitespace() {
        // Strip leading/trailing punctuation that powercfg sometimes adds
        let t = token.trim_matches(|c: char| c == ':' || c == '(' || c == ')' || c == '*');
        let parts: Vec<&str> = t.split('-').collect();
        if parts.len() == 5
            && [8usize, 4, 4, 4, 12]
                .iter()
                .zip(parts.iter())
                .all(|(&len, p)| p.len() == len && p.chars().all(|c| c.is_ascii_hexdigit()))
        {
            return Some(t.to_lowercase());
        }
    }
    None
}

fn get_active_power_scheme() -> Option<String> {
    let out = no_window_cmd(powercfg_exe())
        .args(["/getactivescheme"])
        .output()
        .ok()?;
    // Use from_utf8_lossy â€” safe on any Windows codepage, and GUIDs are always ASCII
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    // Parse UUID directly â€” locale-independent (avoids "Power Scheme GUID" vs "Schemat zasilania GUID" etc.)
    parse_first_guid(&text)
}

// â”€â”€ Performance tweaks â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn apply_ultimate_performance(state: &mut ExpState) -> TweakOpResult {
    const GUID: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";
    if let Some(prev) = get_active_power_scheme() {
        state.previous_power_scheme = Some(prev);
    }
    let pc = powercfg_exe();

    // Step 1: try direct activation (plan may already be visible in the list)
    if let Ok(o) = no_window_cmd(&pc).args(["/setactive", GUID]).output() {
        if o.status.success() {
            state
                .saved_strings
                .insert("ultimate-perf-guid".into(), GUID.to_string());
            state
                .applied
                .insert("set-ultimate-performance".into(), true);
            return TweakOpResult::ok("Ultimate Performance power plan activated");
        }
    }

    // Step 2: reveal the hidden scheme â€” parse the NEW guid locale-independently
    // (powercfg output is localized: "Power Scheme GUID" in EN, "Schemat zasilania GUID" in PL, etc.)
    let dup_guid: String = no_window_cmd(&pc)
        .args(["/duplicatescheme", GUID])
        .output()
        .ok()
        .and_then(|o| {
            let text = String::from_utf8_lossy(&o.stdout).to_string();
            // Find the first GUID in output that is NOT the source GUID we passed in
            text.split_whitespace()
                .filter_map(|token| {
                    let t = token.trim_matches(|c: char| c == ':' || c == '(' || c == ')');
                    let parts: Vec<&str> = t.split('-').collect();
                    if parts.len() == 5
                        && [8usize, 4, 4, 4, 12]
                            .iter()
                            .zip(parts.iter())
                            .all(|(&len, p)| {
                                p.len() == len && p.chars().all(|c| c.is_ascii_hexdigit())
                            })
                    {
                        Some(t.to_lowercase())
                    } else {
                        None
                    }
                })
                .find(|g| *g != GUID.to_lowercase())
        })
        .unwrap_or_else(|| GUID.to_string());

    // Step 3: activate using the guid we found (or original as fallback)
    match no_window_cmd(&pc).args(["/setactive", &dup_guid]).output() {
        Ok(o) if o.status.success() => {
            // Save the activated GUID so check_ultimate_performance can find it
            state
                .saved_strings
                .insert("ultimate-perf-guid".into(), dup_guid.clone());
            state
                .applied
                .insert("set-ultimate-performance".into(), true);
            TweakOpResult::ok("Ultimate Performance power plan activated")
        }
        Ok(_) => TweakOpResult::fail(
            "Could not activate Ultimate Performance plan",
            "Ensure VOptimizer is running as administrator",
        ),
        Err(_) => TweakOpResult::fail(
            "Could not run powercfg.exe",
            "Ensure VOptimizer is running as administrator",
        ),
    }
}

fn revert_ultimate_performance(state: &mut ExpState) -> TweakOpResult {
    let previous = state
        .previous_power_scheme
        .clone()
        .unwrap_or_else(|| "381b4222-f694-41f0-9685-ff5bb260df2e".to_string());
    match no_window_cmd(powercfg_exe())
        .args(["/setactive", &previous])
        .output()
    {
        Ok(o) if o.status.success() => {
            state.applied.remove("set-ultimate-performance");
            TweakOpResult::ok("Power plan restored to previous setting")
        }
        _ => TweakOpResult::fail(
            "Failed to restore previous power plan",
            "Set it manually via Control Panel > Power Options",
        ),
    }
}

fn check_ultimate_performance(state: &ExpState) -> bool {
    const GUID: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";
    let active = match get_active_power_scheme() {
        Some(g) => g.to_lowercase(),
        None => return false,
    };
    // Match original GUID
    if active == GUID {
        return true;
    }
    // Also match the GUID of a duplicated scheme we activated (different GUID, same plan)
    state
        .saved_strings
        .get("ultimate-perf-guid")
        .map(|saved| active == saved.to_lowercase())
        .unwrap_or(false)
}

// SysMain (from v0.5.0 â€” kept verbatim)
fn apply_sysmain(state: &mut ExpState) -> TweakOpResult {
    let sc = sc_exe();
    let out = no_window_cmd(&sc)
        .args(["config", "SysMain", "start=", "disabled"])
        .output();
    match out {
        Err(e) => return TweakOpResult::fail("Failed to run sc.exe", e.to_string()),
        Ok(o) if !o.status.success() => {
            let detail = stdout_or_stderr(&o);
            return TweakOpResult::fail(
                "Failed to disable SysMain â€” administrator required",
                detail,
            );
        }
        Ok(_) => {}
    }
    let _ = no_window_cmd(&sc).args(["stop", "SysMain"]).output();
    state.applied.insert("disable-sysmain".into(), true);
    TweakOpResult::ok("SysMain (Superfetch) service disabled and stopped")
}

fn revert_sysmain(state: &mut ExpState) -> TweakOpResult {
    let sc = sc_exe();
    let out = no_window_cmd(&sc)
        .args(["config", "SysMain", "start=", "auto"])
        .output();
    match out {
        Err(e) => return TweakOpResult::fail("Failed to run sc.exe", e.to_string()),
        Ok(o) if !o.status.success() => {
            let detail = stdout_or_stderr(&o);
            return TweakOpResult::fail(
                "Failed to re-enable SysMain â€” administrator required",
                detail,
            );
        }
        Ok(_) => {}
    }
    let _ = no_window_cmd(&sc).args(["start", "SysMain"]).output();
    state.applied.remove("disable-sysmain");
    TweakOpResult::ok("SysMain (Superfetch) service re-enabled")
}

// Windows Search (WSearch service)
fn apply_windows_search(state: &mut ExpState) -> TweakOpResult {
    let sc = sc_exe();
    let out = no_window_cmd(&sc)
        .args(["config", "WSearch", "start=", "disabled"])
        .output();
    match out {
        Err(e) => return TweakOpResult::fail("Failed to run sc.exe", e.to_string()),
        Ok(o) if !o.status.success() => {
            let detail = stdout_or_stderr(&o);
            return TweakOpResult::fail(
                "Failed to disable WSearch â€” administrator required",
                detail,
            );
        }
        Ok(_) => {}
    }
    let _ = no_window_cmd(&sc).args(["stop", "WSearch"]).output();
    state.applied.insert("disable-windows-search".into(), true);
    TweakOpResult::ok("Windows Search indexing service disabled and stopped")
}

fn revert_windows_search(state: &mut ExpState) -> TweakOpResult {
    let sc = sc_exe();
    let out = no_window_cmd(&sc)
        .args(["config", "WSearch", "start=", "auto"])
        .output();
    match out {
        Err(e) => return TweakOpResult::fail("Failed to run sc.exe", e.to_string()),
        Ok(o) if !o.status.success() => {
            let detail = stdout_or_stderr(&o);
            return TweakOpResult::fail(
                "Failed to re-enable WSearch â€” administrator required",
                detail,
            );
        }
        Ok(_) => {}
    }
    let _ = no_window_cmd(&sc).args(["start", "WSearch"]).output();
    state.applied.remove("disable-windows-search");
    TweakOpResult::ok("Windows Search service re-enabled")
}

fn stdout_or_stderr(o: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&o.stderr);
    let stdout = String::from_utf8_lossy(&o.stdout);
    let s = if stderr.trim().is_empty() {
        stdout
    } else {
        stderr
    };
    s.trim().to_string()
}

// â”€â”€ Gaming tweaks â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn apply_gamedvr(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_dword(
        state,
        "disable-gamedvr",
        "System\\GameConfigStore",
        "GameDVR_Enabled",
        0,
        1,
        "Game DVR / background recording disabled",
    )
}
fn revert_gamedvr(state: &mut ExpState) -> TweakOpResult {
    revert_hkcu_dword(
        state,
        "disable-gamedvr",
        "System\\GameConfigStore",
        "GameDVR_Enabled",
        1,
        "Game DVR restored",
    )
}

fn apply_game_bar_capture(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_dword(
        state,
        "disable-game-bar-capture",
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\GameDVR",
        "AppCaptureEnabled",
        0,
        1,
        "Xbox Game Bar capture disabled",
    )
}
fn revert_game_bar_capture(state: &mut ExpState) -> TweakOpResult {
    revert_hkcu_dword(
        state,
        "disable-game-bar-capture",
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\GameDVR",
        "AppCaptureEnabled",
        1,
        "Xbox Game Bar capture restored",
    )
}

fn apply_fullscreen_optimizations(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_dwords(
        state,
        "disable-fullscreen-optimizations",
        &[
            ("System\\GameConfigStore", "GameDVR_FSEBehaviorMode", 2, 0),
            (
                "System\\GameConfigStore",
                "GameDVR_HonorUserFSEBehaviorMode",
                1,
                0,
            ),
        ],
        "Fullscreen optimizations globally disabled",
    )
}
fn revert_fullscreen_optimizations(state: &mut ExpState) -> TweakOpResult {
    revert_hkcu_dwords(
        state,
        "disable-fullscreen-optimizations",
        &[
            ("System\\GameConfigStore", "GameDVR_FSEBehaviorMode", 0),
            (
                "System\\GameConfigStore",
                "GameDVR_HonorUserFSEBehaviorMode",
                0,
            ),
        ],
        "Fullscreen optimizations restored",
    )
}

fn apply_game_mode(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_dwords(
        state,
        "enable-game-mode",
        &[
            ("SOFTWARE\\Microsoft\\GameBar", "AllowAutoGameMode", 1, 0),
            ("SOFTWARE\\Microsoft\\GameBar", "AutoGameModeEnabled", 1, 0),
        ],
        "Game Mode enabled",
    )
}
fn revert_game_mode(state: &mut ExpState) -> TweakOpResult {
    revert_hkcu_dwords(
        state,
        "enable-game-mode",
        &[
            ("SOFTWARE\\Microsoft\\GameBar", "AllowAutoGameMode", 0),
            ("SOFTWARE\\Microsoft\\GameBar", "AutoGameModeEnabled", 0),
        ],
        "Game Mode settings reverted",
    )
}

fn apply_gpu_scheduling(state: &mut ExpState) -> TweakOpResult {
    apply_hklm_dword(
        state,
        "gpu-scheduling",
        "SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers",
        "HwSchMode",
        2,
        1,
        "Hardware-accelerated GPU scheduling enabled. Restart required.",
    )
}
fn revert_gpu_scheduling(state: &mut ExpState) -> TweakOpResult {
    revert_hklm_dword(
        state,
        "gpu-scheduling",
        "SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers",
        "HwSchMode",
        1,
        "GPU scheduling reverted. Restart required.",
    )
}

// â”€â”€ Privacy tweaks â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn apply_advertising_id(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_dword(
        state,
        "disable-advertising-id",
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\AdvertisingInfo",
        "Enabled",
        0,
        1,
        "Advertising ID disabled",
    )
}
fn revert_advertising_id(state: &mut ExpState) -> TweakOpResult {
    revert_hkcu_dword(
        state,
        "disable-advertising-id",
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\AdvertisingInfo",
        "Enabled",
        1,
        "Advertising ID restored",
    )
}

fn apply_windows_tips(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_dwords(
        state,
        "disable-windows-tips",
        &[
            (
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\ContentDeliveryManager",
                "SoftLandingEnabled",
                0,
                1,
            ),
            (
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\ContentDeliveryManager",
                "SubscribedContent-338389Enabled",
                0,
                1,
            ),
            (
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\ContentDeliveryManager",
                "SystemPaneSuggestionsEnabled",
                0,
                1,
            ),
        ],
        "Windows tips and suggestions disabled",
    )
}
fn revert_windows_tips(state: &mut ExpState) -> TweakOpResult {
    revert_hkcu_dwords(
        state,
        "disable-windows-tips",
        &[
            (
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\ContentDeliveryManager",
                "SoftLandingEnabled",
                1,
            ),
            (
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\ContentDeliveryManager",
                "SubscribedContent-338389Enabled",
                1,
            ),
            (
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\ContentDeliveryManager",
                "SystemPaneSuggestionsEnabled",
                1,
            ),
        ],
        "Windows tips and suggestions restored",
    )
}

fn apply_consumer_features(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_dwords(
        state,
        "disable-consumer-features",
        &[
            (
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\ContentDeliveryManager",
                "SilentInstalledAppsEnabled",
                0,
                1,
            ),
            (
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\ContentDeliveryManager",
                "PreInstalledAppsEnabled",
                0,
                1,
            ),
            (
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\ContentDeliveryManager",
                "OemPreInstalledAppsEnabled",
                0,
                1,
            ),
        ],
        "Consumer features (silent app installs) disabled",
    )
}
fn revert_consumer_features(state: &mut ExpState) -> TweakOpResult {
    revert_hkcu_dwords(
        state,
        "disable-consumer-features",
        &[
            (
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\ContentDeliveryManager",
                "SilentInstalledAppsEnabled",
                1,
            ),
            (
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\ContentDeliveryManager",
                "PreInstalledAppsEnabled",
                1,
            ),
            (
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\ContentDeliveryManager",
                "OemPreInstalledAppsEnabled",
                1,
            ),
        ],
        "Consumer features restored",
    )
}

fn apply_tailored_experiences(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_dword(
        state,
        "disable-tailored-experiences",
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Privacy",
        "TailoredExperiencesWithDiagnosticDataEnabled",
        0,
        1,
        "Tailored experiences disabled",
    )
}
fn revert_tailored_experiences(state: &mut ExpState) -> TweakOpResult {
    revert_hkcu_dword(
        state,
        "disable-tailored-experiences",
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Privacy",
        "TailoredExperiencesWithDiagnosticDataEnabled",
        1,
        "Tailored experiences restored",
    )
}

fn apply_feedback_notifications(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_dword(
        state,
        "disable-feedback-notifications",
        "SOFTWARE\\Microsoft\\Siuf\\Rules",
        "NumberOfSIUFInPeriod",
        0,
        1,
        "Feedback notifications disabled",
    )
}
fn revert_feedback_notifications(state: &mut ExpState) -> TweakOpResult {
    revert_hkcu_dword(
        state,
        "disable-feedback-notifications",
        "SOFTWARE\\Microsoft\\Siuf\\Rules",
        "NumberOfSIUFInPeriod",
        1,
        "Feedback notifications restored",
    )
}

fn apply_telemetry(state: &mut ExpState) -> TweakOpResult {
    apply_hklm_dword(
        state,
        "disable-telemetry",
        "SOFTWARE\\Policies\\Microsoft\\Windows\\DataCollection",
        "AllowTelemetry",
        0,
        3,
        "Telemetry policy set to minimum (Security). Group Policy or restart may be needed.",
    )
}
fn revert_telemetry(state: &mut ExpState) -> TweakOpResult {
    revert_hklm_dword(
        state,
        "disable-telemetry",
        "SOFTWARE\\Policies\\Microsoft\\Windows\\DataCollection",
        "AllowTelemetry",
        3,
        "Telemetry policy removed",
    )
}

fn apply_activity_history(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_dword(
        state,
        "disable-activity-history",
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced",
        "Start_TrackProgs",
        0,
        1,
        "Recent file/program tracking disabled",
    )
}
fn revert_activity_history(state: &mut ExpState) -> TweakOpResult {
    revert_hkcu_dword(
        state,
        "disable-activity-history",
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced",
        "Start_TrackProgs",
        1,
        "Recent file/program tracking restored",
    )
}

// â”€â”€ Network tweaks â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn apply_delivery_optimization(state: &mut ExpState) -> TweakOpResult {
    apply_hklm_dword(state, "disable-delivery-optimization",
        "SOFTWARE\\Policies\\Microsoft\\Windows\\DeliveryOptimization", "DODownloadMode", 0, 3,
        "Delivery Optimization peer-to-peer downloads disabled (HTTP only). Administrator required.")
}
fn revert_delivery_optimization(state: &mut ExpState) -> TweakOpResult {
    // Revert by deleting the policy key entirely
    use winreg::enums::*;
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let _ = hklm.delete_subkey("SOFTWARE\\Policies\\Microsoft\\Windows\\DeliveryOptimization");
    state.applied.remove("disable-delivery-optimization");
    TweakOpResult::ok("Delivery Optimization policy removed â€” default settings restored")
}

fn apply_network_throttling(state: &mut ExpState) -> TweakOpResult {
    apply_hklm_dword(
        state,
        "network-throttling-index",
        "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile",
        "NetworkThrottlingIndex",
        0xFFFFFFFF,
        10,
        "Network throttling disabled (NetworkThrottlingIndex = FFFFFFFF). Administrator required.",
    )
}
fn revert_network_throttling(state: &mut ExpState) -> TweakOpResult {
    revert_hklm_dword(
        state,
        "network-throttling-index",
        "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile",
        "NetworkThrottlingIndex",
        10,
        "Network throttling index restored",
    )
}

fn apply_system_responsiveness(state: &mut ExpState) -> TweakOpResult {
    apply_hklm_dword(
        state,
        "system-responsiveness",
        "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile",
        "SystemResponsiveness",
        0,
        20,
        "SystemResponsiveness set to 0 (prioritize foreground app). Administrator required.",
    )
}
fn revert_system_responsiveness(state: &mut ExpState) -> TweakOpResult {
    revert_hklm_dword(
        state,
        "system-responsiveness",
        "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile",
        "SystemResponsiveness",
        20,
        "SystemResponsiveness restored to default (20)",
    )
}

// â”€â”€ Startup tweaks â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

// OneDrive startup (from v0.5.0 â€” kept verbatim with minor refactor)
fn apply_onedrive_startup(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        KEY_READ | KEY_WRITE,
    ) {
        Ok(k) => k,
        Err(e) => return TweakOpResult::fail("Cannot open Run key", e.to_string()),
    };
    let path: String = match key.get_value("OneDrive") {
        Ok(v) => v,
        Err(_) => {
            return TweakOpResult::fail(
                "OneDrive startup entry not found",
                "OneDrive may not be installed or is already disabled",
            )
        }
    };
    state.onedrive_path = Some(path);
    match key.delete_value("OneDrive") {
        Ok(_) => {
            state
                .applied
                .insert("disable-onedrive-startup".into(), true);
            TweakOpResult::ok("OneDrive startup entry removed")
        }
        Err(e) => TweakOpResult::fail("Failed to remove OneDrive startup entry", e.to_string()),
    }
}

fn revert_onedrive_startup(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let path = match state.onedrive_path.clone() {
        Some(p) => p,
        None => {
            return TweakOpResult::fail(
                "No saved OneDrive path",
                "Disable it via VOptimizer first before reverting",
            )
        }
    };
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        KEY_WRITE,
    ) {
        Ok(k) => k,
        Err(e) => return TweakOpResult::fail("Cannot open Run key", e.to_string()),
    };
    match key.set_value("OneDrive", &path) {
        Ok(_) => {
            state.applied.remove("disable-onedrive-startup");
            TweakOpResult::ok("OneDrive startup entry restored")
        }
        Err(e) => TweakOpResult::fail("Failed to restore OneDrive startup", e.to_string()),
    }
}

fn apply_startup_delay(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_dword(state, "reduce-startup-delay",
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Serialize",
        "StartupDelayInMSec", 0, 10000,
        "Startup delay removed â€” applications in the Startup folder launch immediately after login")
}
fn revert_startup_delay(state: &mut ExpState) -> TweakOpResult {
    // When original key didn't exist, delete rather than restore
    use winreg::enums::*;
    use winreg::RegKey;
    let save_key = "reduce-startup-delay:StartupDelayInMSec";
    if !state.saved_dwords.contains_key(save_key) {
        // Never applied via this tool
        state.applied.remove("reduce-startup-delay");
        return TweakOpResult::ok("Startup delay reverted");
    }
    let original = state.saved_dwords[save_key];
    if original == 10000 {
        // Key likely didn't exist before (we used the default); delete it
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let _ = hkcu
            .open_subkey_with_flags(
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Serialize",
                KEY_WRITE,
            )
            .and_then(|k| k.delete_value("StartupDelayInMSec"));
        state.applied.remove("reduce-startup-delay");
        TweakOpResult::ok("Startup delay reverted to Windows default")
    } else {
        revert_hkcu_dword(
            state,
            "reduce-startup-delay",
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Serialize",
            "StartupDelayInMSec",
            10000,
            "Startup delay restored to previous value",
        )
    }
}

fn apply_edge_startup_boost(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_dword(
        state,
        "disable-edge-startup-boost",
        "SOFTWARE\\Microsoft\\Edge\\Main",
        "StartupBoostEnabled",
        0,
        1,
        "Edge startup boost disabled",
    )
}
fn revert_edge_startup_boost(state: &mut ExpState) -> TweakOpResult {
    revert_hkcu_dword(
        state,
        "disable-edge-startup-boost",
        "SOFTWARE\\Microsoft\\Edge\\Main",
        "StartupBoostEnabled",
        1,
        "Edge startup boost restored",
    )
}

fn apply_edge_background(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_dword(
        state,
        "disable-edge-background-mode",
        "SOFTWARE\\Microsoft\\Edge\\Main",
        "BackgroundModeEnabled",
        0,
        1,
        "Edge background mode disabled",
    )
}
fn revert_edge_background(state: &mut ExpState) -> TweakOpResult {
    revert_hkcu_dword(
        state,
        "disable-edge-background-mode",
        "SOFTWARE\\Microsoft\\Edge\\Main",
        "BackgroundModeEnabled",
        1,
        "Edge background mode restored",
    )
}

// â”€â”€ Interface tweaks â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn apply_file_extensions(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_dword(
        state,
        "show-file-extensions",
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced",
        "HideFileExt",
        0,
        1,
        "File extensions are now visible in Explorer. Restart Explorer to see the change.",
    )
}
fn revert_file_extensions(state: &mut ExpState) -> TweakOpResult {
    revert_hkcu_dword(
        state,
        "show-file-extensions",
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced",
        "HideFileExt",
        1,
        "File extensions hidden again",
    )
}

fn apply_dark_mode(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_dwords(
        state,
        "dark-mode",
        &[
            (
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
                "AppsUseLightTheme",
                0,
                1,
            ),
            (
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
                "SystemUsesLightTheme",
                0,
                1,
            ),
        ],
        "System and app dark mode enabled",
    )
}
fn revert_dark_mode(state: &mut ExpState) -> TweakOpResult {
    revert_hkcu_dwords(
        state,
        "dark-mode",
        &[
            (
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
                "AppsUseLightTheme",
                1,
            ),
            (
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
                "SystemUsesLightTheme",
                1,
            ),
        ],
        "Light mode restored",
    )
}

fn apply_classic_context_menu(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = "SOFTWARE\\Classes\\CLSID\\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}\\InprocServer32";
    match hkcu.create_subkey(path) {
        Ok((key, _)) => match key.set_value("", &"") {
            Ok(_) => {
                state.applied.insert("classic-context-menu".into(), true);
                TweakOpResult::ok(
                    "Classic right-click context menu restored. Restart Explorer to apply.",
                )
            }
            Err(e) => TweakOpResult::fail("Failed to set registry value", e.to_string()),
        },
        Err(e) => TweakOpResult::fail("Failed to create registry key", e.to_string()),
    }
}

fn revert_classic_context_menu(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let _ =
        hkcu.delete_subkey_all("SOFTWARE\\Classes\\CLSID\\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}");
    state.applied.remove("classic-context-menu");
    TweakOpResult::ok("New-style context menu restored. Restart Explorer to apply.")
}

// Widgets (from v0.5.0 â€” kept verbatim with refactor to use helpers)
fn apply_widgets(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced",
        KEY_READ | KEY_WRITE,
    ) {
        Ok(k) => k,
        Err(e) => return TweakOpResult::fail("Cannot open Explorer Advanced key", e.to_string()),
    };
    let original: u32 = key.get_value("TaskbarDa").unwrap_or(1u32);
    state.widgets_original = Some(original);
    match key.set_value("TaskbarDa", &0u32) {
        Ok(_) => {
            state.applied.insert("disable-widgets".into(), true);
            TweakOpResult::ok(
                "Widgets hidden from taskbar. Sign out or restart Explorer to see the change.",
            )
        }
        Err(e) => TweakOpResult::fail("Failed to disable Widgets", e.to_string()),
    }
}

fn revert_widgets(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let original = state.widgets_original.unwrap_or(1u32);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced",
        KEY_WRITE,
    ) {
        Ok(k) => k,
        Err(e) => return TweakOpResult::fail("Cannot open Explorer Advanced key", e.to_string()),
    };
    match key.set_value("TaskbarDa", &original) {
        Ok(_) => {
            state.applied.remove("disable-widgets");
            TweakOpResult::ok(
                "Widgets restored to taskbar. Sign out or restart Explorer to see the change.",
            )
        }
        Err(e) => TweakOpResult::fail("Failed to restore Widgets", e.to_string()),
    }
}

fn apply_transparency(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_dword(
        state,
        "disable-transparency-effects",
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
        "EnableTransparency",
        0,
        1,
        "Transparency effects disabled",
    )
}
fn revert_transparency(state: &mut ExpState) -> TweakOpResult {
    revert_hkcu_dword(
        state,
        "disable-transparency-effects",
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
        "EnableTransparency",
        1,
        "Transparency effects restored",
    )
}

fn apply_animations(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_string(
        state,
        "disable-animations",
        "Control Panel\\Desktop\\WindowMetrics",
        "MinAnimate",
        "0",
        "1",
        "Window minimize/maximize animations disabled. Sign out to apply.",
    )
}
fn revert_animations(state: &mut ExpState) -> TweakOpResult {
    revert_hkcu_string(
        state,
        "disable-animations",
        "Control Panel\\Desktop\\WindowMetrics",
        "MinAnimate",
        "1",
        "Window animations restored",
    )
}

fn apply_visual_effects(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_dword(
        state,
        "disable-visual-effects",
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VisualEffects",
        "VisualFXSetting",
        2,
        0,
        "Visual effects set to Best Performance â€” all effects disabled",
    )
}
fn revert_visual_effects(state: &mut ExpState) -> TweakOpResult {
    revert_hkcu_dword(
        state,
        "disable-visual-effects",
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VisualEffects",
        "VisualFXSetting",
        0,
        "Visual effects restored to Windows default (Let Windows choose)",
    )
}

// â”€â”€ NVIDIA telemetry (from v0.5.0) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn apply_nvidia_telemetry(state: &mut ExpState) -> TweakOpResult {
    let ps = ps_exe();
    let script = concat!(
        "$tasks = Get-ScheduledTask 2>$null | Where-Object {",
        " $_.TaskName -like '*NvTm*' -or",
        " $_.TaskName -like '*NvDriver*' -or",
        " $_.TaskName -like '*NvNode*' };",
        "if ($tasks) {",
        " $tasks | Disable-ScheduledTask | Out-Null;",
        " Write-Output \"Disabled $($tasks.Count) NVIDIA telemetry task(s)\"",
        "} else { Write-Output 'No NVIDIA telemetry tasks found' }"
    );
    match no_window_cmd(&ps)
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output()
    {
        Err(e) => TweakOpResult::fail("Failed to run PowerShell", e.to_string()),
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
            if !stderr.is_empty() && !o.status.success() {
                TweakOpResult::fail("PowerShell returned an error", stderr)
            } else {
                state
                    .applied
                    .insert("disable-nvidia-telemetry".into(), true);
                TweakOpResult::ok(if stdout.is_empty() {
                    "NVIDIA telemetry tasks processed".into()
                } else {
                    stdout
                })
            }
        }
    }
}

fn revert_nvidia_telemetry(state: &mut ExpState) -> TweakOpResult {
    let ps = ps_exe();
    let script = concat!(
        "$tasks = Get-ScheduledTask 2>$null | Where-Object {",
        " $_.TaskName -like '*NvTm*' -or",
        " $_.TaskName -like '*NvDriver*' -or",
        " $_.TaskName -like '*NvNode*' };",
        "if ($tasks) {",
        " $tasks | Enable-ScheduledTask | Out-Null;",
        " Write-Output \"Re-enabled $($tasks.Count) NVIDIA telemetry task(s)\"",
        "} else { Write-Output 'No NVIDIA telemetry tasks found' }"
    );
    match no_window_cmd(&ps)
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output()
    {
        Err(e) => TweakOpResult::fail("Failed to run PowerShell", e.to_string()),
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
            if !stderr.is_empty() && !o.status.success() {
                TweakOpResult::fail("PowerShell returned an error", stderr)
            } else {
                state.applied.remove("disable-nvidia-telemetry");
                TweakOpResult::ok(if stdout.is_empty() {
                    "NVIDIA telemetry tasks re-enabled".into()
                } else {
                    stdout
                })
            }
        }
    }
}

// â”€â”€ Power throttling â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn apply_power_throttling(state: &mut ExpState) -> TweakOpResult {
    apply_hklm_dword(
        state,
        "disable-power-throttling",
        "SYSTEM\\CurrentControlSet\\Control\\Power\\PowerThrottling",
        "PowerThrottlingOff",
        1,
        0,
        "Power throttling (EcoQoS) disabled system-wide. Restart required.",
    )
}
fn revert_power_throttling(state: &mut ExpState) -> TweakOpResult {
    revert_hklm_dword(
        state,
        "disable-power-throttling",
        "SYSTEM\\CurrentControlSet\\Control\\Power\\PowerThrottling",
        "PowerThrottlingOff",
        0,
        "Power throttling restored to Windows default. Restart required.",
    )
}

// â”€â”€ NVIDIA overlay startup â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

const NVIDIA_RUN_NAMES: &[&str] = &[
    "NvTray",
    "NvBackend",
    "NvTmRep_CrashReporter",
    "NVIDIA Settings",
];

fn apply_nvidia_overlay_startup(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let run_path = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run";
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let mut removed = 0usize;
    for &name in NVIDIA_RUN_NAMES {
        if let Ok(key) = hkcu.open_subkey_with_flags(run_path, KEY_READ | KEY_WRITE) {
            if let Ok(val) = key.get_value::<String, _>(name) {
                state
                    .saved_strings
                    .entry(format!("nvidia-overlay:hkcu:{}", name))
                    .or_insert(val);
                let _ = key.delete_value(name);
                removed += 1;
            }
        }
        if let Ok(key) = hklm.open_subkey_with_flags(run_path, KEY_READ | KEY_WRITE) {
            if let Ok(val) = key.get_value::<String, _>(name) {
                state
                    .saved_strings
                    .entry(format!("nvidia-overlay:hklm:{}", name))
                    .or_insert(val);
                let _ = key.delete_value(name);
                removed += 1;
            }
        }
    }
    state
        .applied
        .insert("disable-nvidia-overlay-startup".into(), true);
    if removed > 0 {
        TweakOpResult::ok(format!(
            "Removed {} NVIDIA overlay startup entry/entries",
            removed
        ))
    } else {
        TweakOpResult::ok("No NVIDIA overlay startup entries found â€” may already be clean")
    }
}
fn revert_nvidia_overlay_startup(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let run_path = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run";
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let mut restored = 0usize;
    let entries: Vec<(String, String)> = state
        .saved_strings
        .iter()
        .filter(|(k, _)| k.starts_with("nvidia-overlay:"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    for (save_key, value) in entries {
        let parts: Vec<&str> = save_key.splitn(3, ':').collect();
        if parts.len() < 3 {
            continue;
        }
        let hive_root = if parts[1] == "hkcu" { &hkcu } else { &hklm };
        if let Ok(key) = hive_root.open_subkey_with_flags(run_path, KEY_WRITE) {
            if key.set_value(parts[2], &value).is_ok() {
                restored += 1;
            }
        }
    }
    state.applied.remove("disable-nvidia-overlay-startup");
    TweakOpResult::ok(format!(
        "Restored {} NVIDIA overlay startup entry/entries",
        restored
    ))
}

// â”€â”€ AMD Radeon autostart â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

const AMD_RUN_NAMES: &[&str] = &[
    "AMD Radeon Software",
    "AMDRadeonSettings",
    "RadeonsoftwareElf",
    "AMD Radeon",
];

fn apply_amd_radeon_autostart(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let run_path = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run";
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let mut removed = 0usize;
    for &name in AMD_RUN_NAMES {
        if let Ok(key) = hkcu.open_subkey_with_flags(run_path, KEY_READ | KEY_WRITE) {
            if let Ok(val) = key.get_value::<String, _>(name) {
                state
                    .saved_strings
                    .entry(format!("amd-autostart:hkcu:{}", name))
                    .or_insert(val);
                let _ = key.delete_value(name);
                removed += 1;
            }
        }
        if let Ok(key) = hklm.open_subkey_with_flags(run_path, KEY_READ | KEY_WRITE) {
            if let Ok(val) = key.get_value::<String, _>(name) {
                state
                    .saved_strings
                    .entry(format!("amd-autostart:hklm:{}", name))
                    .or_insert(val);
                let _ = key.delete_value(name);
                removed += 1;
            }
        }
    }
    state
        .applied
        .insert("disable-amd-radeon-autostart".into(), true);
    if removed > 0 {
        TweakOpResult::ok(format!(
            "Removed {} AMD Radeon startup entry/entries",
            removed
        ))
    } else {
        TweakOpResult::ok("No AMD Radeon startup entries found â€” may already be clean")
    }
}
fn revert_amd_radeon_autostart(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let run_path = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run";
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let mut restored = 0usize;
    let entries: Vec<(String, String)> = state
        .saved_strings
        .iter()
        .filter(|(k, _)| k.starts_with("amd-autostart:"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    for (save_key, value) in entries {
        let parts: Vec<&str> = save_key.splitn(3, ':').collect();
        if parts.len() < 3 {
            continue;
        }
        let hive_root = if parts[1] == "hkcu" { &hkcu } else { &hklm };
        if let Ok(key) = hive_root.open_subkey_with_flags(run_path, KEY_WRITE) {
            if key.set_value(parts[2], &value).is_ok() {
                restored += 1;
            }
        }
    }
    state.applied.remove("disable-amd-radeon-autostart");
    TweakOpResult::ok(format!(
        "Restored {} AMD Radeon startup entry/entries",
        restored
    ))
}

// â”€â”€ AMD telemetry â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn apply_amd_telemetry(state: &mut ExpState) -> TweakOpResult {
    let ps = ps_exe();
    let script = concat!(
        "$tasks = Get-ScheduledTask 2>$null | Where-Object {",
        " ($_.TaskName -like '*AMD*Crash*' -or $_.TaskName -like '*AMD*Updater*'",
        " -or $_.TaskName -like '*AMD*Experience*' -or $_.TaskName -like '*AMD*User*'",
        " -or $_.TaskPath -like '*\\AMD\\*') -and $_.State -ne 'Disabled' };",
        "if ($tasks) {",
        " $tasks | Disable-ScheduledTask | Out-Null;",
        " Write-Output \"Disabled $($tasks.Count) AMD scheduled task(s)\"",
        "} else { Write-Output 'No AMD telemetry tasks found or all already disabled' }"
    );
    match no_window_cmd(&ps)
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output()
    {
        Err(e) => TweakOpResult::fail("Failed to run PowerShell", e.to_string()),
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
            if !stderr.is_empty() && !o.status.success() {
                TweakOpResult::fail("PowerShell returned an error", stderr)
            } else {
                state.applied.insert("disable-amd-telemetry".into(), true);
                TweakOpResult::ok(if stdout.is_empty() {
                    "AMD telemetry tasks processed".into()
                } else {
                    stdout
                })
            }
        }
    }
}
fn revert_amd_telemetry(state: &mut ExpState) -> TweakOpResult {
    let ps = ps_exe();
    let script = concat!(
        "$tasks = Get-ScheduledTask 2>$null | Where-Object {",
        " ($_.TaskName -like '*AMD*Crash*' -or $_.TaskName -like '*AMD*Updater*'",
        " -or $_.TaskName -like '*AMD*Experience*' -or $_.TaskName -like '*AMD*User*'",
        " -or $_.TaskPath -like '*\\AMD\\*') -and $_.State -eq 'Disabled' };",
        "if ($tasks) {",
        " $tasks | Enable-ScheduledTask | Out-Null;",
        " Write-Output \"Re-enabled $($tasks.Count) AMD scheduled task(s)\"",
        "} else { Write-Output 'No disabled AMD tasks found' }"
    );
    match no_window_cmd(&ps)
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output()
    {
        Err(e) => TweakOpResult::fail("Failed to run PowerShell", e.to_string()),
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
            state.applied.remove("disable-amd-telemetry");
            TweakOpResult::ok(if stdout.is_empty() {
                "AMD telemetry tasks re-enabled".into()
            } else {
                stdout
            })
        }
    }
}

// â”€â”€ Page file â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn apply_adjust_page_file(state: &mut ExpState) -> TweakOpResult {
    apply_hklm_dword(
        state,
        "adjust-page-file",
        "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Memory Management",
        "AutomaticManagedPagefile",
        1,
        0,
        "Pagefile set to system-managed. A restart is required for the change to take effect.",
    )
}
fn revert_adjust_page_file(state: &mut ExpState) -> TweakOpResult {
    revert_hklm_dword(
        state,
        "adjust-page-file",
        "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Memory Management",
        "AutomaticManagedPagefile",
        0,
        "Pagefile configuration reverted. A restart is required.",
    )
}

// â”€â”€ RAM Standby Cleaner â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn apply_ram_standby_cleaner(state: &mut ExpState) -> TweakOpResult {
    let ps = ps_exe();
    // NtSetSystemInformation(SystemMemoryListInformation=24, EmptyStandbyList=80)
    let script = concat!(
        "Add-Type -MemberDefinition '[DllImport(\"ntdll.dll\")] ",
        "public static extern uint NtSetSystemInformation(int InfoClass, IntPtr Info, int Length);' ",
        "-Name 'NtDll' -Namespace 'Win32';",
        "[IntPtr]$ptr = [System.Runtime.InteropServices.Marshal]::AllocHGlobal(4);",
        "try {",
        " [System.Runtime.InteropServices.Marshal]::WriteInt32($ptr, 0, 80);",
        " $r = [Win32.NtDll]::NtSetSystemInformation(24, $ptr, 4);",
        " if ($r -eq 0) { Write-Output 'Standby list flushed â€” RAM reclaimed from standby state' }",
        " else { Write-Error (\"NtSetSystemInformation returned 0x\" + $r.ToString('X8')) }",
        "} finally { [System.Runtime.InteropServices.Marshal]::FreeHGlobal($ptr) }"
    );
    match no_window_cmd(&ps)
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output()
    {
        Err(e) => TweakOpResult::fail("Failed to run PowerShell", e.to_string()),
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
            if o.status.success() && stderr.is_empty() {
                state.applied.insert("ram-standby-cleaner".into(), true);
                TweakOpResult::ok(if stdout.is_empty() {
                    "Standby list flushed".into()
                } else {
                    stdout
                })
            } else {
                let detail = if !stderr.is_empty() { stderr } else { stdout };
                TweakOpResult::fail(
                    "Failed to flush standby list â€” administrator privileges required",
                    detail,
                )
            }
        }
    }
}
fn revert_ram_standby_cleaner(state: &mut ExpState) -> TweakOpResult {
    // One-shot action â€” nothing persistent to undo
    state.applied.remove("ram-standby-cleaner");
    TweakOpResult::ok("Standby cleaner is a one-shot action â€” no persistent change to revert")
}

// â”€â”€ Suggested Content (Start Menu recommended / app suggestions) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

const CDM_PATH: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\ContentDeliveryManager";

fn apply_suggested_content(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_dwords(
        state,
        "disable-suggested-content",
        &[
            (CDM_PATH, "SubscribedContent-338393Enabled", 0, 1), // App suggestions in Start
            (CDM_PATH, "SubscribedContent-353694Enabled", 0, 1), // Timeline suggestions
            (CDM_PATH, "SubscribedContent-353696Enabled", 0, 1), // Subscription content
            (CDM_PATH, "SubscribedContent-310093Enabled", 0, 1), // Windows Welcome experience
        ],
        "Suggested apps and recommended content in Start Menu disabled",
    )
}
fn revert_suggested_content(state: &mut ExpState) -> TweakOpResult {
    revert_hkcu_dwords(
        state,
        "disable-suggested-content",
        &[
            (CDM_PATH, "SubscribedContent-338393Enabled", 1),
            (CDM_PATH, "SubscribedContent-353694Enabled", 1),
            (CDM_PATH, "SubscribedContent-353696Enabled", 1),
            (CDM_PATH, "SubscribedContent-310093Enabled", 1),
        ],
        "Suggested content in Start Menu restored",
    )
}

// â”€â”€ Nagle's Algorithm â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn apply_disable_nagle(state: &mut ExpState) -> TweakOpResult {
    let ps = ps_exe();
    let script = concat!(
        "$path = 'HKLM:\\SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters\\Interfaces';",
        "$count = 0;",
        "Get-ChildItem $path -ErrorAction SilentlyContinue | ForEach-Object {",
        " Set-ItemProperty -Path $_.PSPath -Name 'TcpAckFrequency' -Value 1 -Type DWord -Force -ErrorAction SilentlyContinue;",
        " Set-ItemProperty -Path $_.PSPath -Name 'TCPNoDelay' -Value 1 -Type DWord -Force -ErrorAction SilentlyContinue;",
        " $count++ };",
        "Write-Output \"Nagle's algorithm disabled on $count network interface(s)\""
    );
    match no_window_cmd(&ps)
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output()
    {
        Err(e) => TweakOpResult::fail("Failed to run PowerShell", e.to_string()),
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if o.status.success() {
                state.applied.insert("disable-nagle".into(), true);
                TweakOpResult::ok(if stdout.is_empty() {
                    "Nagle's algorithm disabled".into()
                } else {
                    stdout
                })
            } else {
                let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
                TweakOpResult::fail(
                    "Failed to disable Nagle's algorithm â€” administrator privileges required",
                    stderr,
                )
            }
        }
    }
}
fn revert_disable_nagle(state: &mut ExpState) -> TweakOpResult {
    let ps = ps_exe();
    let script = concat!(
        "$path = 'HKLM:\\SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters\\Interfaces';",
        "Get-ChildItem $path -ErrorAction SilentlyContinue | ForEach-Object {",
        " Remove-ItemProperty -Path $_.PSPath -Name 'TcpAckFrequency' -ErrorAction SilentlyContinue;",
        " Remove-ItemProperty -Path $_.PSPath -Name 'TCPNoDelay' -ErrorAction SilentlyContinue };",
        "Write-Output \"Nagle's algorithm restored on all interfaces\""
    );
    match no_window_cmd(&ps)
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output()
    {
        Err(e) => TweakOpResult::fail("Failed to run PowerShell", e.to_string()),
        Ok(o) => {
            state.applied.remove("disable-nagle");
            let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
            TweakOpResult::ok(if stdout.is_empty() {
                "Nagle's algorithm restored".into()
            } else {
                stdout
            })
        }
    }
}

// â”€â”€ DNS Cache Size â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

const DNS_PARAMS_PATH: &str = "SYSTEM\\CurrentControlSet\\Services\\Dnscache\\Parameters";

fn apply_dns_cache_size(state: &mut ExpState) -> TweakOpResult {
    // Raise max TTL for cached positive responses to 24 hours (86400s).
    // Windows default is no explicit limit; many ISPs serve TTLs of 300s or less.
    apply_hklm_dword(
        state,
        "dns-cache-size",
        DNS_PARAMS_PATH,
        "MaxCacheEntryTtlLimit",
        86400,
        0,
        "DNS cache TTL limit set to 24 hours â€” frequently visited domains resolve faster",
    )
}
fn revert_dns_cache_size(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    // If original was 0 (key didn't exist), delete rather than write 0
    let save_key = "dns-cache-size:MaxCacheEntryTtlLimit";
    let original = state.saved_dwords.get(save_key).copied().unwrap_or(0);
    if original == 0 {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(key) = hklm.open_subkey_with_flags(DNS_PARAMS_PATH, winreg::enums::KEY_WRITE) {
            let _ = key.delete_value("MaxCacheEntryTtlLimit");
        }
        state.applied.remove("dns-cache-size");
        TweakOpResult::ok("DNS cache TTL limit removed â€” Windows default restored")
    } else {
        revert_hklm_dword(
            state,
            "dns-cache-size",
            DNS_PARAMS_PATH,
            "MaxCacheEntryTtlLimit",
            0,
            "DNS cache TTL limit restored to previous value",
        )
    }
}

// â”€â”€ NVIDIA MSI Interrupt Mode â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

const NVIDIA_VENDOR_PREFIX: &str = "VEN_10DE";
const MSI_SUB_PATH: &str =
    "Device Parameters\\Interrupt Management\\MessageSignaledInterruptProperties";

/// Returns HKLM-relative MSI property paths for every NVIDIA PCI device instance.
fn enumerate_nvidia_msi_paths() -> Vec<String> {
    use winreg::enums::*;
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let pci_root = match hklm.open_subkey("SYSTEM\\CurrentControlSet\\Enum\\PCI") {
        Ok(k) => k,
        Err(_) => return vec![],
    };
    let mut paths = vec![];
    for dev_id in pci_root.enum_keys().filter_map(|r| r.ok()) {
        if !dev_id
            .to_ascii_uppercase()
            .starts_with(NVIDIA_VENDOR_PREFIX)
        {
            continue;
        }
        if let Ok(dev_key) = pci_root.open_subkey(&dev_id) {
            for instance in dev_key.enum_keys().filter_map(|r| r.ok()) {
                paths.push(format!(
                    "SYSTEM\\CurrentControlSet\\Enum\\PCI\\{}\\{}\\{}",
                    dev_id, instance, MSI_SUB_PATH
                ));
            }
        }
    }
    paths
}

fn apply_nvidia_msi_mode(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let paths = enumerate_nvidia_msi_paths();
    if paths.is_empty() {
        return TweakOpResult::fail(
            "No NVIDIA PCI devices found in registry â€” ensure NVIDIA drivers are installed",
            "VEN_10DE not present under SYSTEM\\CurrentControlSet\\Enum\\PCI",
        );
    }
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let mut applied = 0usize;
    let mut skipped = 0usize;
    for path in &paths {
        match hklm.create_subkey(path) {
            Err(_) => {
                skipped += 1;
            }
            Ok((key, _)) => {
                let original: u32 = key.get_value("MSISupported").unwrap_or(0);
                state
                    .saved_dwords
                    .entry(format!("nvidia-msi-mode:{}", path))
                    .or_insert(original);
                if key.set_value("MSISupported", &1u32).is_ok() {
                    applied += 1;
                } else {
                    skipped += 1;
                }
            }
        }
    }
    if applied > 0 {
        state.applied.insert("nvidia-msi-mode".into(), true);
        let msg = if skipped > 0 {
            format!(
                "MSI interrupts enabled on {}/{} NVIDIA device(s). {} skipped (access denied). Restart required.",
                applied, paths.len(), skipped
            )
        } else {
            format!(
                "MSI interrupts enabled on {} NVIDIA device(s). Restart required.",
                applied
            )
        };
        TweakOpResult::ok(msg)
    } else {
        TweakOpResult::fail(
            "Could not write NVIDIA MSI registry entries â€” run VOptimizer as administrator",
            "All device paths returned access denied",
        )
    }
}

fn revert_nvidia_msi_mode(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    const PREFIX: &str = "nvidia-msi-mode:";
    let entries: Vec<(String, u32)> = state
        .saved_dwords
        .iter()
        .filter(|(k, _)| k.starts_with(PREFIX))
        .map(|(k, &v)| (k[PREFIX.len()..].to_string(), v))
        .collect();
    if entries.is_empty() {
        state.applied.remove("nvidia-msi-mode");
        return TweakOpResult::ok("No NVIDIA MSI settings saved to revert");
    }
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let mut restored = 0usize;
    for (path, original) in &entries {
        if let Ok((key, _)) = hklm.create_subkey(path) {
            if key.set_value("MSISupported", original).is_ok() {
                restored += 1;
            }
        }
    }
    // Clean up saved keys regardless of success
    state.saved_dwords.retain(|k, _| !k.starts_with(PREFIX));
    state.applied.remove("nvidia-msi-mode");
    TweakOpResult::ok(format!(
        "MSI interrupt mode reverted on {}/{} NVIDIA device(s). Restart required.",
        restored,
        entries.len()
    ))
}

// â”€â”€ Classic Alt+Tab â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn apply_alt_tab(state: &mut ExpState) -> TweakOpResult {
    apply_hkcu_dword(
        state,
        "classic-alt-tab",
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer",
        "AltTabSettings",
        1,
        0,
        "Classic Alt+Tab style enabled. Restart Explorer or sign out to see the change.",
    )
}
fn revert_alt_tab(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let save_key = "classic-alt-tab:AltTabSettings";
    // If original value wasn't set before we wrote it, delete the key rather than write 0
    let original = state.saved_dwords.get(save_key).copied().unwrap_or(0);
    if original == 0 && !state.saved_dwords.contains_key(save_key) {
        // Key was never present; remove to restore true default behaviour
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok((key, _)) =
            hkcu.create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer")
        {
            let _ = key.delete_value("AltTabSettings");
        }
        state.applied.remove("classic-alt-tab");
        return TweakOpResult::ok("Alt+Tab reverted to modern Windows style");
    }
    revert_hkcu_dword(
        state,
        "classic-alt-tab",
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer",
        "AltTabSettings",
        0,
        "Alt+Tab style restored to previous setting",
    )
}

// â”€â”€ Per-exe Fullscreen Optimizations â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

const APP_COMPAT_LAYERS: &str =
    "Software\\Microsoft\\Windows NT\\CurrentVersion\\AppCompatFlags\\Layers";

/// Open a native Windows file-picker via PowerShell + WinForms.
/// Returns `None` if the user cancelled or PowerShell failed.
pub fn pick_exe_file_pub() -> Option<String> {
    let ps = ps_exe();
    let script = concat!(
        "Add-Type -AssemblyName System.Windows.Forms;",
        "$dlg = New-Object System.Windows.Forms.OpenFileDialog;",
        "$dlg.Filter = 'Executable files (*.exe)|*.exe|All files (*.*)|*.*';",
        "$dlg.Title = 'Select game or application executable';",
        "$dlg.Multiselect = $false;",
        // TopMost so the dialog isn't buried behind the app window
        "$dlg.TopMost = $true;",
        "if ($dlg.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {",
        " Write-Output $dlg.FileName",
        "}"
    );
    let out = no_window_cmd(&ps)
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output()
        .ok()?;
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}

fn apply_exe_fsopt_impl(state: &mut ExpState, exe_path: &str) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = match hkcu.create_subkey(APP_COMPAT_LAYERS) {
        Ok(k) => k,
        Err(e) => {
            return TweakOpResult::fail("Cannot open AppCompatFlags registry key", e.to_string())
        }
    };
    // Read existing compatibility flags for this exe (may not exist)
    let existing: String = key.get_value(exe_path).unwrap_or_default();
    // Persist original value and the path so we can revert later
    if let Some(saved_path) = state.saved_strings.get("exe-fsopt:path") {
        if saved_path != exe_path {
            return TweakOpResult::fail(
                "A different executable already has saved fullscreen optimization state",
                "Revert the currently selected exe before applying this tweak to another file",
            );
        }
    } else {
        state
            .saved_strings
            .insert("exe-fsopt:path".into(), exe_path.to_string());
    }
    state
        .saved_strings
        .entry("exe-fsopt:original".into())
        .or_insert(existing.clone());
    const FLAG: &str = "DISABLEDXMAXIMIZEDWINDOWEDMODE";
    let new_value = if existing.is_empty() {
        format!("~ {}", FLAG)
    } else if existing.contains(FLAG) {
        existing // already set â€” treat as success
    } else {
        format!("{} {}", existing, FLAG)
    };
    match key.set_value(exe_path, &new_value) {
        Ok(_) => {
            state
                .applied
                .insert("disable-fullscreen-optimizations-selected-exe".into(), true);
            let exe_name = std::path::Path::new(exe_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(exe_path);
            TweakOpResult::ok(format!(
                "Fullscreen optimizations disabled for {}. Restart the game to take effect.",
                exe_name
            ))
        }
        Err(e) => TweakOpResult::fail(
            "Failed to write compatibility flags to registry",
            e.to_string(),
        ),
    }
}

/// Public wrapper called from lib.rs â€” loads/saves state internally.
pub fn apply_exe_fsopt_pub(exe_path: &str) -> TweakOpResult {
    let mut state = load_state();
    let result = apply_exe_fsopt_impl(&mut state, exe_path);
    save_state(&state);
    result
}

fn revert_exe_fsopt_impl(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    const TWEAK_ID: &str = "disable-fullscreen-optimizations-selected-exe";
    let exe_path = match state.saved_strings.get("exe-fsopt:path").cloned() {
        Some(p) => p,
        None => {
            state.applied.remove(TWEAK_ID);
            return TweakOpResult::ok("No per-exe fullscreen optimization to revert");
        }
    };
    let original = state
        .saved_strings
        .get("exe-fsopt:original")
        .cloned()
        .unwrap_or_default();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = match hkcu.create_subkey(APP_COMPAT_LAYERS) {
        Ok(k) => k,
        Err(e) => {
            return TweakOpResult::fail("Cannot open AppCompatFlags registry key", e.to_string())
        }
    };
    let result = if original.is_empty() {
        // The value didn't exist before â€” delete it to restore default
        let _ = key.delete_value(&exe_path);
        TweakOpResult::ok("Fullscreen optimizations restored (compatibility entry removed)")
    } else {
        match key.set_value(&exe_path, &original) {
            Ok(_) => TweakOpResult::ok("Fullscreen optimizations restored to previous flags"),
            Err(e) => TweakOpResult::fail("Failed to restore compatibility flags", e.to_string()),
        }
    };
    if result.success {
        state.applied.remove(TWEAK_ID);
        state.saved_strings.remove("exe-fsopt:path");
        state.saved_strings.remove("exe-fsopt:original");
    }
    result
}

// ── Keep Kernel in RAM (DisablePagingExecutive) ──────────────────────────────
// Adapted from Fortnite-Optimizer (verifyOptimizations referenced this value).
// Keeps the kernel + drivers resident in RAM instead of paging them to disk.
// Safe on systems with ample RAM; mild benefit. Fully reversible.

const MEM_MGMT_PATH: &str =
    "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Memory Management";

fn apply_keep_kernel_in_ram(state: &mut ExpState) -> TweakOpResult {
    apply_hklm_dword(
        state,
        "keep-kernel-in-ram",
        MEM_MGMT_PATH,
        "DisablePagingExecutive",
        1,
        0,
        "Kernel and drivers kept resident in RAM (DisablePagingExecutive=1). Restart to apply.",
    )
}

fn revert_keep_kernel_in_ram(state: &mut ExpState) -> TweakOpResult {
    revert_hklm_dword(
        state,
        "keep-kernel-in-ram",
        MEM_MGMT_PATH,
        "DisablePagingExecutive",
        0,
        "Kernel paging restored to Windows default (DisablePagingExecutive=0). Restart to apply.",
    )
}

// ── Disable CPU Core Parking (powercfg) ──────────────────────────────────────
// Adapted from Fortnite-Optimizer's optimizeCPUParking().
// Keeps all CPU cores unparked for more consistent frame times.
//
// The repo only ever SET values (powercfg has no `getacvalueindex` subcommand —
// reading must come from the registry). We read the original per-scheme override
// from the registry so revert is exact: if the user had no explicit override we
// delete ours to return to the plan default; otherwise we restore their value.

const PWR_SUBGROUP_PROCESSOR: &str = "54533251-82be-4824-96c1-47b60b740d00";
const PWR_SETTING_CPMINCORES: &str = "0cc5b647-c1df-4637-891a-dec35c318583";
const PWR_SETTING_CPMAXCORES: &str = "ea062031-0e34-4ff1-9b6d-eb1059334028";
// USB settings subgroup + "USB selective suspend setting"
const PWR_SUBGROUP_USB: &str = "2a737441-1930-4402-8d77-b2bebba308a3";
const PWR_SETTING_USB_SUSPEND: &str = "48e6b7a6-50f5-4782-a5d4-53bb8f07e226";

fn power_setting_reg_path(scheme: &str, subgroup: &str, setting: &str) -> String {
    format!(
        "SYSTEM\\CurrentControlSet\\Control\\Power\\User\\PowerSchemes\\{}\\{}\\{}",
        scheme, subgroup, setting
    )
}

/// Reads the AC override index from the registry, or None if no override is set
/// (the setting is at the plan default).
fn read_ac_setting_index(scheme: &str, subgroup: &str, setting: &str) -> Option<u32> {
    use winreg::enums::*;
    use winreg::RegKey;
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(power_setting_reg_path(scheme, subgroup, setting))
        .ok()
        .and_then(|k| k.get_value::<u32, _>("ACSettingIndex").ok())
}

/// Deletes our AC override so the setting reverts to the plan default.
fn delete_ac_override(scheme: &str, subgroup: &str, setting: &str) {
    use winreg::enums::*;
    use winreg::RegKey;
    if let Ok(k) = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(power_setting_reg_path(scheme, subgroup, setting), KEY_ALL_ACCESS)
    {
        let _ = k.delete_value("ACSettingIndex");
    }
}

/// Sets a power-setting AC index via powercfg using explicit GUIDs
/// (many of these setting aliases are hidden by default, so we avoid them).
fn powercfg_set_ac_index(subgroup: &str, setting: &str, value: u32) -> bool {
    no_window_cmd(powercfg_exe())
        .args([
            "/setacvalueindex",
            "SCHEME_CURRENT",
            subgroup,
            setting,
            &value.to_string(),
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn powercfg_activate_current() -> bool {
    no_window_cmd(powercfg_exe())
        .args(["/setactive", "SCHEME_CURRENT"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn apply_core_parking(state: &mut ExpState) -> TweakOpResult {
    let scheme = match get_active_power_scheme() {
        Some(s) => s,
        None => {
            return TweakOpResult::fail(
                "Could not read the active power scheme",
                "powercfg /getactivescheme failed",
            )
        }
    };
    state
        .saved_strings
        .entry("disable-core-parking:scheme".into())
        .or_insert(scheme.clone());
    // Record whether an explicit override existed so revert can be exact.
    match read_ac_setting_index(&scheme, PWR_SUBGROUP_PROCESSOR, PWR_SETTING_CPMINCORES) {
        Some(v) => {
            state
                .saved_dwords
                .entry("disable-core-parking:CPMINCORES".into())
                .or_insert(v);
            state
                .saved_strings
                .entry("disable-core-parking:existed".into())
                .or_insert("1".into());
        }
        None => {
            state
                .saved_strings
                .entry("disable-core-parking:existed".into())
                .or_insert("0".into());
        }
    }

    let ok_min = powercfg_set_ac_index(PWR_SUBGROUP_PROCESSOR, PWR_SETTING_CPMINCORES, 100);
    // CPMAXCORES = 100 (all cores available) matches the default; set for safety.
    let _ = powercfg_set_ac_index(PWR_SUBGROUP_PROCESSOR, PWR_SETTING_CPMAXCORES, 100);
    let ok_active = powercfg_activate_current();
    if ok_min && ok_active {
        state.applied.insert("disable-core-parking".into(), true);
        TweakOpResult::ok("CPU core parking disabled — all cores kept unparked")
    } else {
        TweakOpResult::fail(
            "Failed to disable core parking — run VOptimizer as administrator",
            "powercfg /setacvalueindex failed",
        )
    }
}

fn revert_core_parking(state: &mut ExpState) -> TweakOpResult {
    let scheme = state
        .saved_strings
        .get("disable-core-parking:scheme")
        .cloned()
        .or_else(get_active_power_scheme)
        .unwrap_or_default();
    let existed = state
        .saved_strings
        .get("disable-core-parking:existed")
        .map(|s| s == "1")
        .unwrap_or(false);

    if existed {
        // Restore the user's explicit override exactly.
        let orig = state
            .saved_dwords
            .get("disable-core-parking:CPMINCORES")
            .copied()
            .unwrap_or(100);
        powercfg_set_ac_index(PWR_SUBGROUP_PROCESSOR, PWR_SETTING_CPMINCORES, orig);
    } else if !scheme.is_empty() {
        // No override existed — remove ours to return to the plan default.
        delete_ac_override(&scheme, PWR_SUBGROUP_PROCESSOR, PWR_SETTING_CPMINCORES);
        delete_ac_override(&scheme, PWR_SUBGROUP_PROCESSOR, PWR_SETTING_CPMAXCORES);
    }
    let ok_active = powercfg_activate_current();

    state.applied.remove("disable-core-parking");
    state.saved_dwords.remove("disable-core-parking:CPMINCORES");
    state.saved_strings.remove("disable-core-parking:scheme");
    state.saved_strings.remove("disable-core-parking:existed");

    if ok_active {
        TweakOpResult::ok("CPU core parking restored to previous setting")
    } else {
        TweakOpResult::fail(
            "Failed to reactivate the power scheme — run VOptimizer as administrator",
            "powercfg /setactive failed",
        )
    }
}

// ── Disable USB Selective Suspend (powercfg) ─────────────────────────────────
// Adapted from ToX. Stops Windows power-gating USB devices (mice/keyboards/
// controllers), preventing brief input dropouts. Reversible — restores the
// user's prior override, or removes ours to return to the plan default (1).

fn apply_usb_suspend(state: &mut ExpState) -> TweakOpResult {
    let scheme = match get_active_power_scheme() {
        Some(s) => s,
        None => {
            return TweakOpResult::fail(
                "Could not read the active power scheme",
                "powercfg /getactivescheme failed",
            )
        }
    };
    state
        .saved_strings
        .entry("disable-usb-selective-suspend:scheme".into())
        .or_insert(scheme.clone());
    match read_ac_setting_index(&scheme, PWR_SUBGROUP_USB, PWR_SETTING_USB_SUSPEND) {
        Some(v) => {
            state
                .saved_dwords
                .entry("disable-usb-selective-suspend:val".into())
                .or_insert(v);
            state
                .saved_strings
                .entry("disable-usb-selective-suspend:existed".into())
                .or_insert("1".into());
        }
        None => {
            state
                .saved_strings
                .entry("disable-usb-selective-suspend:existed".into())
                .or_insert("0".into());
        }
    }
    // 0 = Disabled (the goal). Default is 1 = Enabled.
    let ok = powercfg_set_ac_index(PWR_SUBGROUP_USB, PWR_SETTING_USB_SUSPEND, 0);
    let ok_active = powercfg_activate_current();
    if ok && ok_active {
        state
            .applied
            .insert("disable-usb-selective-suspend".into(), true);
        TweakOpResult::ok("USB selective suspend disabled — USB devices stay powered")
    } else {
        TweakOpResult::fail(
            "Failed to disable USB selective suspend — run VOptimizer as administrator",
            "powercfg /setacvalueindex failed",
        )
    }
}

fn revert_usb_suspend(state: &mut ExpState) -> TweakOpResult {
    let scheme = state
        .saved_strings
        .get("disable-usb-selective-suspend:scheme")
        .cloned()
        .or_else(get_active_power_scheme)
        .unwrap_or_default();
    let existed = state
        .saved_strings
        .get("disable-usb-selective-suspend:existed")
        .map(|s| s == "1")
        .unwrap_or(false);
    if existed {
        let orig = state
            .saved_dwords
            .get("disable-usb-selective-suspend:val")
            .copied()
            .unwrap_or(1);
        powercfg_set_ac_index(PWR_SUBGROUP_USB, PWR_SETTING_USB_SUSPEND, orig);
    } else if !scheme.is_empty() {
        delete_ac_override(&scheme, PWR_SUBGROUP_USB, PWR_SETTING_USB_SUSPEND);
    }
    let ok_active = powercfg_activate_current();
    state.applied.remove("disable-usb-selective-suspend");
    state.saved_dwords.remove("disable-usb-selective-suspend:val");
    state
        .saved_strings
        .remove("disable-usb-selective-suspend:scheme");
    state
        .saved_strings
        .remove("disable-usb-selective-suspend:existed");
    if ok_active {
        TweakOpResult::ok("USB selective suspend restored to previous setting")
    } else {
        TweakOpResult::fail(
            "Failed to reactivate the power scheme — run VOptimizer as administrator",
            "powercfg /setactive failed",
        )
    }
}

// ── Disable Hibernate (powercfg /hibernate off) ──────────────────────────────
// Adapted from ToX. Frees a RAM-sized hiberfil.sys and disables hibernate /
// Fast Startup. Reversible with /hibernate on.

fn apply_disable_hibernate(state: &mut ExpState) -> TweakOpResult {
    match no_window_cmd(powercfg_exe())
        .args(["/hibernate", "off"])
        .output()
    {
        Ok(o) if o.status.success() => {
            state.applied.insert("disable-hibernate".into(), true);
            TweakOpResult::ok("Hibernate disabled — hiberfil.sys freed, Fast Startup off")
        }
        Ok(_) => TweakOpResult::fail(
            "Failed to disable hibernate — run VOptimizer as administrator",
            "powercfg /hibernate off failed",
        ),
        Err(e) => TweakOpResult::fail("Could not run powercfg.exe", e.to_string()),
    }
}

fn revert_disable_hibernate(state: &mut ExpState) -> TweakOpResult {
    match no_window_cmd(powercfg_exe())
        .args(["/hibernate", "on"])
        .output()
    {
        Ok(o) if o.status.success() => {
            state.applied.remove("disable-hibernate");
            TweakOpResult::ok("Hibernate re-enabled")
        }
        Ok(_) => TweakOpResult::fail(
            "Failed to re-enable hibernate — run VOptimizer as administrator",
            "powercfg /hibernate on failed",
        ),
        Err(e) => TweakOpResult::fail("Could not run powercfg.exe", e.to_string()),
    }
}

// ── Disable Mouse Acceleration (HKCU Control Panel\Mouse) ─────────────────────
// Adapted from ToX (implemented correctly here). Sets pointer-precision off for
// raw 1:1 mouse input. Defaults restored on revert (1 / 6 / 10).

const MOUSE_PATH: &str = "Control Panel\\Mouse";

fn apply_mouse_accel(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = match hkcu.create_subkey(MOUSE_PATH) {
        Ok(k) => k,
        Err(e) => return TweakOpResult::fail("Cannot open Mouse registry key", e.to_string()),
    };
    // Save originals (REG_SZ), defaulting to Windows defaults if absent.
    for (name, default) in [
        ("MouseSpeed", "1"),
        ("MouseThreshold1", "6"),
        ("MouseThreshold2", "10"),
    ] {
        let orig: String = key.get_value(name).unwrap_or_else(|_| default.to_string());
        state
            .saved_strings
            .entry(format!("disable-mouse-accel:{}", name))
            .or_insert(orig);
        if let Err(e) = key.set_value(name, &"0") {
            return TweakOpResult::fail("Failed to write Mouse registry value", e.to_string());
        }
    }
    state.applied.insert("disable-mouse-accel".into(), true);
    TweakOpResult::ok("Mouse acceleration disabled — sign out and back in to apply")
}

fn revert_mouse_accel(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = match hkcu.create_subkey(MOUSE_PATH) {
        Ok(k) => k,
        Err(e) => return TweakOpResult::fail("Cannot open Mouse registry key", e.to_string()),
    };
    for (name, default) in [
        ("MouseSpeed", "1"),
        ("MouseThreshold1", "6"),
        ("MouseThreshold2", "10"),
    ] {
        let save_key = format!("disable-mouse-accel:{}", name);
        let orig = state
            .saved_strings
            .get(&save_key)
            .cloned()
            .unwrap_or_else(|| default.to_string());
        let _ = key.set_value(name, &orig.as_str());
        state.saved_strings.remove(&save_key);
    }
    state.applied.remove("disable-mouse-accel");
    TweakOpResult::ok("Mouse acceleration restored — sign out and back in to apply")
}

// ── Fastest Keyboard Repeat (HKCU Control Panel\Keyboard) ─────────────────────
// Adapted from ToX. Shortest repeat delay + fastest repeat rate.

const KEYBOARD_PATH: &str = "Control Panel\\Keyboard";

fn apply_fast_keyboard(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = match hkcu.create_subkey(KEYBOARD_PATH) {
        Ok(k) => k,
        Err(e) => return TweakOpResult::fail("Cannot open Keyboard registry key", e.to_string()),
    };
    // KeyboardDelay default "1", KeyboardSpeed default "31".
    for (name, default, new) in [("KeyboardDelay", "1", "0"), ("KeyboardSpeed", "31", "31")] {
        let orig: String = key.get_value(name).unwrap_or_else(|_| default.to_string());
        state
            .saved_strings
            .entry(format!("fast-keyboard-repeat:{}", name))
            .or_insert(orig);
        if let Err(e) = key.set_value(name, &new) {
            return TweakOpResult::fail("Failed to write Keyboard registry value", e.to_string());
        }
    }
    state.applied.insert("fast-keyboard-repeat".into(), true);
    TweakOpResult::ok("Keyboard repeat set to fastest — sign out and back in to apply")
}

fn revert_fast_keyboard(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = match hkcu.create_subkey(KEYBOARD_PATH) {
        Ok(k) => k,
        Err(e) => return TweakOpResult::fail("Cannot open Keyboard registry key", e.to_string()),
    };
    for (name, default) in [("KeyboardDelay", "1"), ("KeyboardSpeed", "31")] {
        let save_key = format!("fast-keyboard-repeat:{}", name);
        let orig = state
            .saved_strings
            .get(&save_key)
            .cloned()
            .unwrap_or_else(|| default.to_string());
        let _ = key.set_value(name, &orig.as_str());
        state.saved_strings.remove(&save_key);
    }
    state.applied.remove("fast-keyboard-repeat");
    TweakOpResult::ok("Keyboard repeat restored — sign out and back in to apply")
}

// ── Foreground Boost (Win32PrioritySeparation) ───────────────────────────────
// Adapted from ToX. Tunes the scheduler quantum to favour the foreground app.
// Unproven on modern multi-core CPUs but harmless and reversible (default 2).

const PRIORITY_CONTROL_PATH: &str = "SYSTEM\\CurrentControlSet\\Control\\PriorityControl";

fn apply_foreground_boost(state: &mut ExpState) -> TweakOpResult {
    apply_hklm_dword(
        state,
        "foreground-boost",
        PRIORITY_CONTROL_PATH,
        "Win32PrioritySeparation",
        0x1A, // short, variable quantums with a 3x foreground boost
        2,
        "Foreground boost enabled (Win32PrioritySeparation = 0x1A)",
    )
}

fn revert_foreground_boost(state: &mut ExpState) -> TweakOpResult {
    revert_hklm_dword(
        state,
        "foreground-boost",
        PRIORITY_CONTROL_PATH,
        "Win32PrioritySeparation",
        2,
        "Foreground boost reverted to Windows default (Win32PrioritySeparation = 2)",
    )
}

// ── Disable Network Adapter Power Saving (per-adapter PnPCapabilities) ────────
// Adapted from ToX's Disable-NetAdapterPowerManagement, implemented as a
// reversible registry edit: PnPCapabilities = 24 (0x18) disables "Allow the
// computer to turn off this device". Original values are saved per adapter.

const NET_CLASS_PATH: &str =
    "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e972-e325-11ce-bfc1-08002be10318}";

/// Returns full registry paths of PHYSICAL network adapters only. A physical
/// adapter's ComponentId starts with "PCI\" or "USB\"; this filters out WAN
/// miniports (ms_*), the kernel-debug adapter (root\kdnic), and other virtual
/// adapters that have a NetCfgInstanceId but no real power management.
fn enumerate_net_adapter_keys() -> Vec<String> {
    use winreg::enums::*;
    use winreg::RegKey;
    let mut out = vec![];
    if let Ok(class_key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(NET_CLASS_PATH) {
        for sub in class_key.enum_keys().flatten() {
            if sub.len() == 4 && sub.chars().all(|c| c.is_ascii_digit()) {
                let full = format!("{}\\{}", NET_CLASS_PATH, sub);
                if let Ok(k) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(&full) {
                    if k.get_value::<String, _>("NetCfgInstanceId").is_err() {
                        continue;
                    }
                    let component: String =
                        k.get_value("ComponentId").unwrap_or_default();
                    let c = component.to_ascii_lowercase();
                    if c.starts_with("pci\\") || c.starts_with("usb\\") {
                        out.push(full);
                    }
                }
            }
        }
    }
    out
}

fn apply_nic_power_saving(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    const ID: &str = "disable-nic-power-saving";
    let keys = enumerate_net_adapter_keys();
    if keys.is_empty() {
        return TweakOpResult::fail(
            "No network adapters found in the registry",
            "net class enumeration returned nothing",
        );
    }
    let mut count = 0u32;
    let mut last_err = String::new();
    for (i, path) in keys.iter().enumerate() {
        match RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(path, KEY_ALL_ACCESS) {
            Ok(k) => {
                let orig = k.get_value::<u32, _>("PnPCapabilities").ok();
                state
                    .saved_strings
                    .entry(format!("{}:path:{}", ID, i))
                    .or_insert(path.clone());
                match orig {
                    Some(v) => {
                        state
                            .saved_dwords
                            .entry(format!("{}:orig:{}", ID, i))
                            .or_insert(v);
                        state
                            .saved_strings
                            .entry(format!("{}:existed:{}", ID, i))
                            .or_insert("1".into());
                    }
                    None => {
                        state
                            .saved_strings
                            .entry(format!("{}:existed:{}", ID, i))
                            .or_insert("0".into());
                    }
                }
                // 0x18 (24) = disable "allow the computer to turn off this device"
                if k.set_value("PnPCapabilities", &24u32).is_ok() {
                    count += 1;
                }
            }
            Err(e) => last_err = e.to_string(),
        }
    }
    if count > 0 {
        state
            .saved_dwords
            .insert(format!("{}:count", ID), keys.len() as u32);
        state.applied.insert(ID.into(), true);
        TweakOpResult::ok(format!(
            "Power saving disabled on {} network adapter(s) — restart to fully apply",
            count
        ))
    } else {
        TweakOpResult::fail(
            "Failed to disable NIC power saving — run VOptimizer as administrator",
            last_err,
        )
    }
}

fn revert_nic_power_saving(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    const ID: &str = "disable-nic-power-saving";
    let count = state
        .saved_dwords
        .get(&format!("{}:count", ID))
        .copied()
        .unwrap_or(0);
    for i in 0..count {
        if let Some(path) = state.saved_strings.get(&format!("{}:path:{}", ID, i)).cloned() {
            let existed = state
                .saved_strings
                .get(&format!("{}:existed:{}", ID, i))
                .map(|s| s == "1")
                .unwrap_or(false);
            if let Ok(k) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(&path, KEY_ALL_ACCESS)
            {
                if existed {
                    let orig = state
                        .saved_dwords
                        .get(&format!("{}:orig:{}", ID, i))
                        .copied()
                        .unwrap_or(0);
                    let _ = k.set_value("PnPCapabilities", &orig);
                } else {
                    let _ = k.delete_value("PnPCapabilities");
                }
            }
        }
    }
    // Remove all saved entries for this tweak
    let prefix = format!("{}:", ID);
    state.saved_strings.retain(|k, _| !k.starts_with(&prefix));
    state.saved_dwords.retain(|k, _| !k.starts_with(&prefix));
    state.applied.remove(ID);
    TweakOpResult::ok("Network adapter power saving restored to previous settings")
}

// ── Disable Large Send Offload (PowerShell, unproven) ────────────────────────
// Adapted from ToX's Disable-NetAdapterLso. Reversible via Enable-NetAdapterLso.

fn apply_disable_lso(state: &mut ExpState) -> TweakOpResult {
    let ps = ps_exe();
    let script = "Disable-NetAdapterLso -Name '*' -ErrorAction SilentlyContinue; Write-Output 'ok'";
    match no_window_cmd(&ps)
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output()
    {
        Ok(o) if o.status.success() => {
            state.applied.insert("disable-lso".into(), true);
            TweakOpResult::ok("Large Send Offload disabled on all adapters")
        }
        Ok(o) => TweakOpResult::fail(
            "Failed to disable LSO — run VOptimizer as administrator",
            String::from_utf8_lossy(&o.stderr).trim().to_string(),
        ),
        Err(e) => TweakOpResult::fail("Failed to run PowerShell", e.to_string()),
    }
}

fn revert_disable_lso(state: &mut ExpState) -> TweakOpResult {
    let ps = ps_exe();
    let script = "Enable-NetAdapterLso -Name '*' -ErrorAction SilentlyContinue; Write-Output 'ok'";
    match no_window_cmd(&ps)
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output()
    {
        Ok(o) if o.status.success() => {
            state.applied.remove("disable-lso");
            TweakOpResult::ok("Large Send Offload re-enabled on all adapters")
        }
        Ok(o) => TweakOpResult::fail(
            "Failed to re-enable LSO — run VOptimizer as administrator",
            String::from_utf8_lossy(&o.stderr).trim().to_string(),
        ),
        Err(e) => TweakOpResult::fail("Failed to run PowerShell", e.to_string()),
    }
}

// ── Per-exe: Set Game Process Priority (IFEO PerfOptions) ─────────────────────
// Adapted from Fortnite-Optimizer's optimizeProcessPriority(), generalized to
// any user-chosen exe (instead of hardcoding FortniteClient-Win64-Shipping.exe).
// Uses High (3), never Realtime (4) — realtime can starve the OS.

const IFEO_PATH: &str =
    "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Image File Execution Options";

fn apply_exe_priority_impl(state: &mut ExpState, exe_path: &str) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    const ID: &str = "set-game-priority-selected-exe";
    // IFEO matches on the bare file name, not the full path.
    let exe_name = std::path::Path::new(exe_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(exe_path)
        .to_string();

    if let Some(saved) = state.saved_strings.get("exe-priority:exe") {
        if saved != &exe_name {
            return TweakOpResult::fail(
                "A different executable already has saved priority state",
                "Revert the current one before applying this tweak to another file",
            );
        }
    }

    let perf_path = format!("{}\\{}\\PerfOptions", IFEO_PATH, exe_name);
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let (key, _) = match hklm.create_subkey(&perf_path) {
        Ok(k) => k,
        Err(e) => {
            return TweakOpResult::fail(
                "Cannot open IFEO registry key — administrator privileges required",
                e.to_string(),
            )
        }
    };

    let existed = key.get_value::<u32, _>("CpuPriorityClass").ok();
    state
        .saved_strings
        .entry("exe-priority:path".into())
        .or_insert(exe_path.to_string());
    state
        .saved_strings
        .entry("exe-priority:exe".into())
        .or_insert(exe_name.clone());
    match existed {
        Some(o) => {
            state
                .saved_dwords
                .entry("exe-priority:CpuPriorityClass".into())
                .or_insert(o);
            state
                .saved_strings
                .entry("exe-priority:existed".into())
                .or_insert("1".into());
        }
        None => {
            state
                .saved_strings
                .entry("exe-priority:existed".into())
                .or_insert("0".into());
        }
    }

    // CpuPriorityClass = 3 (High). IoPriority = 3 (High), PagePriority = 5 (High).
    if let Err(e) = key.set_value("CpuPriorityClass", &3u32) {
        return TweakOpResult::fail("Failed to write process priority", e.to_string());
    }
    let _ = key.set_value("IoPriority", &3u32);
    let _ = key.set_value("PagePriority", &5u32);

    state.applied.insert(ID.into(), true);
    TweakOpResult::ok(format!(
        "High priority set for {} — restart the game to take effect.",
        exe_name
    ))
}

fn revert_exe_priority_impl(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    const ID: &str = "set-game-priority-selected-exe";
    let exe_name = match state.saved_strings.get("exe-priority:exe").cloned() {
        Some(n) => n,
        None => {
            state.applied.remove(ID);
            return TweakOpResult::ok("No per-exe priority to revert");
        }
    };
    let existed = state
        .saved_strings
        .get("exe-priority:existed")
        .map(|s| s == "1")
        .unwrap_or(false);
    let perf_path = format!("{}\\{}\\PerfOptions", IFEO_PATH, exe_name);
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    if let Ok(key) = hklm.open_subkey_with_flags(&perf_path, KEY_ALL_ACCESS) {
        if existed {
            let orig = state
                .saved_dwords
                .get("exe-priority:CpuPriorityClass")
                .copied()
                .unwrap_or(2);
            let _ = key.set_value("CpuPriorityClass", &orig);
            let _ = key.delete_value("IoPriority");
            let _ = key.delete_value("PagePriority");
        } else {
            let _ = key.delete_value("CpuPriorityClass");
            let _ = key.delete_value("IoPriority");
            let _ = key.delete_value("PagePriority");
        }
    }
    // If we created the PerfOptions key, remove it — but only if now empty
    // (delete_subkey fails on a non-empty key, which is the safe behaviour).
    if !existed {
        if let Ok(exe_key) =
            hklm.open_subkey_with_flags(format!("{}\\{}", IFEO_PATH, exe_name), KEY_ALL_ACCESS)
        {
            let _ = exe_key.delete_subkey("PerfOptions");
        }
    }

    state.applied.remove(ID);
    for k in ["exe-priority:path", "exe-priority:exe", "exe-priority:existed"] {
        state.saved_strings.remove(k);
    }
    state.saved_dwords.remove("exe-priority:CpuPriorityClass");
    TweakOpResult::ok(format!("Priority settings removed for {}", exe_name))
}

/// Public wrapper called from lib.rs — loads/saves state internally.
pub fn apply_exe_priority_pub(exe_path: &str) -> TweakOpResult {
    let mut state = load_state();
    let result = apply_exe_priority_impl(&mut state, exe_path);
    save_state(&state);
    result
}

// ── Per-exe: Prefer High-Performance GPU (UserGpuPreferences) ─────────────────
// Adapted from Fortnite-Optimizer's optimizeGPUPreference(). NOTE: the source
// wrongly wrote to HKLM — this preference is a per-user setting and belongs in
// HKCU (same place the Windows Settings > Graphics page writes it).

const GPU_PREF_PATH: &str = "Software\\Microsoft\\DirectX\\UserGpuPreferences";

fn apply_exe_gpu_pref_impl(state: &mut ExpState, exe_path: &str) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    const ID: &str = "prefer-high-perf-gpu-selected-exe";
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = match hkcu.create_subkey(GPU_PREF_PATH) {
        Ok(k) => k,
        Err(e) => return TweakOpResult::fail("Cannot open UserGpuPreferences key", e.to_string()),
    };

    if let Some(saved) = state.saved_strings.get("exe-gpupref:path") {
        if saved != exe_path {
            return TweakOpResult::fail(
                "A different executable already has saved GPU-preference state",
                "Revert the current one before applying this tweak to another file",
            );
        }
    }
    let existing: String = key.get_value(exe_path).unwrap_or_default();
    state
        .saved_strings
        .entry("exe-gpupref:path".into())
        .or_insert(exe_path.to_string());
    state
        .saved_strings
        .entry("exe-gpupref:original".into())
        .or_insert(existing.clone());

    match key.set_value(exe_path, &"GpuPreference=2;") {
        Ok(_) => {
            state.applied.insert(ID.into(), true);
            let exe_name = std::path::Path::new(exe_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(exe_path);
            TweakOpResult::ok(format!(
                "{} set to prefer the high-performance GPU. Restart the game to take effect.",
                exe_name
            ))
        }
        Err(e) => TweakOpResult::fail("Failed to write GPU preference", e.to_string()),
    }
}

fn revert_exe_gpu_pref_impl(state: &mut ExpState) -> TweakOpResult {
    use winreg::enums::*;
    use winreg::RegKey;
    const ID: &str = "prefer-high-perf-gpu-selected-exe";
    let exe_path = match state.saved_strings.get("exe-gpupref:path").cloned() {
        Some(p) => p,
        None => {
            state.applied.remove(ID);
            return TweakOpResult::ok("No per-exe GPU preference to revert");
        }
    };
    let original = state
        .saved_strings
        .get("exe-gpupref:original")
        .cloned()
        .unwrap_or_default();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = match hkcu.create_subkey(GPU_PREF_PATH) {
        Ok(k) => k,
        Err(e) => return TweakOpResult::fail("Cannot open UserGpuPreferences key", e.to_string()),
    };
    let result = if original.is_empty() {
        let _ = key.delete_value(&exe_path);
        TweakOpResult::ok("GPU preference reset to Windows default (Let Windows decide)")
    } else {
        match key.set_value(&exe_path, &original) {
            Ok(_) => TweakOpResult::ok("GPU preference restored to previous value"),
            Err(e) => TweakOpResult::fail("Failed to restore GPU preference", e.to_string()),
        }
    };
    if result.success {
        state.applied.remove(ID);
        state.saved_strings.remove("exe-gpupref:path");
        state.saved_strings.remove("exe-gpupref:original");
    }
    result
}

/// Public wrapper called from lib.rs — loads/saves state internally.
pub fn apply_exe_gpu_pref_pub(exe_path: &str) -> TweakOpResult {
    let mut state = load_state();
    let result = apply_exe_gpu_pref_impl(&mut state, exe_path);
    save_state(&state);
    result
}

// â”€â”€ NVIDIA detection â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub fn detect_nvidia_impl() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    hklm.open_subkey("SOFTWARE\\NVIDIA Corporation").is_ok()
        || hklm
            .open_subkey("SOFTWARE\\WOW6432Node\\NVIDIA Corporation")
            .is_ok()
}

// â”€â”€ Status checking â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

// ── Real-state status checks (prefer actual system state over the stored map) ─

/// disable-nagle is applied when any interface carries TcpAckFrequency = 1.
fn check_nagle_applied() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    let path = "SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters\\Interfaces";
    if let Ok(k) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(path) {
        for sub in k.enum_keys().flatten() {
            if let Ok(ik) = k.open_subkey(&sub) {
                if ik
                    .get_value::<u32, _>("TcpAckFrequency")
                    .map(|v| v == 1)
                    .unwrap_or(false)
                {
                    return true;
                }
            }
        }
    }
    false
}

/// disable-nic-power-saving is applied when a physical NIC has PnPCapabilities = 24.
fn check_nic_power_saving_applied() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    enumerate_net_adapter_keys().iter().any(|p| {
        hklm.open_subkey(p)
            .ok()
            .and_then(|k| k.get_value::<u32, _>("PnPCapabilities").ok())
            .map(|v| v == 24)
            .unwrap_or(false)
    })
}

/// Per-exe priority is applied when the stored exe's IFEO CpuPriorityClass = 3.
fn check_exe_priority_applied() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    let state = load_state();
    let exe = match state.saved_strings.get("exe-priority:exe") {
        Some(e) => e,
        None => return false,
    };
    let perf = format!("{}\\{}\\PerfOptions", IFEO_PATH, exe);
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(perf)
        .ok()
        .and_then(|k| k.get_value::<u32, _>("CpuPriorityClass").ok())
        .map(|v| v == 3)
        .unwrap_or(false)
}

/// Per-exe GPU preference is applied when the stored exe's UserGpuPreferences = high-perf.
fn check_exe_gpu_pref_applied() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    let state = load_state();
    let path = match state.saved_strings.get("exe-gpupref:path") {
        Some(p) => p,
        None => return false,
    };
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(GPU_PREF_PATH)
        .ok()
        .and_then(|k| k.get_value::<String, _>(path).ok())
        .map(|v| v.contains("GpuPreference=2"))
        .unwrap_or(false)
}

pub fn check_status_impl(tweak_id: &str) -> bool {
    // Load state only when needed (ultimate-performance uses saved GUID)
    if tweak_id == "set-ultimate-performance" {
        let state = load_state();
        return check_ultimate_performance(&state);
    }
    match tweak_id {
        // Interface
        "show-file-extensions" => hkcu_dword_eq(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced",
            "HideFileExt",
            0,
        ),
        "disable-widgets" => hkcu_dword_eq(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced",
            "TaskbarDa",
            0,
        ),
        "disable-transparency-effects" => hkcu_dword_eq(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
            "EnableTransparency",
            0,
        ),
        "disable-animations" => {
            hkcu_string_eq("Control Panel\\Desktop\\WindowMetrics", "MinAnimate", "0")
        }
        "dark-mode" => hkcu_dword_eq(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
            "AppsUseLightTheme",
            0,
        ),
        "classic-context-menu" => {
            use winreg::enums::*;
            use winreg::RegKey;
            RegKey::predef(HKEY_CURRENT_USER)
                .open_subkey("SOFTWARE\\Classes\\CLSID\\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}\\InprocServer32")
                .is_ok()
        }
        "disable-visual-effects" => hkcu_dword_eq(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VisualEffects",
            "VisualFXSetting",
            2,
        ),

        // Privacy
        "disable-advertising-id" => hkcu_dword_eq(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\AdvertisingInfo",
            "Enabled",
            0,
        ),
        "disable-windows-tips" => hkcu_dword_eq(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\ContentDeliveryManager",
            "SoftLandingEnabled",
            0,
        ),
        "disable-consumer-features" => hkcu_dword_eq(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\ContentDeliveryManager",
            "SilentInstalledAppsEnabled",
            0,
        ),
        "disable-tailored-experiences" => hkcu_dword_eq(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Privacy",
            "TailoredExperiencesWithDiagnosticDataEnabled",
            0,
        ),
        "disable-feedback-notifications" => hkcu_dword_eq(
            "SOFTWARE\\Microsoft\\Siuf\\Rules",
            "NumberOfSIUFInPeriod",
            0,
        ),
        "disable-telemetry" => hklm_dword_eq(
            "SOFTWARE\\Policies\\Microsoft\\Windows\\DataCollection",
            "AllowTelemetry",
            0,
        ),
        "disable-activity-history" => hkcu_dword_eq(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced",
            "Start_TrackProgs",
            0,
        ),

        // Gaming
        "disable-gamedvr" => hkcu_dword_eq("System\\GameConfigStore", "GameDVR_Enabled", 0),
        "disable-game-bar-capture" => hkcu_dword_eq(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\GameDVR",
            "AppCaptureEnabled",
            0,
        ),
        "disable-fullscreen-optimizations" => {
            hkcu_dword_eq("System\\GameConfigStore", "GameDVR_FSEBehaviorMode", 2)
        }
        "enable-game-mode" => {
            hkcu_dword_eq("SOFTWARE\\Microsoft\\GameBar", "AutoGameModeEnabled", 1)
        }
        "gpu-scheduling" => hklm_dword_eq(
            "SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers",
            "HwSchMode",
            2,
        ),

        // Performance (set-ultimate-performance is handled before this match)
        "disable-sysmain" => service_start_type("SysMain")
            .map(|s| s == 4)
            .unwrap_or(false),
        "disable-windows-search" => service_start_type("WSearch")
            .map(|s| s == 4)
            .unwrap_or(false),

        // Startup
        "disable-onedrive-startup" => !hkcu_run_value_exists("OneDrive"),
        "reduce-startup-delay" => hkcu_dword_eq(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Serialize",
            "StartupDelayInMSec",
            0,
        ),
        "disable-edge-startup-boost" => {
            hkcu_dword_eq("SOFTWARE\\Microsoft\\Edge\\Main", "StartupBoostEnabled", 0)
        }
        "disable-edge-background-mode" => hkcu_dword_eq(
            "SOFTWARE\\Microsoft\\Edge\\Main",
            "BackgroundModeEnabled",
            0,
        ),

        // Performance
        "disable-power-throttling" => hklm_dword_eq(
            "SYSTEM\\CurrentControlSet\\Control\\Power\\PowerThrottling",
            "PowerThrottlingOff",
            1,
        ),

        // Network
        "disable-delivery-optimization" => hklm_dword_eq(
            "SOFTWARE\\Policies\\Microsoft\\Windows\\DeliveryOptimization",
            "DODownloadMode",
            0,
        ),
        "network-throttling-index" => hklm_dword_eq(
            "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile",
            "NetworkThrottlingIndex",
            0xFFFFFFFF,
        ),
        "system-responsiveness" => hklm_dword_eq(
            "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile",
            "SystemResponsiveness",
            0,
        ),

        // Performance (v1.1.0)
        "adjust-page-file" => hklm_dword_eq(
            "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Memory Management",
            "AutomaticManagedPagefile",
            1,
        ),
        // Privacy (v1.1.0)
        "disable-suggested-content" => hkcu_dword_eq(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\ContentDeliveryManager",
            "SubscribedContent-338393Enabled",
            0,
        ),
        // Network (v1.1.0) â€” dns-cache-size uses stored state (default may not exist in registry)
        "dns-cache-size" => {
            use winreg::enums::*;
            use winreg::RegKey;
            RegKey::predef(HKEY_LOCAL_MACHINE)
                .open_subkey(DNS_PARAMS_PATH)
                .ok()
                .and_then(|k| k.get_value::<u32, _>("MaxCacheEntryTtlLimit").ok())
                .map(|v| v == 86400)
                .unwrap_or(false)
        }

        // Interface (v1.2.0)
        "classic-alt-tab" => hkcu_dword_eq(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer",
            "AltTabSettings",
            1,
        ),

        // Gaming (v1.3.0) â€” check any NVIDIA device has MSISupported = 1
        "nvidia-msi-mode" => {
            use winreg::enums::*;
            use winreg::RegKey;
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            enumerate_nvidia_msi_paths().iter().any(|p| {
                hklm.open_subkey(p)
                    .ok()
                    .and_then(|k| k.get_value::<u32, _>("MSISupported").ok())
                    .map(|v| v == 1)
                    .unwrap_or(false)
            })
        }

        // Performance (v1.6.0) — kernel-in-RAM has a direct registry check
        "keep-kernel-in-ram" => hklm_dword_eq(MEM_MGMT_PATH, "DisablePagingExecutive", 1),

        // v1.7.0 — ToX adaptations with direct registry checks
        "disable-mouse-accel" => hkcu_string_eq(MOUSE_PATH, "MouseSpeed", "0"),
        "fast-keyboard-repeat" => hkcu_string_eq(KEYBOARD_PATH, "KeyboardDelay", "0"),
        "foreground-boost" => {
            hklm_dword_eq(PRIORITY_CONTROL_PATH, "Win32PrioritySeparation", 0x1A)
        }
        "disable-hibernate" => hklm_dword_eq(
            "SYSTEM\\CurrentControlSet\\Control\\Power",
            "HibernateEnabled",
            0,
        ),

        // Real-state checks — read the actual system instead of the stored map,
        // so these can't desync if the setting is changed outside VOptimizer.
        "disable-nagle" => check_nagle_applied(),
        "disable-nic-power-saving" => check_nic_power_saving_applied(),
        "set-game-priority-selected-exe" => check_exe_priority_applied(),
        "prefer-high-perf-gpu-selected-exe" => check_exe_gpu_pref_applied(),

        // Stored-state only (no simple registry check):
        // ram-standby-cleaner (one-shot), disable-nagle (multi-key),
        // disable-core-parking (powercfg), and the per-exe tweaks
        // (set-game-priority/prefer-high-perf-gpu/disable-fullscreen-opt-selected-exe)
        // are all tracked via the applied map.
        _ => load_state().applied.get(tweak_id).copied().unwrap_or(false),
    }
}

pub fn check_all_statuses_impl(ids: &[String]) -> HashMap<String, bool> {
    ids.iter()
        .map(|id| (id.clone(), check_status_impl(id)))
        .collect()
}

// â”€â”€ Apply / revert dispatch â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub fn apply_impl(tweak_id: &str) -> TweakOpResult {
    let mut state = load_state();
    let result = match tweak_id {
        // Performance
        "set-ultimate-performance" => apply_ultimate_performance(&mut state),
        "disable-visual-effects" => apply_visual_effects(&mut state),
        "disable-sysmain" => apply_sysmain(&mut state),
        "disable-windows-search" => apply_windows_search(&mut state),
        "disable-power-throttling" => apply_power_throttling(&mut state),
        // Gaming
        "disable-gamedvr" => apply_gamedvr(&mut state),
        "disable-game-bar-capture" => apply_game_bar_capture(&mut state),
        "disable-fullscreen-optimizations" => apply_fullscreen_optimizations(&mut state),
        "enable-game-mode" => apply_game_mode(&mut state),
        "gpu-scheduling" => apply_gpu_scheduling(&mut state),
        // Privacy
        "disable-advertising-id" => apply_advertising_id(&mut state),
        "disable-windows-tips" => apply_windows_tips(&mut state),
        "disable-consumer-features" => apply_consumer_features(&mut state),
        "disable-tailored-experiences" => apply_tailored_experiences(&mut state),
        "disable-feedback-notifications" => apply_feedback_notifications(&mut state),
        "disable-telemetry" => apply_telemetry(&mut state),
        "disable-activity-history" => apply_activity_history(&mut state),
        "disable-nvidia-telemetry" => apply_nvidia_telemetry(&mut state),
        "disable-amd-telemetry" => apply_amd_telemetry(&mut state),
        // Network
        "disable-delivery-optimization" => apply_delivery_optimization(&mut state),
        "network-throttling-index" => apply_network_throttling(&mut state),
        "system-responsiveness" => apply_system_responsiveness(&mut state),
        // Startup
        "disable-onedrive-startup" => apply_onedrive_startup(&mut state),
        "reduce-startup-delay" => apply_startup_delay(&mut state),
        "disable-edge-startup-boost" => apply_edge_startup_boost(&mut state),
        "disable-edge-background-mode" => apply_edge_background(&mut state),
        "disable-nvidia-overlay-startup" => apply_nvidia_overlay_startup(&mut state),
        "disable-amd-radeon-autostart" => apply_amd_radeon_autostart(&mut state),
        // Interface
        "show-file-extensions" => apply_file_extensions(&mut state),
        "dark-mode" => apply_dark_mode(&mut state),
        "classic-context-menu" => apply_classic_context_menu(&mut state),
        "disable-widgets" => apply_widgets(&mut state),
        "disable-transparency-effects" => apply_transparency(&mut state),
        "disable-animations" => apply_animations(&mut state),
        // Performance (v1.1.0)
        "adjust-page-file" => apply_adjust_page_file(&mut state),
        "ram-standby-cleaner" => apply_ram_standby_cleaner(&mut state),
        // Privacy (v1.1.0)
        "disable-suggested-content" => apply_suggested_content(&mut state),
        // Network (v1.1.0)
        "disable-nagle" => apply_disable_nagle(&mut state),
        "dns-cache-size" => apply_dns_cache_size(&mut state),
        // Gaming (v1.3.0)
        "nvidia-msi-mode" => apply_nvidia_msi_mode(&mut state),
        // Interface (v1.2.0)
        "classic-alt-tab" => apply_alt_tab(&mut state),
        // Performance / Gaming (v1.6.0 — Fortnite-Optimizer adaptations)
        "disable-core-parking" => apply_core_parking(&mut state),
        "keep-kernel-in-ram" => apply_keep_kernel_in_ram(&mut state),
        // v1.7.0 — ToX adaptations
        "disable-usb-selective-suspend" => apply_usb_suspend(&mut state),
        "disable-hibernate" => apply_disable_hibernate(&mut state),
        "disable-mouse-accel" => apply_mouse_accel(&mut state),
        "fast-keyboard-repeat" => apply_fast_keyboard(&mut state),
        "foreground-boost" => apply_foreground_boost(&mut state),
        "disable-nic-power-saving" => apply_nic_power_saving(&mut state),
        "disable-lso" => apply_disable_lso(&mut state),
        // Per-exe tweaks apply via their own *_pub commands (they need a path arg):
        //   disable-fullscreen-optimizations-selected-exe → apply_exe_fsopt_pub
        //   set-game-priority-selected-exe                 → apply_exe_priority_pub
        //   prefer-high-perf-gpu-selected-exe              → apply_exe_gpu_pref_pub
        _ => TweakOpResult::fail(
            format!("'{}' is a placeholder â€” not yet implemented", tweak_id),
            "placeholder",
        ),
    };
    save_state(&state);
    result
}

pub fn revert_impl(tweak_id: &str) -> TweakOpResult {
    let mut state = load_state();
    let result = match tweak_id {
        // Performance
        "set-ultimate-performance" => revert_ultimate_performance(&mut state),
        "disable-visual-effects" => revert_visual_effects(&mut state),
        "disable-sysmain" => revert_sysmain(&mut state),
        "disable-windows-search" => revert_windows_search(&mut state),
        "disable-power-throttling" => revert_power_throttling(&mut state),
        // Gaming
        "disable-gamedvr" => revert_gamedvr(&mut state),
        "disable-game-bar-capture" => revert_game_bar_capture(&mut state),
        "disable-fullscreen-optimizations" => revert_fullscreen_optimizations(&mut state),
        "enable-game-mode" => revert_game_mode(&mut state),
        "gpu-scheduling" => revert_gpu_scheduling(&mut state),
        // Privacy
        "disable-advertising-id" => revert_advertising_id(&mut state),
        "disable-windows-tips" => revert_windows_tips(&mut state),
        "disable-consumer-features" => revert_consumer_features(&mut state),
        "disable-tailored-experiences" => revert_tailored_experiences(&mut state),
        "disable-feedback-notifications" => revert_feedback_notifications(&mut state),
        "disable-telemetry" => revert_telemetry(&mut state),
        "disable-activity-history" => revert_activity_history(&mut state),
        "disable-nvidia-telemetry" => revert_nvidia_telemetry(&mut state),
        "disable-amd-telemetry" => revert_amd_telemetry(&mut state),
        // Network
        "disable-delivery-optimization" => revert_delivery_optimization(&mut state),
        "network-throttling-index" => revert_network_throttling(&mut state),
        "system-responsiveness" => revert_system_responsiveness(&mut state),
        // Startup
        "disable-onedrive-startup" => revert_onedrive_startup(&mut state),
        "reduce-startup-delay" => revert_startup_delay(&mut state),
        "disable-edge-startup-boost" => revert_edge_startup_boost(&mut state),
        "disable-edge-background-mode" => revert_edge_background(&mut state),
        "disable-nvidia-overlay-startup" => revert_nvidia_overlay_startup(&mut state),
        "disable-amd-radeon-autostart" => revert_amd_radeon_autostart(&mut state),
        // Interface
        "show-file-extensions" => revert_file_extensions(&mut state),
        "dark-mode" => revert_dark_mode(&mut state),
        "classic-context-menu" => revert_classic_context_menu(&mut state),
        "disable-widgets" => revert_widgets(&mut state),
        "disable-transparency-effects" => revert_transparency(&mut state),
        "disable-animations" => revert_animations(&mut state),
        // Performance (v1.1.0)
        "adjust-page-file" => revert_adjust_page_file(&mut state),
        "ram-standby-cleaner" => revert_ram_standby_cleaner(&mut state),
        // Privacy (v1.1.0)
        "disable-suggested-content" => revert_suggested_content(&mut state),
        // Network (v1.1.0)
        "disable-nagle" => revert_disable_nagle(&mut state),
        "dns-cache-size" => revert_dns_cache_size(&mut state),
        // Gaming (v1.3.0)
        "nvidia-msi-mode" => revert_nvidia_msi_mode(&mut state),
        // Interface (v1.2.0)
        "classic-alt-tab" => revert_alt_tab(&mut state),
        // Gaming (v1.2.0)
        "disable-fullscreen-optimizations-selected-exe" => revert_exe_fsopt_impl(&mut state),
        // Performance / Gaming (v1.6.0 — Fortnite-Optimizer adaptations)
        "disable-core-parking" => revert_core_parking(&mut state),
        "keep-kernel-in-ram" => revert_keep_kernel_in_ram(&mut state),
        "set-game-priority-selected-exe" => revert_exe_priority_impl(&mut state),
        "prefer-high-perf-gpu-selected-exe" => revert_exe_gpu_pref_impl(&mut state),
        // v1.7.0 — ToX adaptations
        "disable-usb-selective-suspend" => revert_usb_suspend(&mut state),
        "disable-hibernate" => revert_disable_hibernate(&mut state),
        "disable-mouse-accel" => revert_mouse_accel(&mut state),
        "fast-keyboard-repeat" => revert_fast_keyboard(&mut state),
        "foreground-boost" => revert_foreground_boost(&mut state),
        "disable-nic-power-saving" => revert_nic_power_saving(&mut state),
        "disable-lso" => revert_disable_lso(&mut state),
        _ => TweakOpResult::fail(
            format!("'{}' cannot be reverted â€” not yet implemented", tweak_id),
            "placeholder",
        ),
    };
    save_state(&state);
    result
}
