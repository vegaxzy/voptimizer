use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(windows)]
use winreg::enums::*;
#[cfg(windows)]
use winreg::RegKey;

// ── Registry paths ─────────────────────────────────────────────────────────

const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
// 32-bit apps on 64-bit Windows register their HKLM autostart here.
const RUN_WOW64_SUBKEY: &str =
    r"Software\Wow6432Node\Microsoft\Windows\CurrentVersion\Run";
const VOPT_DISABLED_ROOT: &str = r"Software\VOptimizer\DisabledStartup";
// Where Windows / Task Manager record a Run entry's enabled/disabled state.
// Byte 0 has its low bit clear (e.g. 02) when enabled, set (e.g. 03) when disabled.
const STARTUP_APPROVED_RUN: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
const STARTUP_APPROVED_RUN32: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32";

// ── Safety heuristics ──────────────────────────────────────────────────────

const SENSITIVE_PATTERNS: &[&str] = &[
    "microsoft",
    "defender",
    "securityhealth",
    "windowssecurity",
    "onedrive",
    "nvidia",
    "amdradeon",
    "inteldisplay",
    "intelgraphics",
];

fn is_sensitive(name: &str, command: &str) -> bool {
    let n = name.to_lowercase().replace([' ', '-', '_'], "");
    let c = command.to_lowercase();
    SENSITIVE_PATTERNS
        .iter()
        .any(|p| n.contains(p) || c.contains(p))
}

// ── Public types ───────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StartupApp {
    pub id: String,
    pub name: String,
    pub command: String,
    pub source: String,
    pub source_display: String,
    pub status: String,
    pub is_sensitive: bool,
}

#[derive(Serialize, Deserialize)]
pub struct StartupOpResult {
    pub success: bool,
    pub message: String,
    pub data: Option<StartupApp>,
    pub error: Option<String>,
}

// ── ID helpers ─────────────────────────────────────────────────────────────

fn make_id(source: &str, name: &str) -> String {
    format!("{}:{}", source, name)
}

/// Encodes an (source, entry-name) pair as a registry subkey name.
/// Registry key names must not contain `\` or NUL.
fn encode_subkey(source: &str, name: &str) -> String {
    let safe = name.replace('\\', "_").replace('\0', "_");
    format!("{}|{}", source, safe)
}

fn source_display(source: &str) -> &'static str {
    match source {
        "hkcu_run" => r"HKCU\Run",
        "hklm_run" => r"HKLM\Run",
        "hklm_run_wow64" => r"HKLM\Run (32-bit)",
        "user_startup" => "User Startup",
        "common_startup" => "Common Startup",
        _ => "Unknown",
    }
}

/// For a registry startup source, returns (is_hkcu, run_subkey, approved_subkey).
#[cfg(windows)]
fn registry_source_paths(source: &str) -> Option<(bool, &'static str, &'static str)> {
    match source {
        "hkcu_run" => Some((true, RUN_SUBKEY, STARTUP_APPROVED_RUN)),
        "hklm_run" => Some((false, RUN_SUBKEY, STARTUP_APPROVED_RUN)),
        "hklm_run_wow64" => Some((false, RUN_WOW64_SUBKEY, STARTUP_APPROVED_RUN32)),
        _ => None,
    }
}

/// True when Windows/Task Manager has flagged this Run value as disabled in
/// StartupApproved (byte 0's low bit is set, e.g. 03/07 vs 02/06 when enabled).
#[cfg(windows)]
fn is_approved_disabled(hive: &RegKey, approved_subkey: &str, value_name: &str) -> bool {
    hive.open_subkey(approved_subkey)
        .ok()
        .and_then(|k| k.get_raw_value(value_name).ok())
        .and_then(|raw| raw.bytes.first().copied())
        .map(|b0| b0 & 1 == 1)
        .unwrap_or(false)
}

// ── Listing helpers ────────────────────────────────────────────────────────

#[cfg(windows)]
fn list_registry_run(hive: &RegKey, source: &str) -> Vec<StartupApp> {
    let (_, run_subkey, approved_subkey) = match registry_source_paths(source) {
        Some(v) => v,
        None => return Vec::new(),
    };
    let key = match hive.open_subkey(run_subkey) {
        Ok(k) => k,
        Err(_) => return Vec::new(),
    };
    let names: Vec<String> = key
        .enum_values()
        .filter_map(|r| r.ok())
        .map(|(n, _)| n)
        .filter(|n| !n.is_empty())
        .collect();

    names
        .into_iter()
        .filter_map(|name| {
            let command: String = key.get_value(&name).ok()?;
            let disp = source_display(source).to_string();
            let sensitive = is_sensitive(&name, &command);
            // Reflect Task Manager / Windows StartupApproved state so an entry
            // disabled outside VOptimizer isn't shown as enabled.
            let status = if is_approved_disabled(hive, approved_subkey, &name) {
                "disabled"
            } else {
                "enabled"
            };
            Some(StartupApp {
                id: make_id(source, &name),
                name,
                command,
                source: source.to_string(),
                source_display: disp,
                status: status.to_string(),
                is_sensitive: sensitive,
            })
        })
        .collect()
}

#[cfg(windows)]
fn list_folder_entries(folder: &PathBuf, source: &str) -> Vec<StartupApp> {
    let disp = source_display(source).to_string();
    let entries = match std::fs::read_dir(folder) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    entries
        .filter_map(|e| e.ok())
        .filter_map(|entry| {
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.starts_with('.') || file_name.eq_ignore_ascii_case("desktop.ini") {
                return None;
            }
            let command = entry.path().to_string_lossy().to_string();
            let name = std::path::Path::new(&file_name)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| file_name.clone());
            let sensitive = is_sensitive(&name, &command);
            Some(StartupApp {
                id: make_id(source, &file_name),
                name,
                command,
                source: source.to_string(),
                source_display: disp.clone(),
                status: "enabled".to_string(),
                is_sensitive: sensitive,
            })
        })
        .collect()
}

#[cfg(windows)]
fn list_disabled_registry() -> Vec<StartupApp> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let root = match hkcu.open_subkey(VOPT_DISABLED_ROOT) {
        Ok(k) => k,
        Err(_) => return Vec::new(),
    };
    let subkeys: Vec<String> = root.enum_keys().filter_map(|r| r.ok()).collect();

    subkeys
        .into_iter()
        .filter_map(|sk| {
            let entry = root.open_subkey(&sk).ok()?;
            let orig_source: String = entry.get_value("OriginalSource").ok()?;
            let name: String = entry.get_value("Name").ok()?;
            let command: String = entry.get_value("Command").unwrap_or_default();
            if name.is_empty() {
                return None;
            }
            // Only registry-origin entries are stored here
            if orig_source != "hkcu_run" && orig_source != "hklm_run" {
                return None;
            }
            let disp = source_display(&orig_source).to_string();
            let sensitive = is_sensitive(&name, &command);
            Some(StartupApp {
                id: make_id(&orig_source, &name),
                name,
                command,
                source: orig_source,
                source_display: disp,
                status: "disabled".to_string(),
                is_sensitive: sensitive,
            })
        })
        .collect()
}

#[cfg(windows)]
fn disabled_folder_path() -> Option<PathBuf> {
    std::env::var("APPDATA")
        .ok()
        .map(|d| PathBuf::from(d).join("VOptimizer").join("DisabledStartup"))
}

#[cfg(windows)]
fn list_disabled_folder() -> Vec<StartupApp> {
    let dir = match disabled_folder_path() {
        Some(p) => p,
        None => return Vec::new(),
    };
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    entries
        .filter_map(|e| e.ok())
        .filter_map(|entry| {
            let fname = entry.file_name().to_string_lossy().to_string();
            if !fname.ends_with(".meta") {
                return None;
            }
            #[derive(Deserialize)]
            struct Meta {
                original_source: String,
                file_name: String,
                display_name: String,
                command: String,
            }
            let content = std::fs::read_to_string(entry.path()).ok()?;
            let meta: Meta = serde_json::from_str(&content).ok()?;
            let disp = source_display(&meta.original_source).to_string();
            let sensitive = is_sensitive(&meta.display_name, &meta.command);
            Some(StartupApp {
                id: make_id(&meta.original_source, &meta.file_name),
                name: meta.display_name,
                command: meta.command,
                source: meta.original_source,
                source_display: disp,
                status: "disabled".to_string(),
                is_sensitive: sensitive,
            })
        })
        .collect()
}

// ── Disable helpers ────────────────────────────────────────────────────────

#[cfg(windows)]
fn disable_registry_entry(source: &str, name: &str) -> StartupOpResult {
    let (is_hkcu, run_subkey, _approved) = match registry_source_paths(source) {
        Some(v) => v,
        None => {
            return StartupOpResult {
                success: false,
                message: format!("Unknown source '{}'", source),
                data: None,
                error: Some("Unrecognised startup source".to_string()),
            }
        }
    };
    let hive_predef = if is_hkcu {
        HKEY_CURRENT_USER
    } else {
        HKEY_LOCAL_MACHINE
    };

    // 1. Read current command (read-only open is enough)
    let command: String = match RegKey::predef(hive_predef)
        .open_subkey(run_subkey)
        .and_then(|k| k.get_value(name))
    {
        Ok(v) => v,
        Err(e) => {
            return StartupOpResult {
                success: false,
                message: format!("Entry '{}' not found in Run key", name),
                data: None,
                error: Some(e.to_string()),
            }
        }
    };

    // 2. Persist to VOptimizer disabled storage under HKCU
    let subkey = encode_subkey(source, name);
    let storage_path = format!("{}\\{}", VOPT_DISABLED_ROOT, subkey);
    let store_result = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(&storage_path)
        .and_then(|(k, _)| {
            k.set_value("OriginalSource", &source.to_string())?;
            k.set_value("Name", &name.to_string())?;
            k.set_value("Command", &command)?;
            Ok(())
        });

    if let Err(e) = store_result {
        return StartupOpResult {
            success: false,
            message: "Could not write to VOptimizer disabled storage".to_string(),
            data: None,
            error: Some(e.to_string()),
        };
    }

    // 3. Delete from original Run key (requires write access)
    let delete_result = RegKey::predef(hive_predef)
        .open_subkey_with_flags(run_subkey, KEY_ALL_ACCESS)
        .and_then(|k| k.delete_value(name));

    if let Err(e) = delete_result {
        // Roll back stored entry
        let _ = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(VOPT_DISABLED_ROOT, KEY_ALL_ACCESS)
            .and_then(|k| k.delete_subkey_all(&subkey));
        let hint = if !is_hkcu {
            " (HKLM entries require administrator privileges)"
        } else {
            ""
        };
        return StartupOpResult {
            success: false,
            message: format!("Could not remove from Run key{}", hint),
            data: None,
            error: Some(e.to_string()),
        };
    }

    let disp = source_display(source).to_string();
    let sensitive = is_sensitive(name, &command);
    StartupOpResult {
        success: true,
        message: format!("'{}' disabled successfully", name),
        data: Some(StartupApp {
            id: make_id(source, name),
            name: name.to_string(),
            command,
            source: source.to_string(),
            source_display: disp,
            status: "disabled".to_string(),
            is_sensitive: sensitive,
        }),
        error: None,
    }
}

#[cfg(windows)]
fn startup_folder_path(is_user: bool) -> Option<PathBuf> {
    let base = if is_user {
        std::env::var("APPDATA").ok()?
    } else {
        std::env::var("PROGRAMDATA").ok()?
    };
    Some(
        PathBuf::from(base)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Startup"),
    )
}

#[cfg(windows)]
fn disable_folder_entry(source: &str, file_name: &str, is_user: bool) -> StartupOpResult {
    let src_folder = match startup_folder_path(is_user) {
        Some(p) => p,
        None => {
            return StartupOpResult {
                success: false,
                message: "Cannot resolve startup folder path".to_string(),
                data: None,
                error: Some("Environment variable not available".to_string()),
            }
        }
    };
    let src_file = src_folder.join(file_name);
    if !src_file.exists() {
        return StartupOpResult {
            success: false,
            message: format!("'{}' not found in startup folder", file_name),
            data: None,
            error: Some("File not found".to_string()),
        };
    }

    let dst_dir = match disabled_folder_path() {
        Some(p) => p,
        None => {
            return StartupOpResult {
                success: false,
                message: "Cannot resolve VOptimizer disabled folder path".to_string(),
                data: None,
                error: Some("APPDATA not available".to_string()),
            }
        }
    };
    if let Err(e) = std::fs::create_dir_all(&dst_dir) {
        return StartupOpResult {
            success: false,
            message: format!("Cannot create disabled folder: {}", e),
            data: None,
            error: Some(e.to_string()),
        };
    }

    let dst_file = dst_dir.join(file_name);
    if let Err(e) = std::fs::rename(&src_file, &dst_file) {
        return StartupOpResult {
            success: false,
            message: format!("Cannot move file to disabled storage: {}", e),
            data: None,
            error: Some(e.to_string()),
        };
    }

    // Write metadata sidecar
    let display_name = std::path::Path::new(file_name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| file_name.to_string());
    let command = dst_file.to_string_lossy().to_string();
    let meta = serde_json::json!({
        "original_source": source,
        "file_name": file_name,
        "display_name": display_name,
        "command": command,
        "original_folder": src_folder.to_string_lossy().to_string(),
    });
    std::fs::write(
        dst_dir.join(format!("{}.meta", file_name)),
        meta.to_string(),
    )
    .ok();

    let disp = source_display(source).to_string();
    let sensitive = is_sensitive(&display_name, &command);
    StartupOpResult {
        success: true,
        message: format!("'{}' disabled successfully", display_name),
        data: Some(StartupApp {
            id: make_id(source, file_name),
            name: display_name,
            command,
            source: source.to_string(),
            source_display: disp,
            status: "disabled".to_string(),
            is_sensitive: sensitive,
        }),
        error: None,
    }
}

// ── Enable helpers ─────────────────────────────────────────────────────────

/// Clears the StartupApproved "disabled" flag (low bit of byte 0) so Windows
/// treats the Run entry as enabled again — the same thing Task Manager does.
#[cfg(windows)]
fn clear_approved_flag(hive: &RegKey, approved_subkey: &str, name: &str) -> std::io::Result<()> {
    let key = hive.open_subkey_with_flags(approved_subkey, KEY_ALL_ACCESS)?;
    let mut raw = key.get_raw_value(name)?;
    if let Some(b0) = raw.bytes.first_mut() {
        *b0 &= !1u8;
    }
    key.set_raw_value(name, &raw)
}

#[cfg(windows)]
fn enable_registry_entry(source: &str, name: &str) -> StartupOpResult {
    let (is_hkcu, run_subkey, approved_subkey) = match registry_source_paths(source) {
        Some(v) => v,
        None => {
            return StartupOpResult {
                success: false,
                message: format!("Unknown source '{}'", source),
                data: None,
                error: Some("Unrecognised startup source".to_string()),
            }
        }
    };
    let hive_predef = if is_hkcu {
        HKEY_CURRENT_USER
    } else {
        HKEY_LOCAL_MACHINE
    };
    let admin_hint = if !is_hkcu {
        " (HKLM entries require administrator privileges)"
    } else {
        ""
    };
    let disp = source_display(source).to_string();
    let subkey = encode_subkey(source, name);
    let storage_path = format!("{}\\{}", VOPT_DISABLED_ROOT, subkey);

    // Case A — entry was disabled by VOptimizer (value moved to our storage).
    if let Ok(command) = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(&storage_path)
        .and_then(|k| k.get_value::<String, _>("Command"))
    {
        let restore = RegKey::predef(hive_predef)
            .open_subkey_with_flags(run_subkey, KEY_ALL_ACCESS)
            .and_then(|k| k.set_value(name, &command));
        if let Err(e) = restore {
            return StartupOpResult {
                success: false,
                message: format!("Could not restore to Run key{}", admin_hint),
                data: None,
                error: Some(e.to_string()),
            };
        }
        let _ = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(VOPT_DISABLED_ROOT, KEY_ALL_ACCESS)
            .and_then(|k| k.delete_subkey_all(&subkey));
        let sensitive = is_sensitive(name, &command);
        return StartupOpResult {
            success: true,
            message: format!("'{}' enabled successfully", name),
            data: Some(StartupApp {
                id: make_id(source, name),
                name: name.to_string(),
                command,
                source: source.to_string(),
                source_display: disp,
                status: "enabled".to_string(),
                is_sensitive: sensitive,
            }),
            error: None,
        };
    }

    // Case B — entry is still in the Run key but flagged disabled in
    // StartupApproved (e.g. disabled via Task Manager). Clear that flag.
    let hive = RegKey::predef(hive_predef);
    if is_approved_disabled(&hive, approved_subkey, name) {
        let command: String = hive
            .open_subkey(run_subkey)
            .and_then(|k| k.get_value(name))
            .unwrap_or_default();
        return match clear_approved_flag(&hive, approved_subkey, name) {
            Ok(_) => {
                let sensitive = is_sensitive(name, &command);
                StartupOpResult {
                    success: true,
                    message: format!("'{}' enabled successfully", name),
                    data: Some(StartupApp {
                        id: make_id(source, name),
                        name: name.to_string(),
                        command,
                        source: source.to_string(),
                        source_display: disp,
                        status: "enabled".to_string(),
                        is_sensitive: sensitive,
                    }),
                    error: None,
                }
            }
            Err(e) => StartupOpResult {
                success: false,
                message: format!("Could not clear the disabled flag{}", admin_hint),
                data: None,
                error: Some(e.to_string()),
            },
        };
    }

    StartupOpResult {
        success: false,
        message: format!("Disabled entry '{}' not found", name),
        data: None,
        error: Some("Not in VOptimizer storage or StartupApproved".to_string()),
    }
}

#[cfg(windows)]
fn enable_folder_entry(source: &str, file_name: &str, is_user: bool) -> StartupOpResult {
    let dis_dir = match disabled_folder_path() {
        Some(p) => p,
        None => {
            return StartupOpResult {
                success: false,
                message: "Cannot resolve VOptimizer disabled folder path".to_string(),
                data: None,
                error: Some("APPDATA not available".to_string()),
            }
        }
    };
    let dis_file = dis_dir.join(file_name);
    if !dis_file.exists() {
        return StartupOpResult {
            success: false,
            message: format!("'{}' not found in disabled storage", file_name),
            data: None,
            error: Some("File not found".to_string()),
        };
    }

    let dst_folder = match startup_folder_path(is_user) {
        Some(p) => p,
        None => {
            return StartupOpResult {
                success: false,
                message: "Cannot resolve startup folder path".to_string(),
                data: None,
                error: Some("Environment variable not available".to_string()),
            }
        }
    };
    let dst_file = dst_folder.join(file_name);
    if let Err(e) = std::fs::rename(&dis_file, &dst_file) {
        return StartupOpResult {
            success: false,
            message: format!("Cannot move file back to startup folder: {}", e),
            data: None,
            error: Some(e.to_string()),
        };
    }
    // Remove sidecar
    std::fs::remove_file(dis_dir.join(format!("{}.meta", file_name))).ok();

    let display_name = std::path::Path::new(file_name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| file_name.to_string());
    let command = dst_file.to_string_lossy().to_string();
    let disp = source_display(source).to_string();
    let sensitive = is_sensitive(&display_name, &command);
    StartupOpResult {
        success: true,
        message: format!("'{}' enabled successfully", display_name),
        data: Some(StartupApp {
            id: make_id(source, file_name),
            name: display_name,
            command,
            source: source.to_string(),
            source_display: disp,
            status: "enabled".to_string(),
            is_sensitive: sensitive,
        }),
        error: None,
    }
}

// ── Public entry points (called by Tauri commands) ─────────────────────────

pub fn list_impl() -> Vec<StartupApp> {
    #[cfg(not(windows))]
    return Vec::new();

    #[cfg(windows)]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        let mut apps = Vec::new();
        apps.extend(list_registry_run(&hkcu, "hkcu_run"));
        apps.extend(list_registry_run(&hklm, "hklm_run"));
        // 32-bit apps on 64-bit Windows register here — previously missed.
        apps.extend(list_registry_run(&hklm, "hklm_run_wow64"));

        if let Some(p) = startup_folder_path(true) {
            apps.extend(list_folder_entries(&p, "user_startup"));
        }
        if let Some(p) = startup_folder_path(false) {
            apps.extend(list_folder_entries(&p, "common_startup"));
        }

        apps.extend(list_disabled_registry());
        apps.extend(list_disabled_folder());

        // Deduplicate by ID
        let mut seen = std::collections::HashSet::new();
        apps.retain(|a| seen.insert(a.id.clone()));

        // Enabled first, then alphabetical by name
        apps.sort_by(|a, b| {
            a.status
                .cmp(&b.status) // "disabled" > "enabled" lexically, so reverse
                .reverse()
                .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });

        apps
    }
}

pub fn disable_impl(id: String) -> StartupOpResult {
    let Some(sep) = id.find(':') else {
        return StartupOpResult {
            success: false,
            message: "Invalid ID format".to_string(),
            data: None,
            error: Some("Expected 'source:name'".to_string()),
        };
    };
    let source = &id[..sep];
    let name = &id[sep + 1..];

    #[cfg(not(windows))]
    {
        let _ = (source, name);
        return StartupOpResult {
            success: false,
            message: "Not supported on this platform".to_string(),
            data: None,
            error: Some("Windows only".to_string()),
        };
    }

    #[cfg(windows)]
    match source {
        "hkcu_run" | "hklm_run" | "hklm_run_wow64" => disable_registry_entry(source, name),
        "user_startup" => disable_folder_entry(source, name, true),
        "common_startup" => disable_folder_entry(source, name, false),
        _ => StartupOpResult {
            success: false,
            message: format!("Unknown source '{}'", source),
            data: None,
            error: Some("Unrecognised startup source".to_string()),
        },
    }
}

pub fn enable_impl(id: String) -> StartupOpResult {
    let Some(sep) = id.find(':') else {
        return StartupOpResult {
            success: false,
            message: "Invalid ID format".to_string(),
            data: None,
            error: Some("Expected 'source:name'".to_string()),
        };
    };
    let source = &id[..sep];
    let name = &id[sep + 1..];

    #[cfg(not(windows))]
    {
        let _ = (source, name);
        return StartupOpResult {
            success: false,
            message: "Not supported on this platform".to_string(),
            data: None,
            error: Some("Windows only".to_string()),
        };
    }

    #[cfg(windows)]
    match source {
        "hkcu_run" | "hklm_run" | "hklm_run_wow64" => enable_registry_entry(source, name),
        "user_startup" => enable_folder_entry(source, name, true),
        "common_startup" => enable_folder_entry(source, name, false),
        _ => StartupOpResult {
            success: false,
            message: format!("Unknown source '{}'", source),
            data: None,
            error: Some("Unrecognised startup source".to_string()),
        },
    }
}
