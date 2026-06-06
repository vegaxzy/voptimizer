use std::ffi::OsStr;
use std::process::Command;

/// `CREATE_NO_WINDOW` — prevents a visible console window from flashing
/// when spawning PowerShell / reg.exe / sc.exe / powercfg.exe etc.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Identical to `Command::new(program)` but with `CREATE_NO_WINDOW` set so
/// that no console window appears on screen when the child process starts.
pub(crate) fn no_window_cmd<S: AsRef<OsStr>>(program: S) -> Command {
    let mut cmd = Command::new(program);
    // SAFETY: this is a standard Windows process-creation flag, always valid.
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}
