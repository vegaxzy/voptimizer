import { useState, useEffect, useMemo } from "react";
import { Play, RotateCcw, Lock, RefreshCw, Zap } from "lucide-react";
import type { TweakState, LogEntry } from "../types";
import type { ProcessInfo } from "../invoke/minecraft";
import { useMinecraft } from "../hooks/useMinecraft";
import { RiskModal } from "../components/RiskModal";
import { BottomLogDrawer } from "../components/BottomLogDrawer";
import { AdminBanner } from "../components/AdminBanner";
import { cn } from "../lib/cn";

// ── Preset tweak IDs ────────────────────────────────────────────────────────

const PRESET_TWEAK_IDS = [
  "disable-gamedvr",
  "disable-game-bar-capture",
  "set-ultimate-performance",
  "disable-edge-startup-boost",
  "disable-edge-background-mode",
  "disable-windows-tips",
];

// ── Sub-components ──────────────────────────────────────────────────────────

function StatusRow({ label, value, ok }: { label: string; value: string; ok?: boolean }) {
  return (
    <div className="mc-status-row">
      <span className="mc-status-label">{label}</span>
      <span className={cn(
        "mc-status-value",
        ok === false && "mc-status-bad",
        ok === true  && "mc-status-good"
      )}>
        {value}
      </span>
    </div>
  );
}

function SectionHeader({ icon, title, subtitle }: { icon: string; title: string; subtitle?: string }) {
  return (
    <div className="mc-section-header">
      <span className="mc-section-icon">{icon}</span>
      <div>
        <h2 className="mc-section-title">{title}</h2>
        {subtitle && <p className="mc-section-subtitle">{subtitle}</p>}
      </div>
    </div>
  );
}

function InstructionCard({ title, items }: { title: string; items: string[] }) {
  return (
    <div className="mc-instruction-card">
      <p className="mc-instruction-title">{title}</p>
      <ul className="mc-instruction-list">
        {items.map((item, i) => (
          <li key={i}>{item}</li>
        ))}
      </ul>
    </div>
  );
}

function PlaceholderTweak({
  name, benefit, risk, note,
}: {
  name: string; benefit: string; risk: string; note: string;
}) {
  return (
    <div className="mc-placeholder-tweak">
      <div className="mc-placeholder-header">
        <span className="mc-placeholder-name">{name}</span>
        <span className="exp-badge exp-badge--placeholder">Research Required</span>
      </div>
      <p className="mc-placeholder-benefit">{benefit}</p>
      <p className="mc-placeholder-risk">{risk}</p>
      <p className="mc-placeholder-note">{note}</p>
    </div>
  );
}

// ── MetricCard ───────────────────────────────────────────────────────────────

function MetricCard({
  label, value, sub, status,
}: {
  label: string; value: string; sub?: string; status?: "ok" | "warn" | "neutral";
}) {
  return (
    <div className="metric-card">
      <p className="metric-card-label">{label}</p>
      <p className={cn(
        "metric-card-value",
        status === "ok"   && "metric-card-value--ok",
        status === "warn" && "metric-card-value--warn",
      )}>
        {value}
      </p>
      {sub && <p className="metric-card-sub">{sub}</p>}
    </div>
  );
}

// ── Kill confirmation modal ─────────────────────────────────────────────────

function KillModal({
  process, onConfirm, onCancel,
}: {
  process: ProcessInfo; onConfirm: () => void; onCancel: () => void;
}) {
  return (
    <div className="modal-overlay" onClick={onCancel}>
      <div className="modal-box" onClick={(e) => e.stopPropagation()}>
        <h2 className="modal-title">End Process?</h2>
        <p className="modal-body">
          Terminate <strong>{process.name}</strong> (PID {process.pid}, {process.memory_mb.toFixed(1)} MB)?
          Unsaved work in this process will be lost.
        </p>
        <div className="modal-actions">
          <button className="btn btn--revert" onClick={onConfirm}>End Process</button>
          <button className="btn btn--cancel" onClick={onCancel}>Cancel</button>
        </div>
      </div>
    </div>
  );
}

// ── Preset modal ────────────────────────────────────────────────────────────

function PresetModal({
  tweakStates, onConfirm, onCancel,
}: {
  tweakStates: Record<string, TweakState>; onConfirm: () => void; onCancel: () => void;
}) {
  const items = PRESET_TWEAK_IDS.map((id) => tweakStates[id]?.tweak);
  return (
    <div className="modal-overlay" onClick={onCancel}>
      <div className="modal-box modal-box--wide" onClick={(e) => e.stopPropagation()}>
        <h2 className="modal-title">Apply Minecraft Preset</h2>
        <p className="modal-body">
          The following tweaks will be applied. Each can be reverted individually afterwards.
        </p>
        <ul className="mc-preset-list">
          {items.map((t) =>
            t ? (
              <li key={t.id} className="mc-preset-item">
                <span className={`risk-badge risk-badge--${t.riskLevel}`}>{t.riskLevel}</span>
                <span>{t.name}</span>
                {tweakStates[t.id]?.isApplied && (
                  <span className="mc-preset-already">already applied</span>
                )}
              </li>
            ) : null
          )}
        </ul>
        <div className="modal-actions">
          <button className="btn btn--apply" onClick={onConfirm}>Apply All</button>
          <button className="btn btn--cancel" onClick={onCancel}>Cancel</button>
        </div>
      </div>
    </div>
  );
}

// ── Tweak row action (compact, no redundant admin button) ───────────────────

function TweakRowAction({
  state, isAdmin, onRequestApply, onRevert,
}: {
  state: TweakState; isAdmin: boolean;
  onRequestApply: (id: string) => void;
  onRevert: (id: string) => void;
}) {
  const id = state.tweak.id;
  if (state.tweak.requiresAdmin && !isAdmin) {
    return (
      <button className="btn btn--ghost btn--sm" disabled title="Requires administrator">
        <Lock size={10} strokeWidth={2} />
        Apply
      </button>
    );
  }
  if (state.isApplied) {
    return (
      <button
        className="btn btn--revert btn--sm"
        onClick={() => onRevert(id)}
        disabled={state.status === "reverting"}
      >
        <RotateCcw size={10} strokeWidth={2.5} />
        {state.status === "reverting" ? "…" : "Revert"}
      </button>
    );
  }
  return (
    <button
      className="btn btn--apply btn--sm"
      onClick={() => onRequestApply(id)}
      disabled={state.status === "applying"}
    >
      <Play size={10} strokeWidth={2.5} />
      {state.status === "applying" ? "…" : "Apply"}
    </button>
  );
}

// ── Tweak row (mc style) ────────────────────────────────────────────────────

function McTweakRow({
  id, tweakStates, isAdmin, onRequestApply, onRevert,
}: {
  id: string;
  tweakStates: Record<string, TweakState>;
  isAdmin: boolean;
  onRequestApply: (id: string) => void;
  onRevert: (id: string) => void;
}) {
  const state = tweakStates[id];
  if (!state) return null;
  return (
    <div className={cn(
      "mc-tweak-row",
      state.tweak.isExperimental && "mc-tweak-row--exp",
      state.isApplied && "mc-tweak-row--applied",
      state.tweak.requiresAdmin && !isAdmin && "mc-tweak-row--locked"
    )}>
      <div className="mc-tweak-row-info">
        <span className="mc-tweak-row-name">{state.tweak.name}</span>
        <p className="mc-tweak-row-desc">{state.tweak.description}</p>
      </div>
      <div className="mc-tweak-row-action">
        <TweakRowAction
          state={state}
          isAdmin={isAdmin}
          onRequestApply={onRequestApply}
          onRevert={onRevert}
        />
      </div>
    </div>
  );
}

// ── Main page ───────────────────────────────────────────────────────────────

interface MinecraftPageProps {
  tweakStates: Record<string, TweakState>;
  hasNvidia: boolean;
  hasAmd: boolean;
  isAdmin: boolean;
  onRequestApply: (id: string) => void;
  onRevert: (id: string) => void;
  onConfirmApply: () => void;
  onCancelApply: () => void;
  pendingApplyId: string | null;
  applyPreset: (ids: string[]) => void;
  onRestartAsAdmin: () => void;
}

export function MinecraftPage({
  tweakStates,
  hasNvidia,
  hasAmd,
  isAdmin,
  onRequestApply,
  onRevert,
  onConfirmApply,
  onCancelApply,
  pendingApplyId,
  applyPreset,
  onRestartAsAdmin,
}: MinecraftPageProps) {
  const {
    systemInfo,
    systemInfoStatus,
    refreshSystemInfo,
    processes,
    processesStatus,
    refreshProcesses,
    terminateProcess,
    dnsInfo,
    dnsStatus,
    refreshDnsInfo,
    doFlushDns,
    pingResults,
    pingStatus,
    runPing,
    mcLog,
  } = useMinecraft();

  const [showPresetModal, setShowPresetModal] = useState(false);
  const [pendingKill, setPendingKill] = useState<ProcessInfo | null>(null);
  const [flushMsg, setFlushMsg] = useState<string | null>(null);

  useEffect(() => {
    refreshSystemInfo();
    refreshDnsInfo();
  }, [refreshSystemInfo, refreshDnsInfo]);

  const gpuVendor = systemInfo?.gpu_vendor ?? "";

  async function handleFlushDns() {
    const result = await doFlushDns();
    setFlushMsg(result.message);
    setTimeout(() => setFlushMsg(null), 4000);
  }

  async function handleKillConfirm() {
    if (pendingKill) {
      await terminateProcess(pendingKill.pid, pendingKill.name);
      setPendingKill(null);
    }
  }

  function handlePresetConfirm() {
    setShowPresetModal(false);
    applyPreset(PRESET_TWEAK_IDS);
  }

  const presetApplied = PRESET_TWEAK_IDS.filter((id) => tweakStates[id]?.isApplied).length;

  // Convert mcLog string[] → LogEntry[] for BottomLogDrawer
  const logEntries: LogEntry[] = useMemo(
    () =>
      mcLog.map((msg, i) => ({
        id: `mc-${i}`,
        timestamp: new Date(),
        message: msg,
        level: "info" as const,
      })),
    [mcLog]
  );

  return (
    <div className="mc-page">
      <div className="page-scroll">
      <div className="content-container">
        {/* ── Admin banner ─────────────────────────────────────────── */}
        {!isAdmin && <AdminBanner onRestartAsAdmin={onRestartAsAdmin} />}

        {/* ── Page header ─────────────────────────────────────────── */}
        <header className="content-header">
          <span className="content-header-icon">🎮</span>
          <h1 className="content-header-title">Minecraft Optimizer</h1>
          {systemInfo?.minecraft_running && (
            <span className="mc-running-badge">● Minecraft Running</span>
          )}
          <div style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 8 }}>
            <button
              className="btn btn--ghost btn--sm"
              onClick={refreshSystemInfo}
              disabled={systemInfoStatus === "loading"}
            >
              <RefreshCw size={11} strokeWidth={2} className={systemInfoStatus === "loading" ? "spin" : ""} />
              {systemInfoStatus === "loading" ? "Loading…" : "Refresh"}
            </button>
          </div>
        </header>

        {/* ── Hero: 4 MetricCards ─────────────────────────────────── */}
        <div className="metric-grid">
          <MetricCard
            label="CPU"
            value={systemInfo?.cpu_name ?? "—"}
            sub={systemInfo ? `${systemInfo.ram_used_mb} / ${systemInfo.ram_total_mb} MB RAM` : undefined}
            status="neutral"
          />
          <MetricCard
            label="GPU"
            value={systemInfo?.gpu_name ?? "—"}
            sub={systemInfo?.gpu_vendor ? `${systemInfo.gpu_vendor.toUpperCase()} detected` : undefined}
            status="neutral"
          />
          <MetricCard
            label="Power Plan"
            value={systemInfo?.power_plan_name ?? "—"}
            status={
              systemInfo?.power_plan_name?.toLowerCase().includes("ultimate") ? "ok"
              : systemInfo?.power_plan_name?.toLowerCase().includes("high") ? "ok"
              : systemInfo ? "warn"
              : "neutral"
            }
          />
          <MetricCard
            label="Game DVR"
            value={
              systemInfo == null ? "—"
              : systemInfo.gamedvr_enabled ? "Enabled"
              : "Disabled"
            }
            sub={systemInfo?.gamedvr_enabled ? "Impacts FPS" : undefined}
            status={
              systemInfo == null ? "neutral"
              : systemInfo.gamedvr_enabled ? "warn"
              : "ok"
            }
          />
        </div>

        {/* ── Preset card ─────────────────────────────────────────── */}
        <div className="mc-section">
          <div className="mc-preset-card">
            <div className="mc-preset-info">
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                <Zap size={14} strokeWidth={2} style={{ color: "var(--accent)" }} />
                <p className="mc-section-title" style={{ margin: 0 }}>Recommended Preset</p>
                {presetApplied > 0 && (
                  <span className="applied-pill">
                    {presetApplied}/{PRESET_TWEAK_IDS.length} applied
                  </span>
                )}
              </div>
              <p className="mc-preset-desc">
                Disables GameDVR, Game Bar capture, sets Ultimate Performance power plan,
                disables Edge startup boost & background mode, and disables Windows tips.
              </p>
            </div>
            <button className="btn btn--apply btn--lg" onClick={() => setShowPresetModal(true)}>
              Apply Preset
            </button>
          </div>

          <div className="mc-preset-tweaks-grid">
            {PRESET_TWEAK_IDS.map((id) => {
              const state = tweakStates[id];
              if (!state) return null;
              return (
                <div key={id} className={cn("mc-mini-tweak", state.isApplied && "mc-mini-tweak--applied")}>
                  <span className="mc-mini-tweak-name">{state.tweak.name}</span>
                  {state.isApplied ? (
                    <button
                      className="btn btn--revert btn--xs"
                      onClick={() => onRevert(id)}
                      disabled={state.status === "reverting"}
                    >
                      {state.status === "reverting" ? "…" : "Revert"}
                    </button>
                  ) : (
                    <button
                      className="btn btn--apply btn--xs"
                      onClick={() => onRequestApply(id)}
                      disabled={state.status === "applying"}
                    >
                      {state.status === "applying" ? "…" : "Apply"}
                    </button>
                  )}
                </div>
              );
            })}
          </div>
        </div>

        {/* ── 2-column sections grid ──────────────────────────────── */}
        <div className="mc-sections-grid">

          {/* ── CPU & Scheduler ── */}
          <section className="mc-section">
            <SectionHeader
              icon="🔧"
              title="CPU & Scheduler"
              subtitle="Power and scheduler tweaks for better Java performance."
            />
            <div className="mc-tweak-row-grid">
              {["set-ultimate-performance", "disable-power-throttling", "system-responsiveness", "network-throttling-index"].map((id) => (
                <McTweakRow
                  key={id}
                  id={id}
                  tweakStates={tweakStates}
                  isAdmin={isAdmin}
                  onRequestApply={onRequestApply}
                  onRevert={onRevert}
                />
              ))}
            </div>
          </section>

          {/* ── GPU Optimization ── */}
          <section className="mc-section">
            <SectionHeader
              icon="🖥️"
              title="GPU Optimization"
              subtitle="GPU-specific tweaks for your detected graphics card."
            />

            {gpuVendor === "nvidia" || hasNvidia ? (
              <div className="mc-gpu-section">
                <p className="mc-gpu-label">NVIDIA GPU Detected</p>
                <div className="mc-tweak-row-grid">
                  {["disable-nvidia-telemetry", "disable-nvidia-overlay-startup"].map((id) => (
                    <McTweakRow
                      key={id}
                      id={id}
                      tweakStates={tweakStates}
                      isAdmin={isAdmin}
                      onRequestApply={onRequestApply}
                      onRevert={onRevert}
                    />
                  ))}
                </div>
                <InstructionCard
                  title="NVIDIA Control Panel — Recommended"
                  items={[
                    "Power management mode → Prefer maximum performance",
                    "Texture filtering quality → High performance",
                    "Vertical sync → Off (control in-game)",
                    "Low Latency Mode → Ultra (NVCP v4.0+)",
                  ]}
                />
              </div>
            ) : gpuVendor === "amd" || hasAmd ? (
              <div className="mc-gpu-section">
                <p className="mc-gpu-label">AMD GPU Detected</p>
                <div className="mc-tweak-row-grid">
                  {["disable-amd-telemetry", "disable-amd-radeon-autostart"].map((id) => (
                    <McTweakRow
                      key={id}
                      id={id}
                      tweakStates={tweakStates}
                      isAdmin={isAdmin}
                      onRequestApply={onRequestApply}
                      onRevert={onRevert}
                    />
                  ))}
                </div>
                <InstructionCard
                  title="Radeon Software — Recommended"
                  items={[
                    "Open Radeon Software → Gaming → Minecraft",
                    "Anti-Lag → Enabled",
                    "Texture Filtering Quality → Performance",
                    "Frame Rate Target Control → Off",
                  ]}
                />
              </div>
            ) : (
              <div className="mc-gpu-section">
                <p className="mc-gpu-label">
                  {systemInfo ? `${systemInfo.gpu_name} (Intel / Unknown)` : "GPU not detected yet"}
                </p>
                <p className="mc-status-value">
                  No vendor-specific tweaks available. Ensure your graphics drivers are up to date.
                </p>
              </div>
            )}
          </section>

          {/* ── Network ── */}
          <section className="mc-section">
            <SectionHeader
              icon="🌐"
              title="Network"
              subtitle="DNS and network tools for Minecraft multiplayer latency."
            />
            <div className="mc-tweak-row-grid">
              {["disable-delivery-optimization"].map((id) => (
                <McTweakRow
                  key={id}
                  id={id}
                  tweakStates={tweakStates}
                  isAdmin={isAdmin}
                  onRequestApply={onRequestApply}
                  onRevert={onRevert}
                />
              ))}
            </div>

            <div className="mc-network-controls">
              {/* DNS panel */}
              <div className="mc-dns-panel">
                <div className="mc-dns-top">
                  <p className="mc-dns-title">DNS</p>
                  <button
                    className="btn btn--ghost btn--sm"
                    onClick={refreshDnsInfo}
                    disabled={dnsStatus === "loading"}
                  >
                    {dnsStatus === "loading" ? "…" : "Refresh"}
                  </button>
                </div>
                {dnsInfo ? (
                  <>
                    <p className="mc-dns-hostname">Host: {dnsInfo.hostname}</p>
                    {dnsInfo.servers.length > 0 ? (
                      <ul className="mc-dns-servers">
                        {dnsInfo.servers.map((s) => <li key={s}>{s}</li>)}
                      </ul>
                    ) : (
                      <p className="mc-status-value">No DNS servers found</p>
                    )}
                  </>
                ) : (
                  <p className="mc-loading">Loading DNS info…</p>
                )}
                <button
                  className="btn btn--apply btn--sm"
                  style={{ marginTop: 8 }}
                  onClick={handleFlushDns}
                >
                  Flush DNS Cache
                </button>
                {flushMsg && <p className="mc-flush-msg">{flushMsg}</p>}
              </div>

              {/* Ping panel */}
              <div className="mc-ping-panel">
                <div className="mc-dns-top">
                  <p className="mc-dns-title">Ping Test</p>
                  <button
                    className="btn btn--ghost btn--sm"
                    onClick={() => runPing(["1.1.1.1", "8.8.8.8"])}
                    disabled={pingStatus === "loading"}
                  >
                    {pingStatus === "loading" ? "Pinging…" : "Run"}
                  </button>
                </div>
                {Object.entries(pingResults).length > 0 ? (
                  <div className="mc-ping-results">
                    {Object.entries(pingResults).map(([host, result]) => (
                      <div key={host} className="mc-ping-row">
                        <span className="mc-ping-host">{host}</span>
                        {result.success ? (
                          <span className={cn("mc-ping-latency", (result.latency_ms ?? 999) < 50 ? "mc-status-good" : "mc-status-bad")}>
                            {result.latency_ms}ms
                          </span>
                        ) : (
                          <span className="mc-ping-latency mc-status-bad">Timeout</span>
                        )}
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="mc-loading">Click Run to test latency to 1.1.1.1 and 8.8.8.8.</p>
                )}
              </div>
            </div>
          </section>

          {/* ── Background Processes ── */}
          <section className="mc-section">
            <div className="mc-section-top">
              <SectionHeader
                icon="📋"
                title="Background Processes"
                subtitle="High-memory processes competing with Minecraft."
              />
              <button
                className="btn btn--ghost btn--sm"
                onClick={refreshProcesses}
                disabled={processesStatus === "loading"}
              >
                <RefreshCw size={11} strokeWidth={2} className={processesStatus === "loading" ? "spin" : ""} />
                {processesStatus === "loading" ? "Scanning…" : processes.length === 0 ? "Scan" : "Refresh"}
              </button>
            </div>

            {processes.length > 0 ? (
              <div className="mc-process-table-wrap">
                <table className="mc-process-table">
                  <thead>
                    <tr>
                      <th>Process</th>
                      <th>PID</th>
                      <th>Memory</th>
                      <th></th>
                    </tr>
                  </thead>
                  <tbody>
                    {processes.map((p) => (
                      <tr key={p.pid} className={p.is_safe_to_kill ? "" : "mc-process-row--system"}>
                        <td>{p.name}</td>
                        <td>{p.pid}</td>
                        <td>{p.memory_mb.toFixed(1)} MB</td>
                        <td>
                          {p.is_safe_to_kill ? (
                            <button
                              className="btn btn--revert btn--xs"
                              onClick={() => setPendingKill(p)}
                            >
                              End
                            </button>
                          ) : (
                            <span className="mc-process-protected">Protected</span>
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : processesStatus === "idle" ? (
              <p className="mc-loading">Click Scan to list high-memory background processes.</p>
            ) : processesStatus === "loading" ? (
              <p className="mc-loading">Scanning processes…</p>
            ) : (
              <p className="mc-loading">No high-memory background processes found.</p>
            )}
          </section>

        </div>{/* end mc-sections-grid */}

        {/* ── System Status detail (collapsible) ─────────────────── */}
        {systemInfo && (
          <section className="mc-section">
            <SectionHeader
              icon="📊"
              title="System Status"
              subtitle="Full hardware and Windows configuration snapshot."
            />
            <div className="mc-status-grid">
              <div className="mc-status-card">
                <p className="mc-status-card-title">Hardware</p>
                <StatusRow label="CPU" value={systemInfo.cpu_name} />
                <StatusRow label="GPU" value={`${systemInfo.gpu_name} (${systemInfo.gpu_vendor})`} />
                <StatusRow label="RAM" value={`${systemInfo.ram_used_mb} / ${systemInfo.ram_total_mb} MB used`} />
              </div>
              <div className="mc-status-card">
                <p className="mc-status-card-title">Power & Performance</p>
                <StatusRow label="Power Plan" value={systemInfo.power_plan_name} />
                <StatusRow
                  label="Startup Apps"
                  value={String(systemInfo.startup_app_count)}
                  ok={systemInfo.startup_app_count <= 5}
                />
              </div>
              <div className="mc-status-card">
                <p className="mc-status-card-title">Gaming Features</p>
                <StatusRow
                  label="Game DVR"
                  value={systemInfo.gamedvr_enabled ? "Enabled (impacts FPS)" : "Disabled"}
                  ok={!systemInfo.gamedvr_enabled}
                />
                <StatusRow
                  label="Game Bar Capture"
                  value={systemInfo.game_bar_capture_enabled ? "Enabled" : "Disabled"}
                  ok={!systemInfo.game_bar_capture_enabled}
                />
                <StatusRow
                  label="Animations"
                  value={systemInfo.animations_enabled ? "Enabled" : "Disabled"}
                  ok={!systemInfo.animations_enabled}
                />
              </div>
              <div className="mc-status-card">
                <p className="mc-status-card-title">Edge / Startup</p>
                <StatusRow
                  label="Edge Startup Boost"
                  value={systemInfo.edge_startup_boost_enabled ? "Enabled" : "Disabled"}
                  ok={!systemInfo.edge_startup_boost_enabled}
                />
                <StatusRow
                  label="Edge Background"
                  value={systemInfo.edge_background_enabled ? "Enabled" : "Disabled"}
                  ok={!systemInfo.edge_background_enabled}
                />
              </div>
            </div>
          </section>
        )}

        {/* ── Experimental & Placeholders ─────────────────────────── */}
        <section className="mc-section">
          <SectionHeader
            icon="🧪"
            title="Experimental & Placeholders"
            subtitle="These tweaks require more research before implementation. Shown for reference only."
          />
          <div className="mc-placeholder-grid">
            <PlaceholderTweak
              name="BCD Timer Resolution"
              benefit="May reduce frame time variance by forcing a fixed system timer interval."
              risk="Invalid BCD entry can prevent Windows from booting. Requires WinPE to recover."
              note="Benefit on modern hardware is minimal. Not implemented."
            />
            <PlaceholderTweak
              name="Disable HPET"
              benefit="Forces TSC clock for timing; can improve frame pacing on some Intel CPUs."
              risk="May cause audio glitches. No benefit on AMD Ryzen (already uses TSC)."
              note="Hardware-dependent. Not implemented."
            />
            <PlaceholderTweak
              name="NVIDIA MSI Interrupt Mode"
              benefit="Reduces GPU→CPU interrupt latency on compatible hardware."
              risk="Black screens or instability on unsupported GPU/driver combos."
              note="Modern drivers auto-configure MSI. Not implemented."
            />
            <PlaceholderTweak
              name="NVIDIA P-State Override"
              benefit="Forces GPU to stay at maximum P-state clock, reducing latency spikes."
              risk="Requires registry edit per-device. Incorrect values can destabilize display driver."
              note="Not implemented — driver profiles not editable."
            />
            <PlaceholderTweak
              name="RAM Standby List Flush"
              benefit="Frees standby RAM before launching Minecraft for a brief memory headroom boost."
              risk="Windows rebuilds its cache naturally within minutes. Short-lived effect."
              note="Requires EmptyStandbyList privilege call. Not implemented."
            />
          </div>
        </section>

      </div>
      </div>

      {/* ── Log drawer ─────────────────────────────────────────────── */}
      <BottomLogDrawer logs={logEntries} onClear={() => {/* mcLog is read-only from hook */}} />

      {/* ── Modals ─────────────────────────────────────────────────── */}
      {showPresetModal && (
        <PresetModal
          tweakStates={tweakStates}
          onConfirm={handlePresetConfirm}
          onCancel={() => setShowPresetModal(false)}
        />
      )}
      {pendingKill && (
        <KillModal
          process={pendingKill}
          onConfirm={handleKillConfirm}
          onCancel={() => setPendingKill(null)}
        />
      )}
      {pendingApplyId && tweakStates[pendingApplyId] && (
        <RiskModal
          tweak={tweakStates[pendingApplyId].tweak}
          onConfirm={onConfirmApply}
          onCancel={onCancelApply}
        />
      )}
    </div>
  );
}
