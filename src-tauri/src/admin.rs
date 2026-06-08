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
    restart_as_admin_native()
}

#[cfg(target_os = "windows")]
fn restart_as_admin_native() -> Result<(), String> {
    use std::iter;
    use std::os::windows::ffi::OsStrExt;

    const SW_SHOWNORMAL: i32 = 1;

    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteW(
            hwnd: *mut std::ffi::c_void,
            lp_operation: *const u16,
            lp_file: *const u16,
            lp_parameters: *const u16,
            lp_directory: *const u16,
            n_show_cmd: i32,
        ) -> isize;
    }

    fn wide(s: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
        s.as_ref().encode_wide().chain(iter::once(0)).collect()
    }

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let op = wide("runas");
    let file = wide(exe.as_os_str());
    let dir = exe
        .parent()
        .map(wide)
        .unwrap_or_else(|| wide(std::ffi::OsStr::new("")));

    // SAFETY: ShellExecuteW is called with valid null-terminated UTF-16 buffers
    // that live for the duration of the call. Null parameters mean no extra args.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            op.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            dir.as_ptr(),
            SW_SHOWNORMAL,
        )
    };

    if result > 32 {
        Ok(())
    } else {
        Err(format!(
            "Elevation cancelled or failed (ShellExecuteW code {})",
            result
        ))
    }
}

#[cfg(not(target_os = "windows"))]
fn restart_as_admin_native() -> Result<(), String> {
    Err("Elevation is only supported on Windows".to_string())
}
