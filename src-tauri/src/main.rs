// Prevents an additional console window on Windows, including elevated dev runs.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    v_optimizer_lib::run()
}
