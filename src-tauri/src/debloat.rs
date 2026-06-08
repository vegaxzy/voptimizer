//! Debloat — Temp & Cache Cleaner (v1).
//!
//! Safety-first design: `scan_cleanup_impl` only MEASURES sizes (never deletes);
//! the UI previews each category with its size and the user explicitly selects
//! what to clean. `clean_cleanup_impl` only ever touches well-known temp/cache
//! locations — never personal files. Locked/in-use files are skipped, not forced.

use crate::util::no_window_cmd;
use serde::{Deserialize, Serialize};
use std::path::Path;

// ── Data types ───────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CleanCategory {
    pub id: String,
    pub name: String,
    pub description: String,
    pub size_mb: f64,
    pub file_count: u64,
    pub exists: bool,
    pub requires_admin: bool,
    pub irreversible: bool,
    pub default_selected: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DebloatResult {
    pub success: bool,
    pub categories_cleaned: u32,
    pub total_mb_freed: f64,
    pub errors: Vec<String>,
    pub message: String,
}

// ── Environment helpers ──────────────────────────────────────────────────────

fn env_path(var: &str, fallback: &str) -> String {
    std::env::var(var).unwrap_or_else(|_| fallback.to_string())
}
fn local_appdata() -> String {
    env_path("LOCALAPPDATA", "C:\\Users\\Default\\AppData\\Local")
}
fn program_data() -> String {
    env_path("ProgramData", "C:\\ProgramData")
}
fn sys_root() -> String {
    env_path("SystemRoot", "C:\\Windows")
}
fn user_temp() -> String {
    env_path("TEMP", &format!("{}\\Temp", local_appdata()))
}
fn ps_exe() -> String {
    format!(
        "{}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
        sys_root()
    )
}

// ── Category definition ──────────────────────────────────────────────────────

/// How a category's space is measured / reclaimed.
enum CleanKind {
    /// Clear the CONTENTS of each directory (the directories themselves stay).
    DirContents(Vec<String>),
    /// Delete only files in `dir` whose name starts with one of `prefixes`.
    MatchingFiles { dir: String, prefixes: Vec<&'static str> },
    /// The Recycle Bin — measured by summing `$Recycle.Bin`, emptied via PowerShell.
    RecycleBin,
}

struct CatSpec {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    requires_admin: bool,
    irreversible: bool,
    default_selected: bool,
    kind: CleanKind,
}

fn category_specs() -> Vec<CatSpec> {
    let local = local_appdata();
    let win = sys_root();
    let pdata = program_data();

    // Deduplicate user temp vs LOCALAPPDATA\Temp (usually identical).
    let mut temp_dirs = vec![user_temp()];
    let la_temp = format!("{}\\Temp", local);
    if !temp_dirs.iter().any(|p| p.eq_ignore_ascii_case(&la_temp)) {
        temp_dirs.push(la_temp);
    }

    vec![
        CatSpec {
            id: "user-temp",
            name: "User Temp Files",
            description: "Your account's temporary files (%TEMP%). The biggest easy win and always safe to clear.",
            requires_admin: false,
            irreversible: false,
            default_selected: true,
            kind: CleanKind::DirContents(temp_dirs),
        },
        CatSpec {
            id: "windows-temp",
            name: "Windows Temp Files",
            description: "System-wide temporary files in C:\\Windows\\Temp. Safe to clear; needs administrator.",
            requires_admin: true,
            irreversible: false,
            default_selected: true,
            kind: CleanKind::DirContents(vec![format!("{}\\Temp", win)]),
        },
        CatSpec {
            id: "windows-update-cache",
            name: "Windows Update Cache",
            description: "Downloaded update installers in SoftwareDistribution. Windows re-downloads anything it still needs.",
            requires_admin: true,
            irreversible: false,
            default_selected: true,
            kind: CleanKind::DirContents(vec![format!(
                "{}\\SoftwareDistribution\\Download",
                win
            )]),
        },
        CatSpec {
            id: "crash-dumps",
            name: "Crash Dumps & Minidumps",
            description: "Application crash dumps and kernel minidumps. Only useful for debugging past crashes.",
            requires_admin: true,
            irreversible: false,
            default_selected: true,
            kind: CleanKind::DirContents(vec![
                format!("{}\\CrashDumps", local),
                format!("{}\\Minidump", win),
            ]),
        },
        CatSpec {
            id: "error-reporting",
            name: "Windows Error Reporting",
            description: "Queued Windows Error Reporting (WER) data waiting to be sent to Microsoft.",
            requires_admin: true,
            irreversible: false,
            default_selected: true,
            kind: CleanKind::DirContents(vec![
                format!("{}\\Microsoft\\Windows\\WER", local),
                format!("{}\\Microsoft\\Windows\\WER", pdata),
            ]),
        },
        CatSpec {
            id: "thumbnail-cache",
            name: "Thumbnail & Icon Cache",
            description: "Cached thumbnail and icon databases. Windows rebuilds them automatically on demand.",
            requires_admin: false,
            irreversible: false,
            default_selected: true,
            kind: CleanKind::MatchingFiles {
                dir: format!("{}\\Microsoft\\Windows\\Explorer", local),
                prefixes: vec!["thumbcache_", "iconcache_"],
            },
        },
        CatSpec {
            id: "prefetch",
            name: "Prefetch Data",
            description: "App launch-prediction data in C:\\Windows\\Prefetch. Windows rebuilds it; off by default (debatable benefit).",
            requires_admin: true,
            irreversible: false,
            default_selected: false,
            kind: CleanKind::DirContents(vec![format!("{}\\Prefetch", win)]),
        },
        CatSpec {
            id: "recycle-bin",
            name: "Recycle Bin",
            description: "Permanently deletes everything in the Recycle Bin on all drives. This cannot be undone.",
            requires_admin: false,
            irreversible: true,
            default_selected: false,
            kind: CleanKind::RecycleBin,
        },
    ]
}

// ── Size measurement (read-only) ─────────────────────────────────────────────

/// Recursively sum file bytes + count under `path`.
fn dir_stats(path: &Path) -> (u64, u64) {
    let mut bytes = 0u64;
    let mut count = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let ep = entry.path();
            match entry.file_type() {
                Ok(ft) if ft.is_dir() => {
                    let (b, c) = dir_stats(&ep);
                    bytes += b;
                    count += c;
                }
                Ok(ft) if ft.is_file() => {
                    bytes += std::fs::metadata(&ep).map(|m| m.len()).unwrap_or(0);
                    count += 1;
                }
                _ => {}
            }
        }
    }
    (bytes, count)
}

fn matching_files_stats(dir: &Path, prefixes: &[&str]) -> (u64, u64) {
    let mut bytes = 0u64;
    let mut count = 0u64;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if prefixes.iter().any(|p| name.starts_with(p)) {
                if let Ok(md) = entry.metadata() {
                    if md.is_file() {
                        bytes += md.len();
                        count += 1;
                    }
                }
            }
        }
    }
    (bytes, count)
}

fn recycle_bin_stats() -> (u64, u64) {
    let script = r#"
$ErrorActionPreference = 'SilentlyContinue'
$shell = New-Object -ComObject Shell.Application
$bin = $shell.NameSpace(0xA)
$bytes = [int64]0
$count = [int64]0
if ($bin) {
  foreach ($item in $bin.Items()) {
    $count += 1
    try { $bytes += [int64]$item.Size } catch {}
  }
}
Write-Output "BYTES=$bytes"
Write-Output "COUNT=$count"
"#;

    let out = no_window_cmd(ps_exe())
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output();

    let Ok(output) = out else {
        return (0, 0);
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut bytes = 0;
    let mut count = 0;
    for line in text.lines() {
        if let Some(v) = line.trim().strip_prefix("BYTES=") {
            bytes = v.parse().unwrap_or(0);
        } else if let Some(v) = line.trim().strip_prefix("COUNT=") {
            count = v.parse().unwrap_or(0);
        }
    }
    (bytes, count)
}

fn spec_stats(kind: &CleanKind) -> (u64, u64, bool) {
    match kind {
        CleanKind::DirContents(dirs) => {
            let mut bytes = 0u64;
            let mut count = 0u64;
            let mut exists = false;
            for d in dirs {
                let p = Path::new(d);
                if p.exists() {
                    exists = true;
                    let (b, c) = dir_stats(p);
                    bytes += b;
                    count += c;
                }
            }
            (bytes, count, exists)
        }
        CleanKind::MatchingFiles { dir, prefixes } => {
            let p = Path::new(dir);
            if p.exists() {
                let (b, c) = matching_files_stats(p, prefixes);
                (b, c, true)
            } else {
                (0, 0, false)
            }
        }
        CleanKind::RecycleBin => {
            let (b, c) = recycle_bin_stats();
            (b, c, true)
        }
    }
}

fn to_mb(bytes: u64) -> f64 {
    (bytes as f64 / (1024.0 * 1024.0) * 10.0).round() / 10.0
}

pub fn scan_cleanup_impl() -> Vec<CleanCategory> {
    category_specs()
        .into_iter()
        .map(|spec| {
            let (bytes, count, exists) = spec_stats(&spec.kind);
            CleanCategory {
                id: spec.id.to_string(),
                name: spec.name.to_string(),
                description: spec.description.to_string(),
                size_mb: to_mb(bytes),
                file_count: count,
                exists,
                requires_admin: spec.requires_admin,
                irreversible: spec.irreversible,
                default_selected: spec.default_selected,
            }
        })
        .collect()
}

// ── Cleaning ─────────────────────────────────────────────────────────────────

/// Deletes the contents of `dir` (not the directory itself). Returns
/// (bytes_freed, error_count). Locked/in-use files are skipped silently.
fn clean_dir_contents(dir: &Path) -> (u64, u64) {
    let mut freed = 0u64;
    let mut errs = 0u64;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let ep = entry.path();
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            // Measure before deleting so we only count what actually frees.
            let (size, _) = if is_dir {
                dir_stats(&ep)
            } else {
                (std::fs::metadata(&ep).map(|m| m.len()).unwrap_or(0), 1)
            };
            let res = if is_dir {
                std::fs::remove_dir_all(&ep)
            } else {
                std::fs::remove_file(&ep)
            };
            match res {
                Ok(_) => freed += size,
                Err(_) => errs += 1,
            }
        }
    }
    (freed, errs)
}

fn clean_matching_files(dir: &Path, prefixes: &[&str]) -> (u64, u64) {
    let mut freed = 0u64;
    let mut errs = 0u64;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if !prefixes.iter().any(|p| name.starts_with(p)) {
                continue;
            }
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            match std::fs::remove_file(entry.path()) {
                Ok(_) => freed += size,
                Err(_) => errs += 1,
            }
        }
    }
    (freed, errs)
}

fn empty_recycle_bin() -> Result<u64, String> {
    // Measure first (best effort) so we can report what was freed.
    let (bytes, _) = recycle_bin_stats();
    let out = no_window_cmd(ps_exe())
        .args([
            "-NonInteractive",
            "-NoProfile",
            "-Command",
            "Clear-RecycleBin -Force -ErrorAction SilentlyContinue",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(bytes)
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn clean_cleanup_impl(ids: Vec<String>) -> DebloatResult {
    let mut freed_bytes = 0u64;
    let mut categories_cleaned = 0u32;
    let mut errors: Vec<String> = vec![];

    for spec in category_specs() {
        if !ids.iter().any(|id| id == spec.id) {
            continue;
        }
        let (cat_freed, cat_errs): (u64, u64) = match &spec.kind {
            CleanKind::DirContents(dirs) => {
                let mut f = 0u64;
                let mut e = 0u64;
                for d in dirs {
                    let p = Path::new(d);
                    if p.exists() {
                        let (df, de) = clean_dir_contents(p);
                        f += df;
                        e += de;
                    }
                }
                (f, e)
            }
            CleanKind::MatchingFiles { dir, prefixes } => {
                let p = Path::new(dir);
                if p.exists() {
                    clean_matching_files(p, prefixes)
                } else {
                    (0, 0)
                }
            }
            CleanKind::RecycleBin => match empty_recycle_bin() {
                Ok(b) => (b, 0),
                Err(e) => {
                    errors.push(format!("{}: {}", spec.name, e));
                    (0, 0)
                }
            },
        };

        freed_bytes += cat_freed;
        categories_cleaned += 1;
        if cat_errs > 0 {
            errors.push(format!(
                "{}: {} item(s) in use and skipped",
                spec.name, cat_errs
            ));
        }
    }

    let total_mb_freed = to_mb(freed_bytes);
    let message = if categories_cleaned == 0 {
        "No categories selected.".to_string()
    } else if errors.is_empty() {
        format!(
            "Cleaned {} categor{}, freed {:.1} MB",
            categories_cleaned,
            if categories_cleaned == 1 { "y" } else { "ies" },
            total_mb_freed
        )
    } else {
        format!(
            "Cleaned {} categor{}, freed {:.1} MB ({} note(s) — some files were in use)",
            categories_cleaned,
            if categories_cleaned == 1 { "y" } else { "ies" },
            total_mb_freed,
            errors.len()
        )
    };

    DebloatResult {
        success: categories_cleaned > 0,
        categories_cleaned,
        total_mb_freed,
        errors,
        message,
    }
}

// ═════════════════════════════════════════════════════════════════════════════
//  Bloatware (UWP / Microsoft Store apps) remover  (Debloat v2)
//
//  Safety model:
//   • A hardcoded PROTECTED set + frameworks/system-signed apps can NEVER be
//     selected for removal (removable = false), so the Store, runtimes, and
//     shell components are untouchable.
//   • Only per-user removal (Remove-AppxPackage, no -AllUsers) — no admin, and
//     every Store app is reinstallable from the Store afterwards.
//   • remove_appx_impl re-validates each id against the live removable list, so
//     a protected package can't be removed even if the UI sends its id.
// ═════════════════════════════════════════════════════════════════════════════

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppxPackage {
    pub id: String, // PackageFullName — the unique handle used for removal
    pub name: String,
    pub publisher: String,
    pub category: String, // "bloat" | "app" | "system"
    pub removable: bool,
    pub recommended: bool,
    pub note: String,
}

/// Critical packages that must never be offered for removal (substring match,
/// lower-case). Frameworks and System-signed apps are also auto-protected.
const PROTECTED: &[&str] = &[
    "windowsstore",
    "desktopappinstaller",
    "storepurchaseapp",
    "sechealthui",
    "shellexperiencehost",
    "startmenuexperiencehost",
    "aad.brokerplugin",
    "accountscontrol",
    "immersivecontrolpanel", // Settings
    "windows.cbspreview",
    "lockapp",
    "peopleexperiencehost",
    "cloudexperiencehost",
    "creddialoghost",
    "win32webviewhost",
    "windows.search",
    "microsoftwindows.client",
    "xboxgamecallableui",
    "windows.apprep",
    "secondarytileexperience",
    "microsoft.ui.xaml",
    "microsoft.vclibs",
    "microsoft.net.",
    "microsoft.services.store",
    "microsoft.windows.appruntime",
];

/// Returns Some((friendly_name, note)) when a package is known bloat.
fn bloat_lookup(name_lower: &str) -> Option<(&'static str, &'static str)> {
    const REINSTALL: &str = "Reinstallable from the Microsoft Store";
    let table: &[(&str, &str, &str)] = &[
        ("xboxgamingoverlay", "Xbox Game Bar", "Removing disables the Win+G Game Bar overlay"),
        ("xboxgameoverlay", "Xbox Game Overlay", REINSTALL),
        ("xboxspeechtotextoverlay", "Xbox Speech-to-Text", REINSTALL),
        ("xboxidentityprovider", "Xbox Identity Provider", "Needed to sign in to Xbox/Game Pass"),
        ("xbox.tcui", "Xbox Live UI", REINSTALL),
        ("gamingapp", "Xbox app", REINSTALL),
        ("xboxapp", "Xbox Console Companion", REINSTALL),
        ("bingweather", "Weather", REINSTALL),
        ("bingnews", "News", REINSTALL),
        ("bingsearch", "Bing Search", REINSTALL),
        ("solitairecollection", "Microsoft Solitaire Collection", REINSTALL),
        ("zunemusic", "Groove Music / Media Player", REINSTALL),
        ("zunevideo", "Films & TV", REINSTALL),
        ("3dviewer", "3D Viewer", REINSTALL),
        ("mixedreality.portal", "Mixed Reality Portal", REINSTALL),
        ("windowsfeedbackhub", "Feedback Hub", REINSTALL),
        ("gethelp", "Get Help", REINSTALL),
        ("getstarted", "Tips", REINSTALL),
        ("microsoft.people", "People", REINSTALL),
        ("windowsmaps", "Maps", REINSTALL),
        ("officehub", "Office / My Office", REINSTALL),
        ("skypeapp", "Skype", REINSTALL),
        ("microsoft.todos", "Microsoft To Do", REINSTALL),
        ("clipchamp", "Clipchamp", REINSTALL),
        ("powerautomatedesktop", "Power Automate", REINSTALL),
        ("windows.devhome", "Dev Home", REINSTALL),
        ("msteams", "Microsoft Teams (personal)", REINSTALL),
        ("microsoftteams", "Microsoft Teams (personal)", REINSTALL),
        ("yourphone", "Phone Link", "Removing breaks PC↔phone linking"),
        ("windowscommunicationsapps", "Mail & Calendar", REINSTALL),
        ("spotify", "Spotify", REINSTALL),
        ("disney", "Disney+", REINSTALL),
        ("wallet", "Wallet", REINSTALL),
    ];
    table
        .iter()
        .find(|(pat, _, _)| name_lower.contains(pat))
        .map(|(_, friendly, note)| (*friendly, *note))
}

fn ps_field(s: &str) -> String {
    s.trim().to_string()
}

/// Turn a certificate subject ("CN=Microsoft Corporation, O=…") into a short label.
fn friendly_publisher(raw: &str) -> String {
    let r = raw.trim();
    if r.to_ascii_lowercase().contains("microsoft") {
        return "Microsoft".to_string();
    }
    // Extract the CN= value if present, else fall back to the raw string.
    for part in r.split(',') {
        let p = part.trim();
        if let Some(cn) = p.strip_prefix("CN=").or_else(|| p.strip_prefix("cn=")) {
            if !cn.is_empty() {
                return cn.to_string();
            }
        }
    }
    if r.is_empty() {
        "Third-party".to_string()
    } else {
        r.to_string()
    }
}

pub fn list_appx_impl() -> Vec<AppxPackage> {
    // Pipe-delimited so parsing is trivial and locale-independent.
    let script = "Get-AppxPackage | ForEach-Object { \
        \"$($_.Name)|$($_.PackageFullName)|$($_.Publisher)|$($_.IsFramework)|$($_.NonRemovable)|$($_.SignatureKind)\" }";
    let out = no_window_cmd(ps_exe())
        .args(["-NonInteractive", "-NoProfile", "-Command", script])
        .output();
    let text = match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).into_owned(),
        Err(_) => return vec![],
    };

    let mut pkgs: Vec<AppxPackage> = vec![];
    for line in text.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 6 {
            continue;
        }
        let raw_name = ps_field(parts[0]);
        let full = ps_field(parts[1]);
        if raw_name.is_empty() || full.is_empty() {
            continue;
        }
        let publisher = friendly_publisher(parts[2]);
        let is_framework = parts[3].trim().eq_ignore_ascii_case("true");
        let nonremovable = parts[4].trim().eq_ignore_ascii_case("true");
        let signature = ps_field(parts[5]);
        let is_system = signature.eq_ignore_ascii_case("system");

        let name_lower = raw_name.to_ascii_lowercase();
        let critical = PROTECTED.iter().any(|p| name_lower.contains(p));
        let bloat = bloat_lookup(&name_lower);

        // Protected unless it's known bloat we explicitly allow.
        let protected =
            (is_framework || nonremovable || critical || is_system) && bloat.is_none();
        let removable = !protected;

        let (display, category, recommended, note) = match bloat {
            Some((friendly, note)) => (
                friendly.to_string(),
                "bloat".to_string(),
                removable,
                note.to_string(),
            ),
            None => {
                // Prettify "Microsoft.WindowsCalculator" → "Windows Calculator"
                let pretty = raw_name
                    .rsplit('.')
                    .next()
                    .unwrap_or(&raw_name)
                    .to_string();
                if protected {
                    (pretty, "system".to_string(), false, "System component — protected".to_string())
                } else {
                    (pretty, "app".to_string(), false, "Reinstallable from the Microsoft Store".to_string())
                }
            }
        };

        pkgs.push(AppxPackage {
            id: full,
            name: display,
            publisher,
            category,
            removable,
            recommended,
            note,
        });
    }

    // Sort: recommended bloat first, then other removable apps, then protected.
    pkgs.sort_by(|a, b| {
        let rank = |p: &AppxPackage| match p.category.as_str() {
            "bloat" => 0,
            "app" => 1,
            _ => 2,
        };
        rank(a).cmp(&rank(b)).then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    pkgs
}

/// True if a PackageFullName contains only the characters Appx names use —
/// guards the PowerShell interpolation against anything unexpected.
fn safe_pfn(pfn: &str) -> bool {
    !pfn.is_empty()
        && pfn
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '~'))
}

pub fn remove_appx_impl(ids: Vec<String>) -> DebloatResult {
    // Re-validate every requested id against the live removable list, so a
    // protected package can never be removed even if the UI sends its id.
    let live = list_appx_impl();
    let removable: std::collections::HashSet<String> = live
        .iter()
        .filter(|p| p.removable)
        .map(|p| p.id.clone())
        .collect();

    let targets: Vec<String> = ids
        .into_iter()
        .filter(|id| removable.contains(id) && safe_pfn(id))
        .collect();

    if targets.is_empty() {
        return DebloatResult {
            success: false,
            categories_cleaned: 0,
            total_mb_freed: 0.0,
            errors: vec![],
            message: "No removable apps were selected.".to_string(),
        };
    }

    let array = targets
        .iter()
        .map(|p| format!("'{}'", p))
        .collect::<Vec<_>>()
        .join(",");
    let script = format!(
        "@({}) | ForEach-Object {{ try {{ Remove-AppxPackage -Package $_ -ErrorAction Stop; \"OK\" }} catch {{ \"ERR|$_|$($_.Exception.Message)\" }} }}",
        array
    );

    let out = no_window_cmd(ps_exe())
        .args(["-NonInteractive", "-NoProfile", "-Command", &script])
        .output();

    let mut removed = 0u32;
    let mut errors: Vec<String> = vec![];
    match out {
        Ok(o) => {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                let l = line.trim();
                if l == "OK" {
                    removed += 1;
                } else if let Some(rest) = l.strip_prefix("ERR|") {
                    errors.push(rest.to_string());
                }
            }
        }
        Err(e) => {
            return DebloatResult {
                success: false,
                categories_cleaned: 0,
                total_mb_freed: 0.0,
                errors: vec![e.to_string()],
                message: "Failed to run PowerShell.".to_string(),
            }
        }
    }

    let message = if removed == 0 {
        "No apps were removed.".to_string()
    } else if errors.is_empty() {
        format!("Removed {} app(s). They can be reinstalled from the Store.", removed)
    } else {
        format!("Removed {} app(s), {} could not be removed.", removed, errors.len())
    };

    DebloatResult {
        success: removed > 0,
        categories_cleaned: removed,
        total_mb_freed: 0.0,
        errors,
        message,
    }
}
