import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  RefreshCw,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  Cpu,
  MemoryStick,
  HardDrive,
  Clock,
  Layers,
  Wifi,
  ShieldCheck,
} from "lucide-react";

// ── Types matching the Rust SystemOverview struct (flat, snake_case) ────────

type DriverStatus = "ok" | "outdated" | "unknown";

interface DriverEntry {
  name: string;
  version: string;
  date: string;
  status: DriverStatus;
  note: string;
}

interface SystemOverview {
  // Real-time usage
  cpu_pct: number;
  ram_used_mb: number;
  ram_total_mb: number;
  disk_used_gb: number;
  disk_total_gb: number;
  uptime_secs: number;

  // CPU
  cpu_name: string;
  cpu_cores: number;
  cpu_threads: number;

  // GPU
  gpu_name: string;
  gpu_vram_gb: number;
  gpu_driver_version: string;

  // RAM
  ram_type: string;
  ram_speed_mhz: number;

  // Motherboard
  motherboard: string;

  // Storage (system drive)
  storage_name: string;
  storage_partition: string;
  storage_gb: number;
  storage_free_gb: number;
  storage_media_type: string;
  storage_bus_type: string;
  storage_type: string;
  storage_health: string;

  // OS
  os_name: string;
  os_build: string;
  os_version_tag: string;
  os_install_date: string;
  os_architecture: string;
  os_locale: string;
  os_hostname: string;

  // BIOS / UEFI
  bios_vendor: string;
  bios_version: string;
  bios_release_date: string;
  bios_mode: string;
  bios_secure_boot: boolean;
  bios_age_days: number;

  // Drivers
  drivers: DriverEntry[];
}

// ── Helpers ─────────────────────────────────────────────────────────────────

function fmtUptime(secs: number) {
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function fmtTime(ms: number) {
  return new Date(ms).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function gb(n: number) {
  return n > 0 ? `${n.toFixed(n % 1 === 0 ? 0 : 1)} GB` : "—";
}

function healthColor(status: string) {
  const s = status.toLowerCase();
  if (s === "healthy") return "var(--success)";
  if (s === "warning") return "var(--warning)";
  if (s === "unhealthy") return "var(--danger)";
  return "var(--subtle)";
}

// ── Health score ─────────────────────────────────────────────────────────────

function computeHealthScore(d: SystemOverview) {
  let score = 100;
  score -= d.drivers.filter((drv) => drv.status === "outdated").length * 10;
  if (d.bios_age_days > 730)  score -= 10; // >2 years
  if (d.bios_age_days > 1460) score -= 10; // >4 years
  if (!d.bios_secure_boot)    score -= 5;
  score = Math.max(0, score);
  if (score >= 90) return { score, label: "Excellent",       color: "var(--success)" };
  if (score >= 70) return { score, label: "Good",            color: "var(--info)"    };
  if (score >= 50) return { score, label: "Fair",            color: "var(--warning)" };
  return              { score, label: "Needs attention", color: "var(--danger)"  };
}

// ── Sub-components ───────────────────────────────────────────────────────────

function UsageBar({
  value,
  max = 100,
  warn = 70,
  danger = 90,
}: {
  value: number;
  max?: number;
  warn?: number;
  danger?: number;
}) {
  const pct = Math.min(100, Math.round((value / Math.max(max, 1)) * 100));
  const color =
    pct >= danger
      ? "var(--danger)"
      : pct >= warn
      ? "var(--warning)"
      : "var(--success)";
  return (
    <div className="sov-bar-track">
      <div className="sov-bar-fill" style={{ width: `${pct}%`, background: color }} />
    </div>
  );
}

function MetricCard({
  icon,
  label,
  primary,
  secondary,
  barValue,
  barMax,
  accentColor,
}: {
  icon: React.ReactNode;
  label: string;
  primary: string;
  secondary?: string;
  barValue?: number;
  barMax?: number;
  accentColor: string;
}) {
  return (
    <div className="sov-metric-card">
      <div
        className="sov-metric-icon"
        style={{ background: `${accentColor}18`, color: accentColor }}
      >
        {icon}
      </div>
      <div className="sov-metric-body">
        <div className="sov-metric-label">{label}</div>
        <div className="sov-metric-primary">{primary}</div>
        {secondary && <div className="sov-metric-secondary">{secondary}</div>}
        {barValue !== undefined && barMax !== undefined && (
          <UsageBar value={barValue} max={barMax} />
        )}
      </div>
    </div>
  );
}

function DriverStatusIcon({ status }: { status: DriverStatus }) {
  if (status === "ok")
    return <CheckCircle2 size={14} style={{ color: "var(--success)", flexShrink: 0 }} />;
  if (status === "outdated")
    return <AlertTriangle size={14} style={{ color: "var(--warning)", flexShrink: 0 }} />;
  return <XCircle size={14} style={{ color: "var(--subtle)", flexShrink: 0 }} />;
}

function InfoRow({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="sov-info-row">
      <span className="sov-info-label">{label}</span>
      <span className={`sov-info-value${mono ? " sov-info-value--mono" : ""}`}>
        {value || "Unknown"}
      </span>
    </div>
  );
}

function SectionCard({
  title,
  icon,
  children,
  accent,
}: {
  title: string;
  icon: React.ReactNode;
  children: React.ReactNode;
  accent?: string;
}) {
  return (
    <div className="sov-section-card">
      <div
        className="sov-section-header"
        style={{ borderLeftColor: accent ?? "var(--accent)" }}
      >
        <span className="sov-section-icon" style={{ color: accent ?? "var(--accent)" }}>
          {icon}
        </span>
        <span className="sov-section-title">{title}</span>
      </div>
      <div className="sov-section-body">{children}</div>
    </div>
  );
}

// ── Loading skeleton ──────────────────────────────────────────────────────────

function SkeletonBlock({ w = "100%", h = 14 }: { w?: string; h?: number }) {
  return (
    <div
      style={{
        width: w,
        height: h,
        borderRadius: 4,
        background: "var(--card)",
        opacity: 0.6,
        animation: "pulse 1.4s ease-in-out infinite",
      }}
    />
  );
}

function LoadingSkeleton() {
  return (
    <div className="page-wrapper">
      <div className="page-scroll">
        <div className="content-container">
          <div className="sov-header">
            <div className="sov-header-left">
              <div className="sov-header-icon">🖥️</div>
              <div>
                <h1 className="content-header-title">System Overview</h1>
                <p className="content-header-count">Reading hardware data…</p>
              </div>
            </div>
          </div>

          <div className="sov-metrics-grid" style={{ gap: 12 }}>
            {[0, 1, 2, 3].map((i) => (
              <div key={i} className="sov-metric-card">
                <div className="sov-metric-icon" style={{ background: "#ffffff08" }}>
                  <SkeletonBlock w="18px" h={18} />
                </div>
                <div className="sov-metric-body" style={{ gap: 6 }}>
                  <SkeletonBlock w="60%" h={10} />
                  <SkeletonBlock w="80%" h={18} />
                  <SkeletonBlock h={6} />
                </div>
              </div>
            ))}
          </div>

          <div className="sov-two-col">
            {[0, 1].map((i) => (
              <div key={i} className="sov-section-card">
                <div className="sov-section-header" style={{ borderLeftColor: "var(--card)" }}>
                  <SkeletonBlock w="120px" h={14} />
                </div>
                <div className="sov-section-body" style={{ gap: 10 }}>
                  {[0, 1, 2, 3, 4].map((j) => (
                    <SkeletonBlock key={j} w={`${70 + j * 5}%`} h={12} />
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Main page ─────────────────────────────────────────────────────────────────

export function SystemOverviewPage() {
  const [data, setData]           = useState<SystemOverview | null>(null);
  const [loading, setLoading]     = useState(true);
  const [error, setError]         = useState<string | null>(null);
  const [refreshedAt, setRefreshedAt] = useState(0);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<SystemOverview>("get_system_overview");
      setData(result);
      setRefreshedAt(Date.now());
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // Show skeleton while initial data is loading
  if (loading && data === null) return <LoadingSkeleton />;

  // Error state
  if (error && data === null) {
    return (
      <div className="page-wrapper">
        <div className="page-scroll">
          <div className="content-container">
            <div className="sov-header">
              <div className="sov-header-left">
                <div className="sov-header-icon">🖥️</div>
                <div>
                  <h1 className="content-header-title">System Overview</h1>
                  <p className="content-header-count">Failed to read system data</p>
                </div>
              </div>
            </div>
            <div className="sov-alert sov-alert--warn" style={{ margin: "12px 0" }}>
              <AlertTriangle size={13} />
              <span>{error}</span>
            </div>
            <button className="tools-icon-btn" onClick={refresh} style={{ padding: "6px 16px", gap: 6 }}>
              <RefreshCw size={13} />
              Retry
            </button>
          </div>
        </div>
      </div>
    );
  }

  if (!data) return null;

  const health   = computeHealthScore(data);
  const outdated = data.drivers.filter((d) => d.status === "outdated");

  const ramUsedGb  = (data.ram_used_mb  / 1024).toFixed(1);
  const ramTotalGb = (data.ram_total_mb / 1024).toFixed(0);

  // Build RAM label e.g. "DDR4-3600" or just "DDR4" if speed unknown
  const ramLabel =
    data.ram_speed_mhz > 0
      ? `${data.ram_type}-${data.ram_speed_mhz}`
      : data.ram_type;

  // GPU label — show VRAM if we know it
  const gpuLabel =
    data.gpu_vram_gb > 0
      ? `${data.gpu_name} — ${gb(data.gpu_vram_gb)} VRAM`
      : data.gpu_name;

  return (
    <div className="page-wrapper">
      <div className="page-scroll">
        <div className="content-container">

          {/* ── Header ──────────────────────────────────────────────────── */}
          <div className="sov-header">
            <div className="sov-header-left">
              <div className="sov-header-icon">🖥️</div>
              <div>
                <h1 className="content-header-title">System Overview</h1>
                <p className="content-header-count">
                  Hardware · OS · Drivers · BIOS
                </p>
              </div>
            </div>
            <div className="sov-header-right">
              <div
                className="sov-health-badge"
                style={{ borderColor: health.color, color: health.color }}
              >
                <ShieldCheck size={13} />
                <span>{health.label}</span>
                <span className="sov-health-score">{health.score}</span>
              </div>
              <button
                className="tools-icon-btn"
                onClick={refresh}
                disabled={loading}
                title="Refresh"
              >
                <RefreshCw size={13} className={loading ? "spin" : ""} />
              </button>
              {refreshedAt > 0 && (
                <span className="sov-refresh-time">
                  Updated {fmtTime(refreshedAt)}
                </span>
              )}
            </div>
          </div>

          {/* ── Alerts ──────────────────────────────────────────────────── */}
          {(outdated.length > 0 || data.bios_age_days > 730) && (
            <div className="sov-alerts">
              {outdated.map((d) => (
                <div key={d.name} className="sov-alert sov-alert--warn">
                  <AlertTriangle size={13} />
                  <span>
                    <strong>{d.name}</strong> driver is outdated
                    {d.note ? ` — ${d.note}` : ""}
                  </span>
                </div>
              ))}
              {data.bios_age_days > 730 && (
                <div className="sov-alert sov-alert--warn">
                  <AlertTriangle size={13} />
                  <span>
                    BIOS version <strong>{data.bios_version}</strong> is{" "}
                    {Math.floor(data.bios_age_days / 365)} years old — check{" "}
                    {data.bios_vendor.split(" ")[0]} support page for updates
                  </span>
                </div>
              )}
            </div>
          )}

          {/* ── Metric cards ─────────────────────────────────────────────── */}
          <div className="sov-metrics-grid">
            <MetricCard
              icon={<Cpu size={18} />}
              label="CPU Usage"
              primary={`${data.cpu_pct}%`}
              secondary={data.cpu_name}
              barValue={data.cpu_pct}
              barMax={100}
              accentColor="var(--accent)"
            />
            <MetricCard
              icon={<MemoryStick size={18} />}
              label="RAM Usage"
              primary={`${ramUsedGb} / ${ramTotalGb} GB`}
              secondary={ramLabel}
              barValue={data.ram_used_mb}
              barMax={data.ram_total_mb}
              accentColor="var(--info)"
            />
            <MetricCard
              icon={<HardDrive size={18} />}
              label="Storage (C:)"
              primary={
                data.disk_total_gb > 0
                  ? `${data.disk_used_gb.toFixed(0)} / ${data.disk_total_gb.toFixed(0)} GB`
                  : "—"
              }
              secondary={data.storage_type}
              barValue={data.disk_used_gb}
              barMax={data.disk_total_gb}
              accentColor="var(--warning)"
            />
            <MetricCard
              icon={<Clock size={18} />}
              label="System Uptime"
              primary={data.uptime_secs > 0 ? fmtUptime(data.uptime_secs) : "—"}
              secondary={data.os_hostname}
              accentColor="var(--success)"
            />
          </div>

          {/* ── Hardware + OS ────────────────────────────────────────────── */}
          <div className="sov-two-col">
            <SectionCard
              title="Hardware"
              icon={<Cpu size={14} />}
              accent="var(--accent)"
            >
              <InfoRow
                label="CPU"
                value={
                  data.cpu_cores > 0
                    ? `${data.cpu_name} (${data.cpu_cores}C / ${data.cpu_threads}T)`
                    : data.cpu_name
                }
              />
              <InfoRow label="GPU" value={gpuLabel} />
              <InfoRow
                label="RAM"
                value={
                  data.ram_total_mb > 0
                    ? `${ramTotalGb} GB ${ramLabel}`
                    : data.ram_type
                }
              />
              <InfoRow label="Motherboard" value={data.motherboard} />
            </SectionCard>

            <SectionCard
              title="Operating System"
              icon={<Layers size={14} />}
              accent="var(--info)"
            >
              <InfoRow
                label="OS"
                value={
                  data.os_version_tag
                    ? `${data.os_name} (${data.os_version_tag})`
                    : data.os_name
                }
              />
              <InfoRow label="Build" value={data.os_build} mono />
              <InfoRow label="Architecture" value={data.os_architecture} />
              <InfoRow label="Hostname" value={data.os_hostname} mono />
              <InfoRow label="Locale" value={data.os_locale} />
              <InfoRow label="Installed" value={data.os_install_date} />
            </SectionCard>
          </div>

          {/* ── Storage + BIOS ───────────────────────────────────────────── */}
          <div className="sov-two-col">
            <SectionCard
              title="Storage (System Drive)"
              icon={<HardDrive size={14} />}
              accent="var(--warning)"
            >
              <InfoRow label="Model" value={data.storage_name} />
              <InfoRow label="System Partition" value={data.storage_partition} mono />
              <InfoRow
                label="Type"
                value={data.storage_type}
              />
              <InfoRow
                label="Media Type"
                value={data.storage_media_type}
              />
              <InfoRow
                label="Bus Type"
                value={data.storage_bus_type}
              />
              <InfoRow label="Capacity" value={gb(data.storage_gb)} />
              <InfoRow
                label="Free Space"
                value={
                  data.disk_total_gb > 0
                    ? `${gb(data.storage_free_gb)} of ${gb(data.disk_total_gb)}`
                    : gb(data.storage_free_gb)
                }
              />
              <div className="sov-info-row">
                <span className="sov-info-label">Health</span>
                <span
                  className="sov-info-value"
                  style={{ color: healthColor(data.storage_health), fontWeight: 600 }}
                >
                  {data.storage_health}
                </span>
              </div>
              {(data.storage_media_type === "Unknown" ||
                data.storage_type === "Unknown") && (
                <p className="sov-driver-note-footer" style={{ marginTop: 4 }}>
                  Drive type could not be read reliably from the storage
                  subsystem (Get-PhysicalDisk) — shown as Unknown rather than a
                  guess.
                </p>
              )}
            </SectionCard>

            <SectionCard
              title="BIOS / UEFI"
              icon={<ShieldCheck size={14} />}
              accent="var(--warning)"
            >
              <InfoRow label="Vendor"        value={data.bios_vendor}       />
              <InfoRow label="Version"       value={data.bios_version} mono />
              <InfoRow label="Release Date"  value={data.bios_release_date} />
              <InfoRow label="Firmware Mode" value={data.bios_mode}         />
              <InfoRow
                label="Secure Boot"
                value={data.bios_secure_boot ? "Enabled ✓" : "Disabled ✗"}
              />
              <InfoRow
                label="Age"
                value={
                  data.bios_age_days === 0
                    ? "Unknown"
                    : data.bios_age_days < 365
                    ? `${data.bios_age_days} days`
                    : `${(data.bios_age_days / 365).toFixed(1)} years`
                }
              />
              <div
                className={`sov-bios-status ${
                  data.bios_age_days > 730
                    ? "sov-bios-status--warn"
                    : "sov-bios-status--ok"
                }`}
              >
                {data.bios_age_days > 730 ? (
                  <>
                    <AlertTriangle size={13} />
                    <span>
                      BIOS is over {Math.floor(data.bios_age_days / 365)} years
                      old — check for firmware updates on your motherboard's
                      support page
                    </span>
                  </>
                ) : (
                  <>
                    <CheckCircle2 size={13} />
                    <span>BIOS appears reasonably up to date</span>
                  </>
                )}
              </div>
              {!data.bios_secure_boot && (
                <div className="sov-bios-status sov-bios-status--warn">
                  <AlertTriangle size={13} />
                  <span>
                    Secure Boot is disabled — consider enabling it in UEFI
                    settings
                  </span>
                </div>
              )}
            </SectionCard>
          </div>

          {/* ── Drivers ──────────────────────────────────────────────────── */}
          <div className="sov-two-col">
            <SectionCard
              title="Driver Status"
              icon={<Wifi size={14} />}
              accent="var(--success)"
            >
              {data.drivers.length === 0 ? (
                <p className="sov-driver-note-footer" style={{ marginTop: 0 }}>
                  No driver information available — driver enumeration requires
                  elevated privileges or may not be supported on this system.
                </p>
              ) : (
                data.drivers.map((d) => (
                  <div key={d.name} className="sov-driver-row">
                    <DriverStatusIcon status={d.status} />
                    <div className="sov-driver-info">
                      <span className="sov-driver-name">{d.name}</span>
                      {d.note && (
                        <span className="sov-driver-note">{d.note}</span>
                      )}
                    </div>
                    <span
                      className="sov-driver-version"
                      style={{
                        color:
                          d.status === "ok"
                            ? "var(--success)"
                            : d.status === "outdated"
                            ? "var(--warning)"
                            : "var(--subtle)",
                      }}
                    >
                      {d.version}
                    </span>
                    <span className="sov-driver-badge" data-status={d.status}>
                      {d.status === "ok"
                        ? "Up to date"
                        : d.status === "outdated"
                        ? "Outdated"
                        : "Unknown"}
                    </span>
                  </div>
                ))
              )}
              <p className="sov-driver-note-footer">
                Driver versions are read from Win32_VideoController — verify
                via Device Manager for full detail.
              </p>
            </SectionCard>
          </div>

        </div>
      </div>
    </div>
  );
}
