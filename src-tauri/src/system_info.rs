//! Real system information gathered from Windows registry, WMI, and environment.
//! All values come from the actual hardware — never hardcoded or guessed.
//! If a value cannot be read, "Unknown" / 0 is returned; we never fabricate data.

use crate::util::no_window_cmd;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Data structures ──────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DriverEntry {
    pub name: String,
    pub version: String,
    pub date: String,
    pub status: String, // "ok" | "outdated" | "unknown"
    pub note: String,
}

/// Flat structure — every field is a primitive so Tauri serialises it cleanly.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemOverview {
    // ── Real-time usage ──────────────────────────────────────────
    pub cpu_pct: u32,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub disk_used_gb: f64,
    pub disk_total_gb: f64,
    pub uptime_secs: u64,

    // ── CPU ──────────────────────────────────────────────────────
    pub cpu_name: String,
    pub cpu_cores: u32,
    pub cpu_threads: u32,

    // ── GPU ──────────────────────────────────────────────────────
    pub gpu_name: String,
    pub gpu_vram_gb: f64,
    pub gpu_driver_version: String,

    // ── RAM ──────────────────────────────────────────────────────
    pub ram_type: String,
    pub ram_speed_mhz: u32,

    // ── Motherboard ───────────────────────────────────────────────
    pub motherboard: String,

    // ── Storage (system drive) ────────────────────────────────────
    pub storage_name: String,       // physical disk model / FriendlyName
    pub storage_partition: String,  // system partition, e.g. "C:"
    pub storage_gb: f64,            // physical disk capacity
    pub storage_free_gb: f64,       // free space on the system partition
    pub storage_media_type: String, // "SSD" | "HDD" | "Unknown"
    pub storage_bus_type: String,   // "NVMe" | "SATA" | "Unknown"
    pub storage_type: String,       // combined label, e.g. "NVMe SSD"
    pub storage_health: String,     // "Healthy" | "Warning" | "Unknown"

    // ── Operating System ──────────────────────────────────────────
    pub os_name: String,
    pub os_build: String,
    pub os_version_tag: String,
    pub os_install_date: String,
    pub os_architecture: String,
    pub os_locale: String,
    pub os_hostname: String,

    // ── BIOS / UEFI ───────────────────────────────────────────────
    pub bios_vendor: String,
    pub bios_version: String,
    pub bios_release_date: String,
    pub bios_mode: String,
    pub bios_secure_boot: bool,
    pub bios_age_days: u32,

    // ── Drivers ───────────────────────────────────────────────────
    pub drivers: Vec<DriverEntry>,
}

impl Default for SystemOverview {
    fn default() -> Self {
        let u = || "Unknown".to_string();
        Self {
            cpu_pct: 0,
            ram_used_mb: 0,
            ram_total_mb: 0,
            disk_used_gb: 0.0,
            disk_total_gb: 0.0,
            uptime_secs: 0,
            cpu_name: u(),
            cpu_cores: 0,
            cpu_threads: 0,
            gpu_name: u(),
            gpu_vram_gb: 0.0,
            gpu_driver_version: u(),
            ram_type: u(),
            ram_speed_mhz: 0,
            motherboard: u(),
            storage_name: u(),
            storage_partition: u(),
            storage_gb: 0.0,
            storage_free_gb: 0.0,
            storage_media_type: u(),
            storage_bus_type: u(),
            storage_type: u(),
            storage_health: u(),
            os_name: u(),
            os_build: u(),
            os_version_tag: u(),
            os_install_date: u(),
            os_architecture: u(),
            os_locale: u(),
            os_hostname: u(),
            bios_vendor: u(),
            bios_version: u(),
            bios_release_date: u(),
            bios_mode: u(),
            bios_secure_boot: false,
            bios_age_days: 0,
            drivers: vec![],
        }
    }
}

// ── Registry helpers ─────────────────────────────────────────────────────────

fn reg_str(subkey: &str, value: &str) -> Option<String> {
    use winreg::enums::*;
    use winreg::RegKey;
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(subkey)
        .ok()
        .and_then(|k| k.get_value::<String, _>(value).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn reg_u32(subkey: &str, value: &str) -> Option<u32> {
    use winreg::enums::*;
    use winreg::RegKey;
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(subkey)
        .ok()
        .and_then(|k| k.get_value::<u32, _>(value).ok())
}

fn reg_key_exists(subkey: &str) -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(subkey)
        .is_ok()
}

// ── GPU: name + VRAM from driver class registry ───────────────────────────────
// Uses the 64-bit qwMemorySize value so >4 GB VRAM is reported correctly.

fn gpu_info_from_registry() -> (String, Option<u64>) {
    use winreg::enums::*;
    use winreg::RegKey;
    let path =
        "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}";
    if let Ok(class_key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(path) {
        for i in 0..=4u32 {
            let sub = format!("{:04}", i);
            if let Ok(dev_key) = class_key.open_subkey(&sub) {
                let name: String = dev_key.get_value("DriverDesc").unwrap_or_default();
                if name.is_empty() || name.to_ascii_lowercase().contains("base video") {
                    continue;
                }
                // 64-bit VRAM (accurate for any capacity)
                let vram = dev_key
                    .get_value::<u64, _>("HardwareInformation.qwMemorySize")
                    .ok()
                    .filter(|&v| v > 0)
                    .or_else(|| {
                        dev_key
                            .get_value::<u32, _>("HardwareInformation.MemorySize")
                            .ok()
                            .filter(|&v| v > 0)
                            .map(|v| v as u64)
                    });
                return (name.trim().to_string(), vram);
            }
        }
    }
    ("Unknown GPU".to_string(), None)
}

// ── Date / time helpers ──────────────────────────────────────────────────────

/// Compute Julian Day Number — handles the Gregorian calendar correctly.
fn jdn(y: i64, m: i64, d: i64) -> i64 {
    (1461 * (y + 4800 + (m - 14) / 12)) / 4
        + (367 * (m - 2 - 12 * ((m - 14) / 12))) / 12
        - (3 * ((y + 4900 + (m - 14) / 12) / 100)) / 4
        + d
        - 32075
}

/// Number of days that have elapsed since `date_str` (format: "YYYY-MM-DD").
fn days_since(date_str: &str) -> u32 {
    let parts: Vec<i64> = date_str
        .split('-')
        .filter_map(|p| p.parse().ok())
        .collect();
    if parts.len() < 3 {
        return 0;
    }
    let target_jdn = jdn(parts[0], parts[1], parts[2]);

    use std::time::{SystemTime, UNIX_EPOCH};
    let today_days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_secs() / 86400) as i64)
        .unwrap_or(0);
    let today_jdn = 2_440_588 + today_days; // Unix epoch (1970-01-01) = JDN 2440588

    (today_jdn - target_jdn).max(0) as u32
}

/// Convert "YYYY-MM-DD" to "Mon D, YYYY" (e.g. "Jun 15, 2022").
fn fmt_date(date_str: &str) -> String {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() < 3 {
        return date_str.to_string();
    }
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun",
        "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mi: usize = parts[1]
        .parse::<usize>()
        .unwrap_or(1)
        .saturating_sub(1)
        .min(11);
    // Trim leading zero from day
    let day = parts[2].trim_start_matches('0');
    format!("{} {}, {}", MONTHS[mi], day, parts[0])
}

/// Convert a Unix timestamp (seconds since epoch) to "Mon D, YYYY".
fn unix_ts_to_date(ts: u64) -> String {
    // Days since epoch → JDN
    let jdn_val = (2_440_588 + (ts / 86400)) as i32;

    // Gregorian calendar algorithm (Richards, 2013)
    let a = jdn_val + 32044;
    let b = (4 * a + 3) / 146097;
    let c = a - (146097 * b) / 4;
    let d = (4 * c + 3) / 1461;
    let e = c - (1461 * d) / 4;
    let m = (5 * e + 2) / 153;

    let day   = e - (153 * m + 2) / 5 + 1;
    let month = m + 3 - 12 * (m / 10);
    let year  = 100 * b + d - 4800 + m / 10;

    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun",
        "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mi = ((month - 1) as usize).min(11);
    format!("{} {}, {}", MONTHS[mi], day, year)
}

// ── WMI value decoders ───────────────────────────────────────────────────────

fn ram_type_str(code: u32) -> &'static str {
    match code {
        20 => "DDR",
        21 => "DDR2",
        22 => "DDR2 FB-DIMM",
        24 => "DDR3",
        26 => "DDR4",
        34 => "DDR5",
        _  => "Unknown",
    }
}

/// Decode an MSFT_PhysicalDisk.MediaType value into a friendly string.
/// `Get-PhysicalDisk` usually already returns the friendly name ("SSD"/"HDD"),
/// but on some systems the raw numeric code comes through — handle both.
fn normalize_media_type(raw: &str) -> String {
    match raw.trim() {
        "3" => "HDD".to_string(),
        "4" => "SSD".to_string(),
        "5" => "SCM".to_string(), // Storage Class Memory
        "" | "0" | "Unspecified" => "Unspecified".to_string(),
        other => other.to_string(), // already friendly ("SSD", "HDD", …)
    }
}

/// Decode an MSFT_PhysicalDisk.BusType value into a friendly string.
fn normalize_bus_type(raw: &str) -> String {
    match raw.trim() {
        "1"  => "SCSI".to_string(),
        "2"  => "ATAPI".to_string(),
        "3"  => "ATA".to_string(),
        "7"  => "USB".to_string(),
        "8"  => "RAID".to_string(),
        "9"  => "iSCSI".to_string(),
        "10" => "SAS".to_string(),
        "11" => "SATA".to_string(),
        "12" => "SD".to_string(),
        "13" => "MMC".to_string(),
        "17" => "NVMe".to_string(),
        "" | "0" | "Unknown" => "Unknown".to_string(),
        other => other.to_string(), // already friendly ("NVMe", "SATA", …)
    }
}

/// Classify the system drive STRICTLY from its real MediaType + BusType.
/// We never infer from the model string. If the data is inconclusive we return
/// "Unknown" rather than guessing — inaccurate diagnostics are worse than none.
///
/// NVMe is authoritative: an NVMe bus is always solid-state, so even when the
/// MediaType is "Unspecified" we can safely report "NVMe SSD".
fn classify_storage(media_type: &str, bus_type: &str) -> String {
    let mt = media_type.trim().to_ascii_lowercase();
    let bt = bus_type.trim().to_ascii_lowercase();

    let is_nvme = bt == "nvme";
    let is_sata = bt == "sata" || bt == "ata";
    let is_usb  = bt == "usb";

    match mt.as_str() {
        "ssd" => {
            if is_nvme {
                "NVMe SSD".to_string()
            } else if is_sata {
                "SATA SSD".to_string()
            } else if is_usb {
                "External SSD".to_string()
            } else {
                "SSD".to_string()
            }
        }
        "hdd" => {
            if is_sata {
                "SATA HDD".to_string()
            } else if is_usb {
                "External HDD".to_string()
            } else {
                "HDD".to_string()
            }
        }
        "scm" => "Storage Class Memory".to_string(),
        // MediaType unspecified/unknown — only NVMe is reliable enough to assert
        _ => {
            if is_nvme {
                "NVMe SSD".to_string()
            } else {
                "Unknown".to_string()
            }
        }
    }
}

// ── PowerShell path ───────────────────────────────────────────────────────────

fn ps_exe() -> String {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
    format!(
        "{}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
        root
    )
}

// ── Parse PowerShell KEY=VALUE output ────────────────────────────────────────

fn parse_ps_output(stdout: &[u8]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let text = String::from_utf8_lossy(stdout);
    for line in text.lines() {
        let line = line.trim();
        if let Some(idx) = line.find('=') {
            let key = line[..idx].trim().to_string();
            let val = line[idx + 1..].trim().to_string();
            if !val.is_empty() {
                map.insert(key, val);
            }
        }
    }
    map
}

// ═════════════════════════════════════════════════════════════════════════════
//  LIVE METRICS — fast, accurate, locale-independent (Win32 FFI, no subprocess)
//
//  CPU usage is measured the way Task Manager does it: two GetSystemTimes samples
//  and the busy/total delta — NOT Win32_Processor.LoadPercentage, which is a laggy
//  WMI snapshot that does not match Task Manager.
// ═════════════════════════════════════════════════════════════════════════════

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SystemLive {
    pub cpu_pct: u32,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub disk_used_gb: f64,
    pub disk_total_gb: f64,
    pub uptime_secs: u64,
}

#[cfg(windows)]
mod winffi {
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct FILETIME {
        pub low: u32,
        pub high: u32,
    }
    impl FILETIME {
        pub fn as_u64(&self) -> u64 {
            ((self.high as u64) << 32) | self.low as u64
        }
    }

    #[repr(C)]
    pub struct MEMORYSTATUSEX {
        pub length: u32,
        pub memory_load: u32,
        pub total_phys: u64,
        pub avail_phys: u64,
        pub total_pagefile: u64,
        pub avail_pagefile: u64,
        pub total_virtual: u64,
        pub avail_virtual: u64,
        pub avail_ext_virtual: u64,
    }

    #[link(name = "kernel32")]
    extern "system" {
        pub fn GetSystemTimes(
            idle: *mut FILETIME,
            kernel: *mut FILETIME,
            user: *mut FILETIME,
        ) -> i32;
        pub fn GlobalMemoryStatusEx(buf: *mut MEMORYSTATUSEX) -> i32;
        pub fn GetTickCount64() -> u64;
        pub fn GetDiskFreeSpaceExW(
            dir: *const u16,
            free_avail: *mut u64,
            total: *mut u64,
            total_free: *mut u64,
        ) -> i32;
    }
}

#[cfg(windows)]
fn read_system_times() -> Option<(u64, u64, u64)> {
    use winffi::*;
    let mut idle = FILETIME::default();
    let mut kern = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: all three out-pointers are valid, initialised FILETIME structs.
    let ok = unsafe { GetSystemTimes(&mut idle, &mut kern, &mut user) };
    if ok == 0 {
        return None;
    }
    Some((idle.as_u64(), kern.as_u64(), user.as_u64()))
}

/// CPU usage % over a short sampling window. `kernel` time already includes idle,
/// so total = kernel + user and busy = total − idle.
#[cfg(windows)]
fn cpu_usage_percent() -> u32 {
    let (i1, k1, u1) = match read_system_times() {
        Some(v) => v,
        None => return 0,
    };
    std::thread::sleep(std::time::Duration::from_millis(350));
    let (i2, k2, u2) = match read_system_times() {
        Some(v) => v,
        None => return 0,
    };
    let idle = i2.saturating_sub(i1);
    let total = k2.saturating_sub(k1) + u2.saturating_sub(u1);
    if total == 0 {
        return 0;
    }
    let busy = total.saturating_sub(idle);
    ((busy as f64 / total as f64) * 100.0).round().clamp(0.0, 100.0) as u32
}

#[cfg(windows)]
fn mem_used_total_mb() -> (u64, u64) {
    use winffi::*;
    // SAFETY: zero-init then set the required `length` before the call.
    let mut m: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    m.length = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    let ok = unsafe { GlobalMemoryStatusEx(&mut m) };
    if ok == 0 {
        return (0, 0);
    }
    const MB: u64 = 1024 * 1024;
    let total = m.total_phys / MB;
    let used = m.total_phys.saturating_sub(m.avail_phys) / MB;
    (used, total)
}

#[cfg(windows)]
fn uptime_secs_ffi() -> u64 {
    // SAFETY: no arguments, returns a scalar.
    unsafe { winffi::GetTickCount64() / 1000 }
}

#[cfg(windows)]
fn disk_usage_gb(root: &str) -> (f64, f64) {
    use winffi::*;
    let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
    let mut free_avail = 0u64;
    let mut total = 0u64;
    let mut total_free = 0u64;
    // SAFETY: `wide` is a NUL-terminated UTF-16 path; out-pointers are valid.
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free_avail, &mut total, &mut total_free) };
    if ok == 0 {
        return (0.0, 0.0);
    }
    let gb = |b: u64| (b as f64 / 1024.0_f64.powi(3) * 10.0).round() / 10.0;
    (gb(total.saturating_sub(total_free)), gb(total))
}

pub fn get_system_live_impl() -> SystemLive {
    #[cfg(windows)]
    {
        let (ram_used_mb, ram_total_mb) = mem_used_total_mb();
        let sys_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
        let (disk_used_gb, disk_total_gb) = disk_usage_gb(&format!("{}\\", sys_drive));
        SystemLive {
            cpu_pct: cpu_usage_percent(),
            ram_used_mb,
            ram_total_mb,
            disk_used_gb,
            disk_total_gb,
            uptime_secs: uptime_secs_ffi(),
        }
    }
    #[cfg(not(windows))]
    {
        SystemLive::default()
    }
}

// ── Static system info (slow WMI/registry — fetch once, cache aggressively) ───

pub fn get_system_static_impl() -> SystemOverview {
    let mut r = SystemOverview::default();

    // ────────────────────────────────────────────────────────────────────────
    // 1.  Fast registry reads  (no subprocess — runs in microseconds)
    // ────────────────────────────────────────────────────────────────────────

    // CPU name from BIOS/ACPI DMI data via Windows registry
    r.cpu_name = reg_str(
        "HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0",
        "ProcessorNameString",
    )
    .unwrap_or_else(|| "Unknown CPU".to_string());

    // GPU name + VRAM (64-bit) from driver class registry
    let (gpu_name_reg, gpu_vram_reg) = gpu_info_from_registry();
    r.gpu_name = gpu_name_reg;
    if let Some(vram_bytes) = gpu_vram_reg {
        r.gpu_vram_gb = (vram_bytes as f64 / (1024.0_f64.powi(3)) * 10.0).round() / 10.0;
    }

    // OS identity
    let nt = "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion";
    r.os_name = reg_str(nt, "ProductName").unwrap_or_else(|| "Windows".to_string());
    {
        let build = reg_str(nt, "CurrentBuildNumber").unwrap_or_default();
        let ubr   = reg_u32(nt, "UBR").map(|u| format!(".{}", u)).unwrap_or_default();
        r.os_build = format!("{}{}", build, ubr);
    }
    r.os_version_tag = reg_str(nt, "DisplayVersion")
        .or_else(|| reg_str(nt, "ReleaseId"))
        .unwrap_or_default();

    // Install date stored as a DWORD Unix timestamp
    r.os_install_date = reg_u32(nt, "InstallDate")
        .map(|ts| unix_ts_to_date(ts as u64))
        .unwrap_or_else(|| "Unknown".to_string());

    // Architecture from the process environment
    r.os_architecture = std::env::var("PROCESSOR_ARCHITECTURE")
        .map(|a| match a.to_ascii_uppercase().as_str() {
            "AMD64" => "x64 (AMD64)".to_string(),
            "ARM64" => "ARM64".to_string(),
            "X86"   => "x86 (32-bit)".to_string(),
            other   => other.to_string(),
        })
        .unwrap_or_else(|_| "Unknown".to_string());

    // Hostname
    r.os_hostname = std::env::var("COMPUTERNAME").unwrap_or_else(|_| {
        reg_str(
            "SYSTEM\\CurrentControlSet\\Control\\ComputerName\\ComputerName",
            "ComputerName",
        )
        .unwrap_or_else(|| "Unknown".to_string())
    });

    // BIOS firmware mode: UEFI = SecureBoot registry hive exists
    r.bios_mode = if reg_key_exists("SYSTEM\\CurrentControlSet\\Control\\SecureBoot") {
        "UEFI".to_string()
    } else {
        "Legacy BIOS".to_string()
    };

    // Secure Boot state
    r.bios_secure_boot = reg_u32(
        "SYSTEM\\CurrentControlSet\\Control\\SecureBoot\\State",
        "UEFISecureBootEnabled",
    )
    .map(|v| v == 1)
    .unwrap_or(false);

    // ────────────────────────────────────────────────────────────────────────
    // 2.  Single PowerShell WMI script  (one subprocess, ~1-3 s)
    //
    //     We deliberately skip Win32_PnPSignedDriver — on many systems it
    //     takes 15-30 s to enumerate, making the UI feel broken.
    //     GPU driver version comes from the faster Win32_VideoController.
    // ────────────────────────────────────────────────────────────────────────

    let script = r#"
$ErrorActionPreference = 'SilentlyContinue'
# CPU core/thread counts (static). Live CPU usage comes from GetSystemTimes FFI.
$cpu = Get-CimInstance Win32_Processor -Property NumberOfCores,NumberOfLogicalProcessors | Select-Object -First 1
Write-Output "CPU_CORES=$($cpu.NumberOfCores)"
Write-Output "CPU_THREADS=$($cpu.NumberOfLogicalProcessors)"
# OS — total RAM (for the hardware card) + install date. Live RAM usage / uptime come from FFI.
$os = Get-CimInstance Win32_OperatingSystem -Property TotalVisibleMemorySize,InstallDate
Write-Output "RAM_TOTAL_KB=$($os.TotalVisibleMemorySize)"
if ($os.InstallDate)    { Write-Output "OS_INSTALL=$($os.InstallDate.ToString('yyyy-MM-dd'))" }
Write-Output "LOCALE=$((Get-Culture).Name)"
# RAM modules — type code + speed
$ram = Get-CimInstance Win32_PhysicalMemory -Property SMBIOSMemoryType,Speed | Select-Object -First 1
Write-Output "RAM_TYPE_CODE=$($ram.SMBIOSMemoryType)"
Write-Output "RAM_SPEED=$($ram.Speed)"
# GPU — WMI name (cross-check with registry name), driver version
# AdapterRAM is 32-bit in WMI so VRAM already came from 64-bit registry key above
$gpu = Get-CimInstance Win32_VideoController -Property Name,AdapterRAM,DriverVersion | Where-Object { $_.AdapterRAM -gt 0 } | Sort-Object AdapterRAM -Descending | Select-Object -First 1
if ($gpu.Name)          { Write-Output "GPU_WMI_NAME=$($gpu.Name.Trim())" }
if ($gpu.DriverVersion) { Write-Output "GPU_DRIVER=$($gpu.DriverVersion)" }
# If registry VRAM read failed (no qwMemorySize key), fall back to WMI DWORD
Write-Output "GPU_VRAM_B=$($gpu.AdapterRAM)"
# Motherboard (baseboard)
$mb = Get-CimInstance Win32_BaseBoard -Property Manufacturer,Product | Select-Object -First 1
if ($mb.Manufacturer) { Write-Output "MB_MFR=$($mb.Manufacturer.Trim())" }
if ($mb.Product)      { Write-Output "MB_PRODUCT=$($mb.Product.Trim())" }
# BIOS
$bios = Get-CimInstance Win32_BIOS -Property Manufacturer,SMBIOSBIOSVersion,ReleaseDate | Select-Object -First 1
if ($bios.Manufacturer)      { Write-Output "BIOS_VENDOR=$($bios.Manufacturer.Trim())" }
if ($bios.SMBIOSBIOSVersion) { Write-Output "BIOS_VERSION=$($bios.SMBIOSBIOSVersion.Trim())" }
if ($bios.ReleaseDate)       { Write-Output "BIOS_DATE=$($bios.ReleaseDate.ToString('yyyy-MM-dd'))" }
# ── System drive resolution ──────────────────────────────────────────────
# Resolve the ACTUAL system partition, then the physical disk behind it.
$sysDrive  = $env:SystemDrive                         # e.g. "C:"
if (-not $sysDrive) {
    $sysDrive = (Get-CimInstance Win32_OperatingSystem -Property SystemDrive).SystemDrive
}
$sysLetter = $sysDrive.TrimEnd(':')                   # e.g. "C"
Write-Output "SYS_PARTITION=$sysDrive"
# System-partition logical-disk usage (used / free)
$ld = Get-CimInstance Win32_LogicalDisk -Filter "DeviceID='$sysDrive'" -Property Size,FreeSpace | Select-Object -First 1
if ($ld.Size)      { Write-Output "C_TOTAL_B=$($ld.Size)" }
if ($ld.FreeSpace) { Write-Output "C_FREE_B=$($ld.FreeSpace)" }
# Map system partition -> physical disk number
$diskNum = $null
$part = Get-Partition -DriveLetter $sysLetter -ErrorAction SilentlyContinue | Select-Object -First 1
if ($part) {
    $diskNum = $part.DiskNumber
    Write-Output "SYS_DISK_NUMBER=$diskNum"
}
if ($null -ne $diskNum) {
    # Physical disk → real MediaType / BusType / Health (no model-string guessing)
    $phys = Get-PhysicalDisk -ErrorAction SilentlyContinue | Where-Object { $_.DeviceId -eq "$diskNum" } | Select-Object -First 1
    if (-not $phys) {
        $phys = Get-Disk -Number $diskNum -ErrorAction SilentlyContinue | Get-PhysicalDisk -ErrorAction SilentlyContinue | Select-Object -First 1
    }
    if ($phys) {
        if ($phys.FriendlyName) { Write-Output "SYS_DISK_MODEL=$($phys.FriendlyName)" }
        Write-Output "SYS_MEDIA_TYPE=$($phys.MediaType)"
        Write-Output "SYS_BUS_TYPE=$($phys.BusType)"
        Write-Output "SYS_HEALTH=$($phys.HealthStatus)"
        if ($phys.Size) { Write-Output "SYS_DISK_SIZE_B=$($phys.Size)" }
    }
    # Model / size fallback from Get-Disk (covers systems without Storage providers)
    $gdisk = Get-Disk -Number $diskNum -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($gdisk) {
        if ($gdisk.FriendlyName) { Write-Output "SYS_DISK_MODEL2=$($gdisk.FriendlyName)" }
        if ($gdisk.Size)         { Write-Output "SYS_DISK_SIZE_B2=$($gdisk.Size)" }
    }
}
"#;

    let ps_result = no_window_cmd(ps_exe())
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output();

    if let Ok(out) = ps_result {
        let map = parse_ps_output(&out.stdout);

        let get     = |k: &str| -> String { map.get(k).cloned().unwrap_or_default() };
        let get_u64 = |k: &str| -> u64   { get(k).parse().unwrap_or(0) };
        let get_u32 = |k: &str| -> u32   { get(k).parse().unwrap_or(0) };

        // ── CPU (counts only — live usage comes from the live command) ────
        r.cpu_cores   = get_u32("CPU_CORES");
        r.cpu_threads = get_u32("CPU_THREADS");

        // ── RAM (total for the hardware card; live usage from the live command)
        let total_kb = get_u64("RAM_TOTAL_KB");
        r.ram_total_mb = total_kb / 1024;
        r.ram_type      = ram_type_str(get_u32("RAM_TYPE_CODE")).to_string();
        r.ram_speed_mhz = get_u32("RAM_SPEED");

        // ── Install date override (WMI DateTime is more accurate than registry DWORD) ──
        let wmi_install = get("OS_INSTALL");
        if !wmi_install.is_empty() {
            r.os_install_date = fmt_date(&wmi_install);
        }

        // ── Locale ───────────────────────────────────────────────────────
        let locale = get("LOCALE");
        if !locale.is_empty() {
            r.os_locale = locale;
        }

        // ── GPU ──────────────────────────────────────────────────────────
        // Prefer WMI name (often includes suffix like "SUPER") over registry name
        let wmi_gpu_name = get("GPU_WMI_NAME");
        if !wmi_gpu_name.is_empty() {
            r.gpu_name = wmi_gpu_name;
        }
        // Only fall back to WMI AdapterRAM if registry qwMemorySize was not found
        if r.gpu_vram_gb == 0.0 {
            let vram_b = get_u64("GPU_VRAM_B");
            // Sanity-check: AdapterRAM is 32-bit; 4294967295 means overflow (>4 GB)
            // In that case keep 0.0 — better than showing a garbage value
            if vram_b > 0 && vram_b < 4_294_967_295 {
                r.gpu_vram_gb =
                    (vram_b as f64 / 1024.0_f64.powi(3) * 10.0).round() / 10.0;
            }
        }
        let gpu_drv = get("GPU_DRIVER");
        if !gpu_drv.is_empty() {
            r.gpu_driver_version = gpu_drv;
        }

        // ── Motherboard ──────────────────────────────────────────────────
        let mb_mfr  = get("MB_MFR");
        let mb_prod = get("MB_PRODUCT");
        r.motherboard = match (mb_mfr.is_empty(), mb_prod.is_empty()) {
            (false, false) => format!("{} {}", mb_mfr, mb_prod),
            (false, true)  => mb_mfr,
            (true,  false) => mb_prod,
            _              => "Unknown".to_string(),
        };

        // ── BIOS ─────────────────────────────────────────────────────────
        let bios_vnd  = get("BIOS_VENDOR");
        let bios_ver  = get("BIOS_VERSION");
        let bios_date = get("BIOS_DATE");
        if !bios_vnd.is_empty() {
            r.bios_vendor = bios_vnd;
        }
        if !bios_ver.is_empty() {
            r.bios_version = bios_ver;
        }
        if !bios_date.is_empty() {
            r.bios_age_days     = days_since(&bios_date);
            r.bios_release_date = fmt_date(&bios_date);
        }

        // ── Storage: the system drive resolved from the system partition ──
        // System partition (e.g. "C:")
        let sys_partition = get("SYS_PARTITION");
        if !sys_partition.is_empty() {
            r.storage_partition = sys_partition;
        }

        // Physical-disk model — prefer Get-PhysicalDisk, fall back to Get-Disk
        let disk_model = {
            let m = get("SYS_DISK_MODEL");
            if !m.is_empty() { m } else { get("SYS_DISK_MODEL2") }
        };
        if !disk_model.is_empty() {
            r.storage_name = disk_model;
        }

        // Physical-disk capacity (Get-PhysicalDisk, fall back to Get-Disk)
        let disk_size_b = {
            let s = get_u64("SYS_DISK_SIZE_B");
            if s > 0 { s } else { get_u64("SYS_DISK_SIZE_B2") }
        };
        r.storage_gb = (disk_size_b as f64 / 1024.0_f64.powi(3) * 10.0).round() / 10.0;

        // Real MediaType + BusType from the Storage subsystem — never guessed
        let media_type = normalize_media_type(&get("SYS_MEDIA_TYPE"));
        let bus_type   = normalize_bus_type(&get("SYS_BUS_TYPE"));
        r.storage_media_type = if media_type == "Unspecified" {
            "Unknown".to_string()
        } else {
            media_type.clone()
        };
        r.storage_bus_type = bus_type.clone();
        r.storage_type     = classify_storage(&media_type, &bus_type);

        // Drive health (Healthy / Warning / Unhealthy / Unknown)
        let health = get("SYS_HEALTH");
        r.storage_health = if health.is_empty() {
            "Unknown".to_string()
        } else {
            health
        };

        // ── System-partition usage ───────────────────────────────────────
        let c_total = get_u64("C_TOTAL_B");
        let c_free  = get_u64("C_FREE_B");
        r.disk_total_gb = (c_total as f64 / 1024.0_f64.powi(3) * 10.0).round() / 10.0;
        r.disk_used_gb  =
            (c_total.saturating_sub(c_free) as f64 / 1024.0_f64.powi(3) * 10.0).round() / 10.0;
        r.storage_free_gb = (c_free as f64 / 1024.0_f64.powi(3) * 10.0).round() / 10.0;

        // ── Driver entries ───────────────────────────────────────────────
        // Only the GPU driver is queried here (Win32_PnPSignedDriver is too slow).
        // The version comes from Win32_VideoController.DriverVersion.
        let mut drivers = vec![];
        if !r.gpu_driver_version.is_empty() && r.gpu_driver_version != "Unknown" {
            drivers.push(DriverEntry {
                name:    r.gpu_name.clone(),
                version: r.gpu_driver_version.clone(),
                date:    String::new(),
                status:  "ok".to_string(),
                note:    String::new(),
            });
        }
        r.drivers = drivers;
    }

    r
}

/// Backward-compatible combined snapshot (static + live). Kept so the original
/// `get_system_overview` command still works; the UI now prefers the split
/// static/live commands so it can cache the slow part and poll the fast part.
pub fn get_system_overview_impl() -> SystemOverview {
    let mut s = get_system_static_impl();
    let live = get_system_live_impl();
    s.cpu_pct = live.cpu_pct;
    s.ram_used_mb = live.ram_used_mb;
    if live.ram_total_mb > 0 {
        s.ram_total_mb = live.ram_total_mb;
    }
    s.disk_used_gb = live.disk_used_gb;
    if live.disk_total_gb > 0.0 {
        s.disk_total_gb = live.disk_total_gb;
    }
    s.uptime_secs = live.uptime_secs;
    s
}
