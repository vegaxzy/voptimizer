use crate::util::no_window_cmd;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

// â”€â”€ Monotonic counter for ID uniqueness â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn new_id() -> String {
    let t = now_ms();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed) & 0xFFFF;
    format!("{:x}{:04x}", t, n)
}

// â”€â”€ Public types â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BackupEntry {
    pub id: String,
    pub timestamp: i64,
    pub label: String,
    pub registry_key: String,
    pub file_path: String,
    pub size_bytes: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp: i64,
    /// "disable_startup" | "enable_startup" | "apply_tweak" | "revert_tweak"
    /// | "create_backup" | "restore_backup" | "delete_backup" | "create_restore_point"
    pub action: String,
    /// "startup" | "tweak" | "backup"
    pub category: String,
    /// Human-readable name of the affected entity
    pub target: String,
    pub success: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BackupOpResult {
    pub success: bool,
    pub message: String,
    pub data: Option<BackupEntry>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RestorePointStatus {
    pub enabled: bool,
    pub message: String,
}

// â”€â”€ File-system helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Returns `%APPDATA%\VOptimizer`
pub fn get_app_data_dir() -> PathBuf {
    std::env::var("APPDATA")
        .map(|d| PathBuf::from(d).join("VOptimizer"))
        .unwrap_or_else(|_| PathBuf::from("VOptimizer"))
}

/// Serialises `data` as pretty JSON and writes it to `file`,
/// creating any missing parent directories.
pub fn write_json_metadata<T: Serialize>(file: &Path, data: &T) -> std::io::Result<()> {
    let content = serde_json::to_string_pretty(data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(file, content)
}

/// Reads and deserialises JSON from `file`; returns `T::default()` on any error.
pub fn read_json_metadata<T>(file: &Path) -> T
where
    T: for<'de> Deserialize<'de> + Default,
{
    std::fs::read_to_string(file)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

// â”€â”€ History â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn history_file() -> PathBuf {
    get_app_data_dir().join("history.json")
}

/// Prepends `entry` to the persistent history log (capped at 500 entries).
pub fn record_history(entry: HistoryEntry) {
    let file = history_file();
    let mut history: Vec<HistoryEntry> = read_json_metadata(&file);
    history.insert(0, entry);
    history.truncate(500);
    write_json_metadata(&file, &history).ok();
}

pub fn list_history_impl() -> Vec<HistoryEntry> {
    read_json_metadata(&history_file())
}

pub fn clear_history_impl() -> BackupOpResult {
    let file = history_file();
    match write_json_metadata(&file, &Vec::<HistoryEntry>::new()) {
        Ok(_) => BackupOpResult {
            success: true,
            message: "History cleared".to_string(),
            data: None,
            error: None,
        },
        Err(e) => BackupOpResult {
            success: false,
            message: "Failed to clear history".to_string(),
            data: None,
            error: Some(e.to_string()),
        },
    }
}

// â”€â”€ Registry backup â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn backups_file() -> PathBuf {
    get_app_data_dir().join("backups.json")
}

fn backups_dir() -> PathBuf {
    get_app_data_dir().join("Backups")
}

fn validate_backup_file_path_in(base_dir: &Path, file_path: &Path) -> Result<PathBuf, String> {
    if !file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("reg"))
        .unwrap_or(false)
    {
        return Err("Backup path must point to a .reg file".to_string());
    }

    let base = base_dir
        .canonicalize()
        .map_err(|e| format!("Cannot resolve backups directory: {}", e))?;
    let file = file_path
        .canonicalize()
        .map_err(|e| format!("Cannot resolve backup file: {}", e))?;

    if !file.starts_with(&base) {
        return Err("Backup file is outside VOptimizer backups directory".to_string());
    }

    Ok(file)
}

fn validate_backup_file_path(file_path: &Path) -> Result<PathBuf, String> {
    validate_backup_file_path_in(&backups_dir(), file_path)
}

/// Returns `%SystemRoot%\System32\reg.exe`
fn reg_exe() -> PathBuf {
    std::env::var("SystemRoot")
        .map(|r| PathBuf::from(r).join("System32").join("reg.exe"))
        .unwrap_or_else(|_| PathBuf::from("reg.exe"))
}

pub fn list_backups_impl() -> Vec<BackupEntry> {
    read_json_metadata(&backups_file())
}

pub fn create_registry_backup_impl(label: String, registry_key: String) -> BackupOpResult {
    let dir = backups_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return BackupOpResult {
            success: false,
            message: format!("Cannot create backups directory: {}", e),
            data: None,
            error: Some(e.to_string()),
        };
    }

    let id = new_id();
    let file_path = dir.join(format!("{}.reg", id));
    let file_path_str = file_path.to_string_lossy().to_string();

    // reg.exe export "<key>" "<file>" /y
    let output = no_window_cmd(reg_exe())
        .args(["export", &registry_key, &file_path_str, "/y"])
        .output();

    match output {
        Err(e) => BackupOpResult {
            success: false,
            message: "Could not run reg.exe".to_string(),
            data: None,
            error: Some(e.to_string()),
        },
        Ok(o) if !o.status.success() => {
            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let hint = if !is_hkcu_key(&registry_key) {
                " (HKLM keys require administrator privileges)"
            } else {
                ""
            };
            BackupOpResult {
                success: false,
                message: format!("reg.exe export failed{}", hint),
                data: None,
                error: Some(if !stderr.is_empty() { stderr } else { stdout }),
            }
        }
        Ok(_) => {
            let size_bytes = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);

            let entry = BackupEntry {
                id,
                timestamp: now_ms(),
                label,
                registry_key,
                file_path: file_path_str,
                size_bytes,
            };

            // Persist metadata
            let mut backups = list_backups_impl();
            backups.insert(0, entry.clone());
            write_json_metadata(&backups_file(), &backups).ok();

            BackupOpResult {
                success: true,
                message: format!(
                    "Backup '{}' created ({} bytes)",
                    entry.label, entry.size_bytes
                ),
                data: Some(entry),
                error: None,
            }
        }
    }
}

pub fn restore_registry_file_impl(id: String) -> BackupOpResult {
    let backups = list_backups_impl();
    let entry = match backups.iter().find(|b| b.id == id) {
        Some(e) => e.clone(),
        None => {
            return BackupOpResult {
                success: false,
                message: format!("Backup '{}' not found", id),
                data: None,
                error: Some("Backup entry not found in metadata".to_string()),
            }
        }
    };

    let backup_file = Path::new(&entry.file_path);
    if !backup_file.exists() {
        return BackupOpResult {
            success: false,
            message: format!("Backup file not found on disk: {}", entry.file_path),
            data: None,
            error: Some("File missing from disk â€” it may have been moved or deleted".to_string()),
        };
    }

    let safe_file = match validate_backup_file_path(backup_file) {
        Ok(path) => path,
        Err(e) => {
            return BackupOpResult {
                success: false,
                message: "Backup file failed safety validation".to_string(),
                data: None,
                error: Some(e),
            }
        }
    };
    let safe_file_str = safe_file.to_string_lossy().to_string();

    // reg.exe import "<file>"
    let output = no_window_cmd(reg_exe())
        .args(["import", &safe_file_str])
        .output();

    match output {
        Err(e) => BackupOpResult {
            success: false,
            message: "Could not run reg.exe".to_string(),
            data: None,
            error: Some(e.to_string()),
        },
        Ok(o) if !o.status.success() => {
            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
            let hint = if !is_hkcu_key(&entry.registry_key) {
                " (HKLM restores require administrator privileges)"
            } else {
                ""
            };
            BackupOpResult {
                success: false,
                message: format!("reg.exe import failed{}", hint),
                data: None,
                error: Some(stderr),
            }
        }
        Ok(_) => BackupOpResult {
            success: true,
            message: format!("Backup '{}' restored successfully", entry.label),
            data: Some(entry),
            error: None,
        },
    }
}

pub fn delete_backup_impl(id: String) -> BackupOpResult {
    let mut backups = list_backups_impl();
    let pos = match backups.iter().position(|b| b.id == id) {
        Some(p) => p,
        None => {
            return BackupOpResult {
                success: false,
                message: format!("Backup '{}' not found", id),
                data: None,
                error: Some("Not found in metadata".to_string()),
            }
        }
    };

    let entry = backups.remove(pos);
    let delete_warning = match validate_backup_file_path(Path::new(&entry.file_path)) {
        Ok(path) => std::fs::remove_file(&path).err().map(|e| {
            format!(
                "Metadata removed, but backup file could not be deleted: {}",
                e
            )
        }),
        Err(e) => Some(format!("Metadata removed; skipped file deletion: {}", e)),
    };

    match write_json_metadata(&backups_file(), &backups) {
        Ok(_) => BackupOpResult {
            success: true,
            message: delete_warning.unwrap_or_else(|| format!("Backup '{}' deleted", entry.label)),
            data: None,
            error: None,
        },
        Err(e) => BackupOpResult {
            success: false,
            message: "Backup file removed but could not update metadata".to_string(),
            data: None,
            error: Some(e.to_string()),
        },
    }
}

fn is_hkcu_key(key: &str) -> bool {
    let k = key.to_uppercase();
    k.starts_with("HKCU") || k.starts_with("HKEY_CURRENT_USER")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("voptimizer-{}-{}", name, new_id()))
    }

    #[test]
    fn backup_path_validation_accepts_reg_inside_backup_dir() {
        let dir = unique_temp_dir("backup-path-inside");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("safe.reg");
        std::fs::write(&file, "Windows Registry Editor Version 5.00").unwrap();

        let validated = validate_backup_file_path_in(&dir, &file).unwrap();
        assert_eq!(validated, file.canonicalize().unwrap());

        let _ = std::fs::remove_file(file);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn backup_path_validation_rejects_files_outside_backup_dir() {
        let dir = unique_temp_dir("backup-path-base");
        let outside_dir = unique_temp_dir("backup-path-outside");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(&outside_dir).unwrap();
        let file = outside_dir.join("unsafe.reg");
        std::fs::write(&file, "Windows Registry Editor Version 5.00").unwrap();

        let err = validate_backup_file_path_in(&dir, &file).unwrap_err();
        assert!(err.contains("outside VOptimizer backups directory"));

        let _ = std::fs::remove_file(file);
        let _ = std::fs::remove_dir_all(outside_dir);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn backup_path_validation_rejects_non_reg_files() {
        let dir = unique_temp_dir("backup-path-extension");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("not-a-backup.txt");
        std::fs::write(&file, "not a registry file").unwrap();

        let err = validate_backup_file_path_in(&dir, &file).unwrap_err();
        assert!(err.contains(".reg"));

        let _ = std::fs::remove_file(file);
        let _ = std::fs::remove_dir_all(dir);
    }
}

// â”€â”€ Restore points â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub fn check_restore_point_status_impl() -> RestorePointStatus {
    #[cfg(not(windows))]
    return RestorePointStatus {
        enabled: false,
        message: "System Restore is only available on Windows".to_string(),
    };

    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        // Windows 10/11: per-drive disable is stored under SystemRestore\Monitored Drives
        // A simple proxy: check if the VSS (Volume Shadow Copy) service registry key exists
        // and whether a key that task manager uses is present.
        let sr_key = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\SystemRestore");

        let enabled = match sr_key {
            Ok(k) => {
                // DisableSR == 1 means disabled
                let disabled: u32 = k.get_value("DisableSR").unwrap_or(0);
                disabled == 0
            }
            Err(_) => false,
        };

        RestorePointStatus {
            enabled,
            message: if enabled {
                "System Restore appears to be enabled. Creating a restore point requires administrator privileges.".to_string()
            } else {
                "System Restore is disabled or unavailable on this system. Enable it in System Properties â†’ System Protection before using this feature.".to_string()
            },
        }
    }
}

pub fn create_restore_point_impl(description: String) -> BackupOpResult {
    #[cfg(not(windows))]
    return BackupOpResult {
        success: false,
        message: "System Restore is only available on Windows".to_string(),
        data: None,
        error: Some("Windows only".to_string()),
    };

    #[cfg(windows)]
    {
        let status = check_restore_point_status_impl();
        if !status.enabled {
            return BackupOpResult {
                success: false,
                message: "System Restore is not enabled on this system".to_string(),
                data: None,
                error: Some(status.message),
            };
        }

        // Escape single quotes in description for PowerShell
        let safe_desc = description.replace('\'', "''");
        let ps_cmd = format!(
            "Checkpoint-Computer -Description '{}' -RestorePointType 'MODIFY_SETTINGS'",
            safe_desc
        );

        let ps_exe = std::env::var("SystemRoot")
            .map(|r| {
                PathBuf::from(r)
                    .join("System32")
                    .join("WindowsPowerShell")
                    .join("v1.0")
                    .join("powershell.exe")
            })
            .unwrap_or_else(|_| PathBuf::from("powershell.exe"));

        let output = no_window_cmd(&ps_exe)
            .args(["-NonInteractive", "-NoProfile", "-Command", &ps_cmd])
            .output();

        match output {
            Err(e) => BackupOpResult {
                success: false,
                message: "Could not start PowerShell".to_string(),
                data: None,
                error: Some(e.to_string()),
            },
            Ok(o) if !o.status.success() => {
                let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
                BackupOpResult {
                    success: false,
                    message:
                        "Failed to create restore point â€” administrator privileges are required"
                            .to_string(),
                    data: None,
                    error: Some(if stderr.is_empty() {
                        format!("Exit code: {:?}", o.status.code())
                    } else {
                        stderr
                    }),
                }
            }
            Ok(_) => BackupOpResult {
                success: true,
                message: format!("Restore point '{}' created successfully", description),
                data: None,
                error: None,
            },
        }
    }
}
