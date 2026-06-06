use crate::tweaks_impl::{self, TweakOpResult};
use crate::util::no_window_cmd;
use serde::{Deserialize, Serialize};

// â”€â”€ Structs â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SystemInfo {
    pub cpu_name: String,
    pub gpu_name: String,
    pub gpu_vendor: String, // "nvidia" | "amd" | "intel" | "unknown"
    pub ram_total_mb: u64,
    pub ram_used_mb: u64,
    pub power_plan_name: String,
    pub power_plan_guid: String,
    pub gamedvr_enabled: bool,
    pub game_bar_capture_enabled: bool,
    pub animations_enabled: bool,
    pub edge_background_enabled: bool,
    pub edge_startup_boost_enabled: bool,
    pub startup_app_count: usize,
    pub minecraft_running: bool,
    pub nvidia_detected: bool,
    pub amd_detected: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub memory_mb: f64,
    pub is_safe_to_kill: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PingResult {
    pub success: bool,
    pub host: String,
    pub latency_ms: Option<u32>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PresetResult {
    pub tweak_id: String,
    pub success: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct DnsInfo {
    pub servers: Vec<String>,
    pub hostname: String,
}

// â”€â”€ Path helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn sys_root() -> String {
    std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string())
}
fn ps_exe() -> String {
    format!(
        "{}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
        sys_root()
    )
}

// â”€â”€ System information â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn get_cpu_name() -> String {
    use winreg::enums::*;
    use winreg::RegKey;
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey("HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0")
        .ok()
        .and_then(|k| k.get_value::<String, _>("ProcessorNameString").ok())
        .unwrap_or_else(|| "Unknown CPU".to_string())
        .trim()
        .to_string()
}

fn get_gpu_info() -> (String, String) {
    use winreg::enums::*;
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let class_path =
        "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}";
    if let Ok(class_key) = hklm.open_subkey(class_path) {
        for i in 0..=4u32 {
            let sub = format!("{:04}", i);
            if let Ok(dev_key) = class_key.open_subkey(&sub) {
                let name: String = dev_key.get_value("DriverDesc").unwrap_or_default();
                if name.is_empty() || name.to_lowercase().contains("base video") {
                    continue;
                }
                let nl = name.to_lowercase();
                let vendor = if nl.contains("nvidia") {
                    "nvidia"
                } else if nl.contains("amd") || nl.contains("radeon") || nl.contains("ati") {
                    "amd"
                } else if nl.contains("intel") {
                    "intel"
                } else {
                    "unknown"
                };
                return (name, vendor.to_string());
            }
        }
    }
    ("Unknown GPU".to_string(), "unknown".to_string())
}

fn get_ram_mb() -> (u64, u64) {
    let wmic = format!("{}\\System32\\wbem\\wmic.exe", sys_root());
    let out = no_window_cmd(&wmic)
        .args([
            "OS",
            "get",
            "FreePhysicalMemory,TotalVisibleMemorySize",
            "/VALUE",
        ])
        .output()
        .ok();
    let mut total_kb: u64 = 0;
    let mut free_kb: u64 = 0;
    if let Some(o) = out {
        let text = String::from_utf8_lossy(&o.stdout);
        for line in text.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("TotalVisibleMemorySize=") {
                total_kb = v.trim().parse().unwrap_or(0);
            }
            if let Some(v) = line.strip_prefix("FreePhysicalMemory=") {
                free_kb = v.trim().parse().unwrap_or(0);
            }
        }
    }
    (total_kb / 1024, (total_kb.saturating_sub(free_kb)) / 1024)
}

fn get_power_plan() -> (String, String) {
    let pc = format!("{}\\System32\\powercfg.exe", sys_root());
    let out = no_window_cmd(&pc).args(["/getactivescheme"]).output().ok();
    if let Some(o) = out {
        let text = String::from_utf8_lossy(&o.stdout).to_string();
        if let Some(line) = text.lines().find(|l| l.contains("Power Scheme GUID")) {
            let guid = line.split_whitespace().nth(3).unwrap_or("").to_string();
            let name = line
                .find('(')
                .and_then(|s| line.rfind(')').map(|e| line[s + 1..e].to_string()))
                .unwrap_or_else(|| "Unknown".to_string());
            return (guid, name);
        }
    }
    ("unknown".to_string(), "Unknown".to_string())
}

fn is_minecraft_running() -> bool {
    let tasklist = format!("{}\\System32\\tasklist.exe", sys_root());
    no_window_cmd(&tasklist)
        .args(["/FO", "CSV", "/NH"])
        .output()
        .ok()
        .map(|o| {
            let text = String::from_utf8_lossy(&o.stdout).to_lowercase();
            text.contains("javaw.exe")
                || text.contains("java.exe")
                || text.contains("minecraft")
                || text.contains("minecraftlauncher")
        })
        .unwrap_or(false)
}

pub fn detect_amd_impl() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    hklm.open_subkey("SOFTWARE\\AMD").is_ok()
        || hklm.open_subkey("SOFTWARE\\WOW6432Node\\AMD").is_ok()
        || hklm.open_subkey("SOFTWARE\\ATI Technologies").is_ok()
        || hklm
            .open_subkey("SOFTWARE\\WOW6432Node\\ATI Technologies")
            .is_ok()
}

pub fn get_system_info_impl() -> SystemInfo {
    let cpu_name = get_cpu_name();
    let (gpu_name, gpu_vendor) = get_gpu_info();
    let (ram_total_mb, ram_used_mb) = get_ram_mb();
    let (power_plan_guid, power_plan_name) = get_power_plan();
    let startup_app_count = crate::startup::list_impl().len();
    let minecraft_running = is_minecraft_running();
    let nvidia_detected = tweaks_impl::detect_nvidia_impl();
    let amd_detected = detect_amd_impl();

    SystemInfo {
        cpu_name,
        gpu_name,
        gpu_vendor,
        ram_total_mb,
        ram_used_mb,
        power_plan_name,
        power_plan_guid,
        gamedvr_enabled: !tweaks_impl::check_status_impl("disable-gamedvr"),
        game_bar_capture_enabled: !tweaks_impl::check_status_impl("disable-game-bar-capture"),
        animations_enabled: !tweaks_impl::check_status_impl("disable-animations"),
        edge_background_enabled: !tweaks_impl::check_status_impl("disable-edge-background-mode"),
        edge_startup_boost_enabled: !tweaks_impl::check_status_impl("disable-edge-startup-boost"),
        startup_app_count,
        minecraft_running,
        nvidia_detected,
        amd_detected,
    }
}

// â”€â”€ Process management â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

const SYSTEM_PROCESSES: &[&str] = &[
    "system",
    "smss",
    "csrss",
    "wininit",
    "winlogon",
    "lsass",
    "services",
    "svchost",
    "dwm",
    "explorer",
    "taskmgr",
    "conhost",
    "searchindexer",
    "spoolsv",
    "fontdrvhost",
    "wmiprvse",
    "securityhealthservice",
    "msmpeng",
    "registry",
    "ntoskrnl",
    "memory compression",
    "v-optimizer",
    "tasklist",
    "wbem",
];

fn is_system_process(name: &str) -> bool {
    let lower = name.to_lowercase();
    let bare = lower.trim_end_matches(".exe");
    SYSTEM_PROCESSES
        .iter()
        .any(|s| bare == *s || lower.contains(s))
}

pub fn list_processes_impl() -> Vec<ProcessInfo> {
    let ps = ps_exe();
    let script = concat!(
        "@(Get-Process | Where-Object {$_.WorkingSet64 -gt 15MB} | ",
        "Sort-Object WorkingSet64 -Descending | Select-Object -First 35 ",
        "Id,ProcessName,@{N='MemMB';E={[math]::Round($_.WorkingSet64/1MB,1)}}) | ",
        "ConvertTo-Json -Compress"
    );
    match no_window_cmd(&ps)
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output()
    {
        Ok(o) if o.status.success() => parse_processes(&String::from_utf8_lossy(&o.stdout)),
        _ => vec![],
    }
}

fn parse_processes(json: &str) -> Vec<ProcessInfo> {
    let json = json.trim();
    if json.is_empty() {
        return vec![];
    }
    #[derive(Deserialize)]
    struct Raw {
        #[serde(rename = "Id")]
        id: u32,
        #[serde(rename = "ProcessName")]
        process_name: String,
        #[serde(rename = "MemMB")]
        mem_mb: serde_json::Value,
    }
    let raws: Vec<Raw> = serde_json::from_str(json)
        .or_else(|_| serde_json::from_str(&format!("[{}]", json)))
        .unwrap_or_default();
    raws.into_iter()
        .map(|r| {
            let mem = match r.mem_mb {
                serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0),
                _ => 0.0,
            };
            let safe = !is_system_process(&r.process_name);
            ProcessInfo {
                pid: r.id,
                name: r.process_name,
                memory_mb: mem,
                is_safe_to_kill: safe,
            }
        })
        .collect()
}

pub fn kill_process_impl(pid: u32, name: &str) -> TweakOpResult {
    if is_system_process(name) {
        return TweakOpResult::fail(
            "Cannot terminate system process",
            "This process is protected from termination",
        );
    }
    let taskkill = format!("{}\\System32\\taskkill.exe", sys_root());
    match no_window_cmd(&taskkill)
        .args(["/PID", &pid.to_string(), "/F"])
        .output()
    {
        Ok(o) if o.status.success() => {
            TweakOpResult::ok(format!("Process {} (PID {}) terminated", name, pid))
        }
        Ok(o) => TweakOpResult::fail(
            "Failed to terminate process",
            String::from_utf8_lossy(&o.stderr).trim(),
        ),
        Err(e) => TweakOpResult::fail("Failed to run taskkill", e.to_string()),
    }
}

// â”€â”€ DNS â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub fn flush_dns_impl() -> TweakOpResult {
    let ipconfig = format!("{}\\System32\\ipconfig.exe", sys_root());
    match no_window_cmd(&ipconfig).args(["/flushdns"]).output() {
        Ok(o) if o.status.success() => {
            let msg = String::from_utf8_lossy(&o.stdout).trim().to_string();
            TweakOpResult::ok(if msg.is_empty() {
                "DNS resolver cache flushed".to_string()
            } else {
                msg
            })
        }
        Ok(o) => TweakOpResult::fail(
            "Failed to flush DNS",
            String::from_utf8_lossy(&o.stderr).trim(),
        ),
        Err(e) => TweakOpResult::fail("Failed to run ipconfig", e.to_string()),
    }
}

pub fn get_dns_info_impl() -> DnsInfo {
    let ps = ps_exe();
    let script = concat!(
        "$addrs = @(Get-DnsClientServerAddress -AddressFamily IPv4 ",
        "| Where-Object {$_.ServerAddresses.Count -gt 0} ",
        "| Select-Object -ExpandProperty ServerAddresses -First 4); ",
        "$host = $env:COMPUTERNAME; ",
        "@{servers=$addrs; hostname=$host} | ConvertTo-Json -Compress"
    );
    match no_window_cmd(&ps)
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output()
    {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
            #[derive(Deserialize)]
            struct Raw {
                servers: Option<serde_json::Value>,
                hostname: Option<String>,
            }
            if let Ok(raw) = serde_json::from_str::<Raw>(&text) {
                let servers = match raw.servers {
                    Some(serde_json::Value::Array(arr)) => arr
                        .into_iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect(),
                    Some(serde_json::Value::String(s)) => vec![s],
                    _ => vec![],
                };
                return DnsInfo {
                    servers,
                    hostname: raw.hostname.unwrap_or_default(),
                };
            }
            DnsInfo::default()
        }
        _ => DnsInfo::default(),
    }
}

// â”€â”€ Ping â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub fn ping_host_impl(host: &str) -> PingResult {
    let ping = format!("{}\\System32\\ping.exe", sys_root());
    match no_window_cmd(&ping)
        .args(["-n", "1", "-w", "2000", host])
        .output()
    {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let latency = stdout.lines().find_map(|l| {
                if l.contains("time<1ms") {
                    Some(0u32)
                } else if let Some(pos) = l.find("time=") {
                    let after = &l[pos + 5..];
                    let ms: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
                    ms.parse().ok()
                } else {
                    None
                }
            });
            if latency.is_some() {
                PingResult {
                    success: true,
                    host: host.to_string(),
                    latency_ms: latency,
                    error: None,
                }
            } else {
                PingResult {
                    success: false,
                    host: host.to_string(),
                    latency_ms: None,
                    error: Some("Request timed out or host unreachable".to_string()),
                }
            }
        }
        Err(e) => PingResult {
            success: false,
            host: host.to_string(),
            latency_ms: None,
            error: Some(e.to_string()),
        },
    }
}

// â”€â”€ Preset â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub fn apply_minecraft_preset_impl(tweak_ids: &[String]) -> Vec<PresetResult> {
    tweak_ids
        .iter()
        .map(|id| {
            let result = tweaks_impl::apply_impl(id);
            PresetResult {
                tweak_id: id.clone(),
                success: result.success,
                message: result.message,
            }
        })
        .collect()
}
