use crate::backup;
use crate::tweaks_impl::TweakOpResult;
use crate::util::no_window_cmd;
use serde::{Deserialize, Serialize};

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
fn powercfg_exe() -> String {
    format!("{}\\System32\\powercfg.exe", sys_root())
}

fn parse_first_guid(text: &str) -> Option<String> {
    for token in text.split_whitespace() {
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
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    parse_first_guid(&text)
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// 1. OVERLAY DETECTOR
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OverlayInfo {
    pub id: String,
    pub name: String,
    pub process_name: String,
    pub detected: bool,
    pub pid: Option<u32>,
    pub category: String,
    pub tip: String,
}

/// (id, display_name, process_name_no_ext, category, tip)
const OVERLAY_TARGETS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "discord",
        "Discord",
        "discord",
        "communication",
        "Can add overlay latency. Disable Game Activity in Discord Settings â†’ Activity Privacy.",
    ),
    (
        "steam-overlay",
        "Steam Overlay",
        "gameoverlayui",
        "gaming",
        "Thin overhead. Disable via Steam â†’ Settings â†’ In-Game â†’ Enable overlay.",
    ),
    (
        "xbox-game-bar",
        "Xbox Game Bar",
        "gamebar",
        "gaming",
        "Consumes CPU background resources. Disable in Windows Settings â†’ Gaming â†’ Xbox Game Bar.",
    ),
    (
        "nvidia-overlay",
        "NVIDIA GeForce Overlay",
        "nvcontainer",
        "performance",
        "ShadowPlay/Instant Replay uses constant GPU resources. Disable in GeForce Experience.",
    ),
    (
        "overwolf",
        "Overwolf",
        "overwolf",
        "recording",
        "Heavy CPU/RAM consumer. Disable or remove from startup if not actively used.",
    ),
    (
        "medal",
        "Medal.tv",
        "medal",
        "recording",
        "Constant background GPU encoding. Close if you're not actively clipping.",
    ),
    (
        "rtss",
        "RivaTuner Statistics Server",
        "rtss",
        "performance",
        "Frame limiter â€” useful for smooth gameplay, but adds a slight DPC overhead.",
    ),
    (
        "msi-afterburner",
        "MSI Afterburner",
        "msiafterburner",
        "performance",
        "Pairs with RTSS for monitoring. Very low overhead but disable the OSD if unused.",
    ),
];

pub fn detect_overlays_impl() -> Vec<OverlayInfo> {
    // One process name per line (no extension, lowercase)
    let script = "Get-Process | Select-Object -ExpandProperty Name";
    let out = no_window_cmd(ps_exe())
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output();

    let running: Vec<String> = match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| l.trim().to_lowercase())
            .filter(|l| !l.is_empty())
            .collect(),
        _ => vec![],
    };

    // Also fetch PIDs for detected processes: "name|pid" per line
    let pid_script = r#"Get-Process | ForEach-Object { "$($_.Name.ToLower())|$($_.Id)" }"#;
    let pid_out = no_window_cmd(ps_exe())
        .args(["-NonInteractive", "-NoProfile", "-Command", pid_script])
        .output()
        .ok();

    let mut pid_map: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    if let Some(o) = pid_out {
        let text = String::from_utf8_lossy(&o.stdout);
        for line in text.lines() {
            let parts: Vec<&str> = line.trim().splitn(2, '|').collect();
            if parts.len() == 2 {
                if let Ok(pid) = parts[1].parse::<u32>() {
                    // Keep only first occurrence (lowest PID) for each name
                    pid_map.entry(parts[0].to_string()).or_insert(pid);
                }
            }
        }
    }

    OVERLAY_TARGETS
        .iter()
        .map(|(id, name, proc_name, category, tip)| {
            let pn = proc_name.to_lowercase();
            let detected = running.iter().any(|r| r == &pn);
            let pid = if detected {
                pid_map.get(&pn).copied()
            } else {
                None
            };
            OverlayInfo {
                id: id.to_string(),
                name: name.to_string(),
                process_name: format!("{}.exe", proc_name),
                detected,
                pid,
                category: category.to_string(),
                tip: tip.to_string(),
            }
        })
        .collect()
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// 2. BACKGROUND LOAD SCANNER
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessLoad {
    pub pid: u32,
    pub name: String,
    pub ram_mb: f64,
    pub cpu_s: f64,
    pub is_gaming_impact: bool,
    pub impact_reason: String,
}

/// (substring to match in process name lowercase, human reason)
const GAMING_IMPACT: &[(&str, &str)] = &[
    (
        "antimalware",
        "Windows Defender â€” can stutter during background scans",
    ),
    (
        "msmpeng",
        "Windows Defender Antivirus â€” stutter risk during scans",
    ),
    (
        "msseces",
        "Microsoft Security Essentials â€” antivirus overhead",
    ),
    ("avastui", "Avast Antivirus â€” high background CPU usage"),
    ("avgui", "AVG Antivirus â€” high background CPU usage"),
    (
        "bdagent",
        "Bitdefender â€” scan overhead can cause frame drops",
    ),
    (
        "ekrn",
        "ESET Kernel â€” kernel activity can cause micro-stutters",
    ),
    ("ksde", "Kaspersky â€” deep packet inspection overhead"),
    ("mbam", "Malwarebytes â€” periodic scans spike CPU"),
    ("chrome", "Chrome â€” competes for RAM bandwidth"),
    ("firefox", "Firefox â€” RAM-heavy, competes for memory"),
    (
        "msedge",
        "Edge â€” multiple processes, competes for resources",
    ),
    ("opera", "Opera â€” background processes consume memory"),
    (
        "onedrive",
        "OneDrive â€” file sync causes disk I/O during gaming",
    ),
    (
        "dropbox",
        "Dropbox â€” background sync causes disk I/O spikes",
    ),
    (
        "googledrivefs",
        "Google Drive â€” file sync I/O during gaming",
    ),
    (
        "steam",
        "Steam â€” friend list and updates can cause brief hangs",
    ),
    (
        "epicgameslauncher",
        "Epic Games Launcher â€” background update checks",
    ),
    ("origin", "EA App/Origin â€” background activity overhead"),
    ("eadesktop", "EA Desktop â€” background service overhead"),
    ("battlenet", "Battle.net â€” scanning and update checks"),
    (
        "discord",
        "Discord â€” overlay and voice processing overhead",
    ),
    (
        "teams",
        "Microsoft Teams â€” heavy CPU/RAM, disable while gaming",
    ),
    (
        "slack",
        "Slack â€” Electron-based, consumes significant RAM",
    ),
    ("obs64", "OBS Studio â€” active recording uses CPU/GPU"),
    ("obs32", "OBS Studio â€” active recording uses CPU/GPU"),
    ("xboxapp", "Xbox App â€” background services overhead"),
];

pub fn scan_background_load_impl() -> Vec<ProcessLoad> {
    // pipe-delimited: pid|name|workingset_bytes|cpu_seconds
    let script = r#"
Get-Process | Where-Object { $_.Id -ne 0 -and $_.Name -ne 'Idle' } |
Sort-Object WorkingSet64 -Descending |
Select-Object -First 30 |
ForEach-Object { "$($_.Id)|$($_.Name)|$($_.WorkingSet64)|$([math]::Round($_.CPU,2))" }
"#;

    let out = no_window_cmd(ps_exe())
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output();

    let mut results = vec![];

    if let Ok(o) = out {
        let text = String::from_utf8_lossy(&o.stdout);
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.splitn(4, '|').collect();
            if parts.len() < 4 {
                continue;
            }
            let pid: u32 = parts[0].parse().unwrap_or(0);
            let name = parts[1].to_string();
            let ram_bytes: u64 = parts[2].parse().unwrap_or(0);
            let cpu_s: f64 = parts[3].parse().unwrap_or(0.0);

            let name_lower = name.to_lowercase();
            let (is_gaming_impact, impact_reason) = GAMING_IMPACT
                .iter()
                .find(|(pat, _)| name_lower.contains(pat))
                .map(|(_, reason)| (true, reason.to_string()))
                .unwrap_or((false, String::new()));

            results.push(ProcessLoad {
                pid,
                name,
                ram_mb: ram_bytes as f64 / (1024.0 * 1024.0),
                cpu_s,
                is_gaming_impact,
                impact_reason,
            });
        }
    }

    results
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// 3. SHADER CACHE CLEANER
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShaderCacheEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size_mb: f64,
    pub exists: bool,
    pub vendor: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CleanResult {
    pub success: bool,
    pub cleaned_count: u32,
    pub total_mb_freed: f64,
    pub errors: Vec<String>,
    pub message: String,
}

fn dir_size_mb(path: &std::path::Path) -> f64 {
    fn bytes(p: &std::path::Path) -> u64 {
        let mut total = 0u64;
        if let Ok(entries) = std::fs::read_dir(p) {
            for entry in entries.flatten() {
                let ep = entry.path();
                if ep.is_file() {
                    total += std::fs::metadata(&ep).map(|m| m.len()).unwrap_or(0);
                } else if ep.is_dir() {
                    total += bytes(&ep);
                }
            }
        }
        total
    }
    bytes(path) as f64 / (1024.0 * 1024.0)
}

pub fn get_shader_caches_impl() -> Vec<ShaderCacheEntry> {
    let local = std::env::var("LOCALAPPDATA")
        .unwrap_or_else(|_| format!("C:\\Users\\{}\\AppData\\Local", whoami()));
    let roaming = std::env::var("APPDATA")
        .unwrap_or_else(|_| format!("C:\\Users\\{}\\AppData\\Roaming", whoami()));
    let prog_data = std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string());

    let targets: Vec<(&str, &str, String, &str)> = vec![
        (
            "dx-d3d",
            "DirectX Shader Cache",
            format!("{}\\D3DSCache", local),
            "directx",
        ),
        (
            "nvidia-dx",
            "NVIDIA DX Shader Cache",
            format!("{}\\NVIDIA\\DXCache", local),
            "nvidia",
        ),
        (
            "nvidia-gl",
            "NVIDIA OpenGL Cache",
            format!("{}\\NVIDIA\\GLCache", local),
            "nvidia",
        ),
        (
            "amd-dx",
            "AMD DX Shader Cache",
            format!("{}\\AMD\\DxCache", local),
            "amd",
        ),
        (
            "amd-cn",
            "AMD Compute Cache",
            format!("{}\\AMD\\CN", roaming),
            "amd",
        ),
        (
            "nvidia-nv",
            "NVIDIA NV Cache",
            format!("{}\\NVIDIA Corporation\\NV_Cache", prog_data),
            "nvidia",
        ),
    ];

    targets
        .into_iter()
        .map(|(id, name, path, vendor)| {
            let p = std::path::Path::new(&path);
            let exists = p.exists();
            let size_mb = if exists { dir_size_mb(p) } else { 0.0 };
            ShaderCacheEntry {
                id: id.to_string(),
                name: name.to_string(),
                path,
                size_mb,
                exists,
                vendor: vendor.to_string(),
            }
        })
        .collect()
}

pub fn clean_shader_caches_impl(ids: Vec<String>) -> CleanResult {
    let caches = get_shader_caches_impl();
    let mut cleaned = 0u32;
    let mut freed = 0.0f64;
    let mut errors: Vec<String> = vec![];

    for cache in &caches {
        if !ids.contains(&cache.id) || !cache.exists {
            continue;
        }
        freed += cache.size_mb;
        match std::fs::read_dir(&cache.path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let p = entry.path();
                    let res = if p.is_dir() {
                        std::fs::remove_dir_all(&p)
                    } else {
                        std::fs::remove_file(&p)
                    };
                    if let Err(e) = res {
                        errors.push(format!(
                            "{}: {}",
                            p.file_name().unwrap_or_default().to_string_lossy(),
                            e
                        ));
                    }
                }
                cleaned += 1;
            }
            Err(e) => errors.push(format!("{}: {}", cache.name, e)),
        }
    }

    let message = if cleaned == 0 {
        "No caches were cleaned (none selected or all empty).".to_string()
    } else if errors.is_empty() {
        format!("Cleaned {} cache(s), freed ~{:.1} MB", cleaned, freed)
    } else {
        format!(
            "Cleaned {} cache(s), {} error(s). Some files may be in use.",
            cleaned,
            errors.len()
        )
    };

    CleanResult {
        success: cleaned > 0,
        cleaned_count: cleaned,
        total_mb_freed: freed,
        errors,
        message,
    }
}

fn whoami() -> String {
    std::env::var("USERNAME").unwrap_or_else(|_| "User".to_string())
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// 4. GAME SESSION MODE
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct GameSessionState {
    active: bool,
    started_at_ms: i64,
    previous_power_scheme: String,
    actions_applied: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameSessionStatus {
    pub active: bool,
    pub started_at_ms: i64,
    pub duration_secs: u64,
    pub actions_applied: Vec<String>,
}

fn game_session_file() -> std::path::PathBuf {
    backup::get_app_data_dir().join("game_session.json")
}

const HIGH_PERF_GUID: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";

pub fn start_game_session_impl() -> TweakOpResult {
    let state: GameSessionState = backup::read_json_metadata(&game_session_file());
    if state.active {
        return TweakOpResult::ok("Game Session is already active.");
    }

    let mut actions: Vec<String> = vec![];
    let mut new_state = GameSessionState {
        active: true,
        started_at_ms: backup::now_ms(),
        previous_power_scheme: String::new(),
        actions_applied: vec![],
    };

    // 1. Save current power scheme and switch to High Performance
    if let Some(current_guid) = get_active_power_scheme() {
        new_state.previous_power_scheme = current_guid;
    }
    if let Ok(o) = no_window_cmd(powercfg_exe())
        .args(["/setactive", HIGH_PERF_GUID])
        .output()
    {
        if o.status.success() {
            actions.push("Switched to High Performance power plan".to_string());
        }
    }

    // 2. Flush RAM standby list (requires admin â€” silently skip if it fails)
    let ram_script = r#"
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class StandbyFlusher {
    [DllImport("ntdll.dll")] public static extern int NtSetSystemInformation(int cls, IntPtr buf, int len);
    public static void Flush() { var v = new IntPtr(80); NtSetSystemInformation(80, v, IntPtr.Size); }
}
'@
[StandbyFlusher]::Flush()
Write-Output "RAM_FLUSHED"
"#;
    if let Ok(o) = no_window_cmd(ps_exe())
        .args(["-NonInteractive", "-NoProfile", "-Command", ram_script])
        .output()
    {
        if String::from_utf8_lossy(&o.stdout).contains("RAM_FLUSHED") {
            actions.push("Flushed RAM standby list".to_string());
        }
    }

    // 3. Flush DNS cache
    let dns_script = "ipconfig /flushdns | Out-Null; Write-Output 'DNS_FLUSHED'";
    if let Ok(o) = no_window_cmd(ps_exe())
        .args(["-NonInteractive", "-NoProfile", "-Command", dns_script])
        .output()
    {
        if String::from_utf8_lossy(&o.stdout).contains("DNS_FLUSHED") {
            actions.push("Flushed DNS cache".to_string());
        }
    }

    new_state.actions_applied = actions.clone();
    let _ = backup::write_json_metadata(&game_session_file(), &new_state);

    let summary = if actions.is_empty() {
        "Session started (run as Admin for full effect).".to_string()
    } else {
        format!("Session started â€” {}", actions.join(", "))
    };
    TweakOpResult::ok(&summary)
}

pub fn end_game_session_impl() -> TweakOpResult {
    let state: GameSessionState = backup::read_json_metadata(&game_session_file());
    if !state.active {
        return TweakOpResult::ok("No active Game Session to end.");
    }

    let mut restored: Vec<String> = vec![];

    // Restore previous power plan
    if !state.previous_power_scheme.is_empty() {
        if let Ok(o) = no_window_cmd(powercfg_exe())
            .args(["/setactive", &state.previous_power_scheme])
            .output()
        {
            if o.status.success() {
                restored.push("Restored previous power plan".to_string());
            }
        }
    }

    // Clear state
    let _ = backup::write_json_metadata(&game_session_file(), &GameSessionState::default());

    let summary = if restored.is_empty() {
        "Game Session ended.".to_string()
    } else {
        format!("Game Session ended â€” {}", restored.join(", "))
    };
    TweakOpResult::ok(&summary)
}

pub fn get_game_session_status_impl() -> GameSessionStatus {
    let state: GameSessionState = backup::read_json_metadata(&game_session_file());
    let now = backup::now_ms();
    let duration_secs = if state.active && state.started_at_ms > 0 {
        ((now - state.started_at_ms) / 1000).max(0) as u64
    } else {
        0
    };
    GameSessionStatus {
        active: state.active,
        started_at_ms: state.started_at_ms,
        duration_secs,
        actions_applied: state.actions_applied,
    }
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// 5. MINECRAFT PROCESS MONITOR
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MinecraftMonitor {
    pub found: bool,
    pub instance_count: u32,
    pub pid: Option<u32>,
    pub ram_mb: f64,
    pub cpu_s: f64,
    pub window_title: String,
}

pub fn get_minecraft_monitor_impl() -> MinecraftMonitor {
    // Each process line: "pid|ram_bytes|cpu|title"
    let script = r#"
$procs = Get-Process -Name 'javaw' -ErrorAction SilentlyContinue
if (-not $procs) { Write-Output 'NONE'; exit 0 }
Write-Output "COUNT:$($procs.Count)"
foreach ($p in $procs) {
    $title = if ($p.MainWindowTitle) { $p.MainWindowTitle } else { 'Minecraft (background)' }
    Write-Output "PROC:$($p.Id)|$($p.WorkingSet64)|$([math]::Round($p.CPU,1))|$title"
}
"#;

    let out = no_window_cmd(ps_exe())
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output();

    let mut result = MinecraftMonitor {
        found: false,
        instance_count: 0,
        pid: None,
        ram_mb: 0.0,
        cpu_s: 0.0,
        window_title: String::new(),
    };

    if let Ok(o) = out {
        let text = String::from_utf8_lossy(&o.stdout);
        if text.contains("NONE") {
            return result;
        }
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with("COUNT:") {
                result.instance_count = line[6..].parse().unwrap_or(0);
                if result.instance_count > 0 {
                    result.found = true;
                }
            } else if line.starts_with("PROC:") && result.pid.is_none() {
                // First instance becomes the primary one shown
                let data = &line[5..];
                let parts: Vec<&str> = data.splitn(4, '|').collect();
                if parts.len() >= 4 {
                    result.pid = parts[0].parse().ok();
                    result.ram_mb = parts[1].parse::<u64>().unwrap_or(0) as f64 / (1024.0 * 1024.0);
                    result.cpu_s = parts[2].parse().unwrap_or(0.0);
                    result.window_title = parts[3].to_string();
                }
            }
        }
    }

    result
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// 6. BENCHMARK SESSION MODE
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SystemSnapshot {
    pub label: String,
    pub timestamp_ms: i64,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub process_count: u32,
    pub uptime_secs: u64,
    pub power_plan_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BenchmarkComparison {
    pub before: SystemSnapshot,
    pub after: SystemSnapshot,
    pub duration_secs: u64,
    pub ram_delta_mb: i64,
    pub process_delta: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BenchmarkStateResult {
    pub before: Option<SystemSnapshot>,
    pub after: Option<SystemSnapshot>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct BenchmarkStatePersisted {
    before: Option<SystemSnapshot>,
    after: Option<SystemSnapshot>,
}

fn benchmark_file() -> std::path::PathBuf {
    backup::get_app_data_dir().join("benchmark_session.json")
}

fn take_snapshot_raw(label: &str) -> SystemSnapshot {
    let script = r#"
$os = Get-WmiObject Win32_OperatingSystem
$ppline = (powercfg /getactivescheme 2>&1).ToString()
$ppName = if ($ppline -match '\((.+)\)') { $Matches[1].Trim() } else { 'Unknown' }
$procs = (Get-Process | Measure-Object).Count
$freeKB = $os.FreePhysicalMemory
$totalKB = $os.TotalVisibleMemorySize
$usedMB = [math]::Round(($totalKB - $freeKB) / 1024)
$totalMB = [math]::Round($totalKB / 1024)
$boot = $os.ConvertToDateTime($os.LastBootUpTime)
$uptime = [math]::Round(((Get-Date) - $boot).TotalSeconds)
Write-Output "RAM_USED:$usedMB"
Write-Output "RAM_TOTAL:$totalMB"
Write-Output "UPTIME:$uptime"
Write-Output "PROCS:$procs"
Write-Output "PP:$ppName"
"#;

    let out = no_window_cmd(ps_exe())
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output();

    let mut snap = SystemSnapshot {
        label: label.to_string(),
        timestamp_ms: backup::now_ms(),
        ..Default::default()
    };

    if let Ok(o) = out {
        let text = String::from_utf8_lossy(&o.stdout);
        for line in text.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("RAM_USED:") {
                snap.ram_used_mb = v.parse().unwrap_or(0);
            } else if let Some(v) = line.strip_prefix("RAM_TOTAL:") {
                snap.ram_total_mb = v.parse().unwrap_or(0);
            } else if let Some(v) = line.strip_prefix("UPTIME:") {
                snap.uptime_secs = v.parse().unwrap_or(0);
            } else if let Some(v) = line.strip_prefix("PROCS:") {
                snap.process_count = v.parse().unwrap_or(0);
            } else if let Some(v) = line.strip_prefix("PP:") {
                snap.power_plan_name = v.trim().to_string();
            }
        }
    }

    snap
}

/// `slot` is `"before"` or `"after"`.
pub fn take_snapshot_impl(label: String, slot: String) -> SystemSnapshot {
    let snap = take_snapshot_raw(&label);
    let mut state: BenchmarkStatePersisted = backup::read_json_metadata(&benchmark_file());
    if slot == "before" {
        state.before = Some(snap.clone());
        state.after = None; // reset after when a new before is taken
    } else {
        state.after = Some(snap.clone());
    }
    let _ = backup::write_json_metadata(&benchmark_file(), &state);
    snap
}

pub fn get_benchmark_state_impl() -> BenchmarkStateResult {
    let state: BenchmarkStatePersisted = backup::read_json_metadata(&benchmark_file());
    BenchmarkStateResult {
        before: state.before,
        after: state.after,
    }
}

pub fn get_benchmark_comparison_impl() -> Option<BenchmarkComparison> {
    let state: BenchmarkStatePersisted = backup::read_json_metadata(&benchmark_file());
    let before = state.before?;
    let after = state.after?;
    let duration_secs = ((after.timestamp_ms - before.timestamp_ms) / 1000).max(0) as u64;
    let ram_delta_mb = after.ram_used_mb as i64 - before.ram_used_mb as i64;
    let process_delta = after.process_count as i32 - before.process_count as i32;
    Some(BenchmarkComparison {
        before,
        after,
        duration_secs,
        ram_delta_mb,
        process_delta,
    })
}

pub fn clear_benchmark_impl() {
    let _ = backup::write_json_metadata(&benchmark_file(), &BenchmarkStatePersisted::default());
}
