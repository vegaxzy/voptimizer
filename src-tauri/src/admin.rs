use crate::util::no_window_cmd;

fn sys_root() -> String {
    std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string())
}

fn ps_exe() -> String {
    format!(
        "{}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
        sys_root()
    )
}

pub fn is_admin_impl() -> bool {
    let script = "([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)";
    no_window_cmd(ps_exe())
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_lowercase() == "true")
        .unwrap_or(false)
}

pub fn restart_as_admin_impl() -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .replace('\'', "''");

    let script = format!(
        "try {{ Start-Process -FilePath '{}' -Verb RunAs -ErrorAction Stop; exit 0 }} catch {{ exit 1 }}",
        exe
    );

    let status = no_window_cmd(ps_exe())
        .args(["-NonInteractive", "-NoProfile", "-Command", &script])
        .status()
        .map_err(|e| e.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err("Elevation cancelled or failed".to_string())
    }
}
