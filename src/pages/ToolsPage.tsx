import { useState, useEffect, useCallback, useRef } from "react";
import { toast } from "sonner";
import {
  RefreshCw,
  Activity,
  Layers,
  Gamepad2,
  Cpu,
  BarChart2,
  AlertTriangle,
  CheckCircle2,
  Circle,
  Trash2,
  Play,
  Square,
  Camera,
  RotateCcw,
} from "lucide-react";
import { AdminBanner } from "../components/AdminBanner";
import {
  detectOverlays,
  scanBackgroundLoad,
  getShaderCaches,
  cleanShaderCaches,
  startGameSession,
  endGameSession,
  getGameSessionStatus,
  getMinecraftMonitor,
  takeSnapshot,
  getBenchmarkState,
  getBenchmarkComparison,
  clearBenchmark,
} from "../invoke/tools";
import type {
  OverlayInfo,
  ProcessLoad,
  ShaderCacheEntry,
  GameSessionStatus,
  MinecraftMonitor,
  SystemSnapshot,
  BenchmarkComparison,
  BenchmarkStateResult,
} from "../invoke/tools";

interface ToolsPageProps {
  isAdmin: boolean;
  onRestartAsAdmin: () => void;
}

// ── Utility ────────────────────────────────────────────────────────────────

function fmtMB(mb: number) {
  if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`;
  return `${mb.toFixed(0)} MB`;
}

function fmtDuration(secs: number) {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}

function fmtTimestamp(ms: number) {
  if (!ms) return "—";
  return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

function fmtUptime(secs: number) {
  const days = Math.floor(secs / 86400);
  const hours = Math.floor((secs % 86400) / 3600);
  const mins = Math.floor((secs % 3600) / 60);
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${mins}m`;
  return `${mins}m`;
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. OVERLAY DETECTOR CARD
// ═══════════════════════════════════════════════════════════════════════════

function OverlayDetectorCard() {
  const [overlays, setOverlays] = useState<OverlayInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [scanned, setScanned] = useState(false);

  const scan = useCallback(async () => {
    setLoading(true);
    try {
      const data = await detectOverlays();
      setOverlays(data);
      setScanned(true);
    } catch (e) {
      toast.error("Overlay scan failed", { description: String(e) });
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    scan();
  }, [scan]);

  const detected = overlays.filter((o) => o.detected);
  const notDetected = overlays.filter((o) => !o.detected);

  const categoryColor: Record<string, string> = {
    communication: "var(--info)",
    gaming: "var(--accent)",
    recording: "var(--warning)",
    performance: "var(--success)",
  };

  return (
    <div className="tools-card">
      <div className="tools-card-header">
        <div className="tools-card-title-row">
          <Activity size={16} className="tools-card-icon" style={{ color: "var(--info)" }} />
          <div>
            <div className="tools-card-title">Overlay Detector</div>
            <div className="tools-card-desc">
              Detects active game overlays that may impact performance
            </div>
          </div>
        </div>
        <button className="tools-icon-btn" onClick={scan} disabled={loading} title="Refresh">
          <RefreshCw size={13} className={loading ? "spin" : ""} />
        </button>
      </div>

      <div className="tools-card-body">
        {!scanned && !loading && (
          <p className="tools-empty">Scanning overlays…</p>
        )}

        {scanned && detected.length === 0 && (
          <div className="tools-status-ok">
            <CheckCircle2 size={14} />
            <span>No active overlays detected</span>
          </div>
        )}

        {detected.length > 0 && (
          <div className="tools-overlay-section">
            <p className="tools-section-label">
              Active ({detected.length})
            </p>
            {detected.map((o) => (
              <OverlayRow key={o.id} overlay={o} categoryColor={categoryColor} />
            ))}
          </div>
        )}

        {notDetected.length > 0 && scanned && (
          <div className="tools-overlay-section">
            <p className="tools-section-label" style={{ color: "var(--subtle)" }}>
              Not running ({notDetected.length})
            </p>
            {notDetected.map((o) => (
              <OverlayRow key={o.id} overlay={o} categoryColor={categoryColor} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function OverlayRow({
  overlay,
  categoryColor,
}: {
  overlay: OverlayInfo;
  categoryColor: Record<string, string>;
}) {
  const [showTip, setShowTip] = useState(false);
  return (
    <div
      className={`tools-overlay-row ${overlay.detected ? "tools-overlay-row--detected" : ""}`}
      onClick={() => overlay.detected && setShowTip((s) => !s)}
      style={{ cursor: overlay.detected ? "pointer" : "default" }}
    >
      <span
        className="tools-overlay-dot"
        style={{
          background: overlay.detected
            ? (categoryColor[overlay.category] ?? "var(--success)")
            : "var(--subtle)",
          opacity: overlay.detected ? 1 : 0.4,
        }}
      />
      <div className="tools-overlay-info">
        <span className="tools-overlay-name">{overlay.name}</span>
        <span className="tools-overlay-proc">{overlay.process_name}</span>
        {overlay.pid != null && (
          <span className="tools-overlay-pid">PID {overlay.pid}</span>
        )}
      </div>
      <span
        className="tools-overlay-cat"
        style={{ color: categoryColor[overlay.category] ?? "var(--muted)", opacity: overlay.detected ? 1 : 0.5 }}
      >
        {overlay.category}
      </span>

      {showTip && overlay.detected && (
        <div className="tools-overlay-tip">{overlay.tip}</div>
      )}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. BACKGROUND LOAD SCANNER CARD
// ═══════════════════════════════════════════════════════════════════════════

function BackgroundLoadCard() {
  const [processes, setProcesses] = useState<ProcessLoad[]>([]);
  const [loading, setLoading] = useState(false);
  const [scanned, setScanned] = useState(false);
  const [showAll, setShowAll] = useState(false);

  const scan = useCallback(async () => {
    setLoading(true);
    try {
      const data = await scanBackgroundLoad();
      setProcesses(data);
      setScanned(true);
    } catch (e) {
      toast.error("Scan failed", { description: String(e) });
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    scan();
  }, [scan]);

  const impactful = processes.filter((p) => p.is_gaming_impact);
  const displayed = showAll ? processes : processes.slice(0, 10);

  return (
    <div className="tools-card">
      <div className="tools-card-header">
        <div className="tools-card-title-row">
          <Cpu size={16} className="tools-card-icon" style={{ color: "var(--warning)" }} />
          <div>
            <div className="tools-card-title">Background Load Scanner</div>
            <div className="tools-card-desc">
              Top processes by memory — gaming-impactful ones highlighted
            </div>
          </div>
        </div>
        <button className="tools-icon-btn" onClick={scan} disabled={loading} title="Refresh">
          <RefreshCw size={13} className={loading ? "spin" : ""} />
        </button>
      </div>

      <div className="tools-card-body">
        {impactful.length > 0 && (
          <div className="tools-warning-banner">
            <AlertTriangle size={13} />
            <span>
              {impactful.length} gaming-impactful process{impactful.length !== 1 ? "es" : ""} detected
            </span>
          </div>
        )}

        {scanned && processes.length === 0 && (
          <p className="tools-empty">No processes found.</p>
        )}

        {displayed.length > 0 && (
          <div className="tools-proc-table">
            <div className="tools-proc-header">
              <span>Process</span>
              <span>RAM</span>
              <span>CPU (s)</span>
            </div>
            {displayed.map((p) => (
              <div
                key={`${p.pid}-${p.name}`}
                className={`tools-proc-row ${p.is_gaming_impact ? "tools-proc-row--impact" : ""}`}
                title={p.is_gaming_impact ? p.impact_reason : ""}
              >
                <span className="tools-proc-name">
                  {p.is_gaming_impact && (
                    <AlertTriangle
                      size={11}
                      style={{ color: "var(--warning)", flexShrink: 0 }}
                    />
                  )}
                  {p.name}
                  {p.is_gaming_impact && (
                    <span className="tools-proc-impact-label">{p.impact_reason}</span>
                  )}
                </span>
                <span className="tools-proc-ram">{fmtMB(p.ram_mb)}</span>
                <span className="tools-proc-cpu">{p.cpu_s.toFixed(1)}</span>
              </div>
            ))}
          </div>
        )}

        {processes.length > 10 && (
          <button
            className="tools-show-more-btn"
            onClick={() => setShowAll((s) => !s)}
          >
            {showAll ? "Show top 10" : `Show all ${processes.length}`}
          </button>
        )}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. SHADER CACHE CLEANER CARD
// ═══════════════════════════════════════════════════════════════════════════

function ShaderCacheCard() {
  const [caches, setCaches] = useState<ShaderCacheEntry[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(false);
  const [cleaning, setCleaning] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const data = await getShaderCaches();
      setCaches(data);
    } catch (e) {
      toast.error("Failed to scan shader caches", { description: String(e) });
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const toggleSelect = (id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const selectAll = () => {
    setSelected(new Set(caches.filter((c) => c.exists && c.size_mb > 0).map((c) => c.id)));
  };

  const clean = useCallback(async () => {
    if (selected.size === 0) {
      toast.warning("Select at least one cache to clean.");
      return;
    }
    setCleaning(true);
    try {
      const result = await cleanShaderCaches([...selected]);
      if (result.success) {
        toast.success(result.message, { description: "Caches cleaned" });
        setSelected(new Set());
        await refresh();
      } else {
        toast.warning(result.message);
      }
    } catch (e) {
      toast.error("Clean failed", { description: String(e) });
    } finally {
      setCleaning(false);
    }
  }, [selected, refresh]);

  const totalSelected = caches
    .filter((c) => selected.has(c.id))
    .reduce((sum, c) => sum + c.size_mb, 0);

  const vendorColor: Record<string, string> = {
    directx: "var(--info)",
    nvidia: "var(--success)",
    amd: "var(--warning)",
  };

  return (
    <div className="tools-card">
      <div className="tools-card-header">
        <div className="tools-card-title-row">
          <Layers size={16} className="tools-card-icon" style={{ color: "var(--accent)" }} />
          <div>
            <div className="tools-card-title">Shader Cache Cleaner</div>
            <div className="tools-card-desc">
              Clear GPU shader caches to free disk space and fix stutters
            </div>
          </div>
        </div>
        <button className="tools-icon-btn" onClick={refresh} disabled={loading} title="Refresh">
          <RefreshCw size={13} className={loading ? "spin" : ""} />
        </button>
      </div>

      <div className="tools-card-body">
        {caches.length === 0 && !loading && (
          <p className="tools-empty">No shader caches found.</p>
        )}

        {caches.map((cache) => {
          const isSel = selected.has(cache.id);
          const hasContent = cache.exists && cache.size_mb > 0.01;
          return (
            <div
              key={cache.id}
              className={`tools-cache-row ${isSel ? "tools-cache-row--selected" : ""} ${!cache.exists ? "tools-cache-row--absent" : ""}`}
              onClick={() => cache.exists && toggleSelect(cache.id)}
              style={{ cursor: cache.exists ? "pointer" : "default" }}
            >
              <input
                type="checkbox"
                className="tools-cache-check"
                checked={isSel}
                readOnly
                disabled={!cache.exists}
                tabIndex={-1}
              />
              <div className="tools-cache-info">
                <span className="tools-cache-name">{cache.name}</span>
                <span
                  className="tools-cache-vendor"
                  style={{ color: vendorColor[cache.vendor] ?? "var(--muted)" }}
                >
                  {cache.vendor}
                </span>
              </div>
              <span className="tools-cache-size" style={{ color: hasContent ? "var(--text)" : "var(--subtle)" }}>
                {cache.exists ? (hasContent ? fmtMB(cache.size_mb) : "Empty") : "Not found"}
              </span>
            </div>
          );
        })}

        <div className="tools-cache-footer">
          <button className="tools-link-btn" onClick={selectAll} disabled={cleaning}>
            Select all with content
          </button>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            {selected.size > 0 && (
              <span className="tools-cache-selected-size">
                ~{fmtMB(totalSelected)} selected
              </span>
            )}
            <button
              className="btn btn--danger btn--sm"
              onClick={clean}
              disabled={cleaning || selected.size === 0}
            >
              <Trash2 size={12} />
              {cleaning ? "Cleaning…" : `Clean (${selected.size})`}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. GAME SESSION MODE CARD
// ═══════════════════════════════════════════════════════════════════════════

function GameSessionCard({ isAdmin }: { isAdmin: boolean }) {
  const [status, setStatus] = useState<GameSessionStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchStatus = useCallback(async () => {
    try {
      const s = await getGameSessionStatus();
      setStatus(s);
    } catch {
      /* ignore */
    }
  }, []);

  useEffect(() => {
    fetchStatus();
    intervalRef.current = setInterval(fetchStatus, 5000);
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [fetchStatus]);

  // Live duration ticker
  const [tick, setTick] = useState(0);
  useEffect(() => {
    if (!status?.active) return;
    const t = setInterval(() => setTick((n) => n + 1), 1000);
    return () => clearInterval(t);
  }, [status?.active]);

  const liveDuration =
    status?.active && status.started_at_ms
      ? Math.floor((Date.now() - status.started_at_ms) / 1000)
      : 0;

  const toggle = useCallback(async () => {
    setBusy(true);
    try {
      const res = status?.active ? await endGameSession() : await startGameSession();
      if (res.success) {
        toast.success(res.message);
        await fetchStatus();
      } else {
        toast.error(res.message);
      }
    } catch (e) {
      toast.error("Failed", { description: String(e) });
    } finally {
      setBusy(false);
    }
  }, [status?.active, fetchStatus]);

  // suppress lint — tick is used to force re-render
  void tick;

  return (
    <div className="tools-card">
      <div className="tools-card-header">
        <div className="tools-card-title-row">
          <Gamepad2 size={16} className="tools-card-icon" style={{ color: "var(--success)" }} />
          <div>
            <div className="tools-card-title">Game Session Mode</div>
            <div className="tools-card-desc">
              Boost performance for gaming — restores settings when stopped
            </div>
          </div>
        </div>
        <div
          className={`tools-session-badge ${status?.active ? "tools-session-badge--on" : "tools-session-badge--off"}`}
        >
          {status?.active ? "ACTIVE" : "IDLE"}
        </div>
      </div>

      <div className="tools-card-body">
        {!isAdmin && (
          <div className="tools-warning-banner">
            <AlertTriangle size={13} />
            <span>Admin required for RAM flush and power plan changes</span>
          </div>
        )}

        {status?.active && (
          <div className="tools-session-stats">
            <div className="tools-session-stat">
              <span className="tools-session-stat-label">Started</span>
              <span className="tools-session-stat-value">
                {fmtTimestamp(status.started_at_ms)}
              </span>
            </div>
            <div className="tools-session-stat">
              <span className="tools-session-stat-label">Duration</span>
              <span className="tools-session-stat-value" style={{ color: "var(--success)" }}>
                {fmtDuration(liveDuration)}
              </span>
            </div>
          </div>
        )}

        {status?.active && status.actions_applied.length > 0 && (
          <div className="tools-session-actions">
            <p className="tools-section-label">Applied optimizations</p>
            {status.actions_applied.map((a) => (
              <div key={a} className="tools-session-action-item">
                <CheckCircle2 size={12} style={{ color: "var(--success)", flexShrink: 0 }} />
                <span>{a}</span>
              </div>
            ))}
          </div>
        )}

        {!status?.active && (
          <div className="tools-session-preview">
            <div className="tools-session-preview-item">
              <Circle size={8} style={{ color: "var(--accent)" }} />
              Switch to High Performance power plan
            </div>
            <div className="tools-session-preview-item">
              <Circle size={8} style={{ color: "var(--accent)" }} />
              Flush RAM standby list
            </div>
            <div className="tools-session-preview-item">
              <Circle size={8} style={{ color: "var(--accent)" }} />
              Flush DNS cache
            </div>
            <div className="tools-session-preview-item" style={{ color: "var(--subtle)", fontStyle: "italic" }}>
              Restores previous power plan on stop
            </div>
          </div>
        )}

        <button
          className={`btn ${status?.active ? "btn--danger" : "btn--accent"} btn--sm`}
          style={{ alignSelf: "flex-start" }}
          onClick={toggle}
          disabled={busy}
        >
          {status?.active ? (
            <>
              <Square size={12} /> Stop Session
            </>
          ) : (
            <>
              <Play size={12} /> Start Session
            </>
          )}
        </button>
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. MINECRAFT PROCESS MONITOR CARD
// ═══════════════════════════════════════════════════════════════════════════

function MinecraftMonitorCard() {
  const [data, setData] = useState<MinecraftMonitor | null>(null);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const d = await getMinecraftMonitor();
      setData(d);
    } catch (e) {
      toast.error("Monitor failed", { description: String(e) });
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
    const t = setInterval(refresh, 8000);
    return () => clearInterval(t);
  }, [refresh]);

  const ramPct =
    data?.found && data.ram_mb > 0
      ? Math.min(100, Math.round((data.ram_mb / 8192) * 100))
      : 0;

  return (
    <div className="tools-card">
      <div className="tools-card-header">
        <div className="tools-card-title-row">
          <span style={{ fontSize: 16, lineHeight: 1 }}>⛏️</span>
          <div>
            <div className="tools-card-title">Minecraft Monitor</div>
            <div className="tools-card-desc">
              Live javaw.exe process stats — auto-refreshes every 8 s
            </div>
          </div>
        </div>
        <button className="tools-icon-btn" onClick={refresh} disabled={loading} title="Refresh">
          <RefreshCw size={13} className={loading ? "spin" : ""} />
        </button>
      </div>

      <div className="tools-card-body">
        {data == null ? (
          <p className="tools-empty">Checking for Minecraft…</p>
        ) : !data.found ? (
          <div className="tools-status-off">
            <Circle size={10} style={{ color: "var(--subtle)" }} />
            <span>Minecraft is not running</span>
          </div>
        ) : (
          <>
            <div className="tools-mc-status">
              <CheckCircle2 size={14} style={{ color: "var(--success)" }} />
              <span style={{ color: "var(--success)", fontWeight: 600 }}>
                Running
              </span>
              {data.instance_count > 1 && (
                <span className="tools-mc-badge">{data.instance_count} instances</span>
              )}
            </div>

            <div className="tools-mc-title">{data.window_title}</div>

            <div className="tools-mc-stats">
              <div className="tools-mc-stat">
                <span className="tools-mc-stat-label">PID</span>
                <span className="tools-mc-stat-value">{data.pid ?? "—"}</span>
              </div>
              <div className="tools-mc-stat">
                <span className="tools-mc-stat-label">RAM</span>
                <span
                  className="tools-mc-stat-value"
                  style={{
                    color:
                      data.ram_mb > 6144
                        ? "var(--danger)"
                        : data.ram_mb > 3072
                        ? "var(--warning)"
                        : "var(--success)",
                  }}
                >
                  {fmtMB(data.ram_mb)}
                </span>
              </div>
              <div className="tools-mc-stat">
                <span className="tools-mc-stat-label">CPU time</span>
                <span className="tools-mc-stat-value">{data.cpu_s.toFixed(1)} s</span>
              </div>
            </div>

            {data.ram_mb > 0 && (
              <div className="tools-mc-bar-wrap">
                <div
                  className="tools-mc-bar-fill"
                  style={{
                    width: `${ramPct}%`,
                    background:
                      ramPct > 75
                        ? "var(--danger)"
                        : ramPct > 50
                        ? "var(--warning)"
                        : "var(--success)",
                  }}
                />
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. BENCHMARK SESSION CARD
// ═══════════════════════════════════════════════════════════════════════════

function BenchmarkSessionCard() {
  const [state, setState] = useState<BenchmarkStateResult>({ before: null, after: null });
  const [comparison, setComparison] = useState<BenchmarkComparison | null>(null);
  const [busyBefore, setBusyBefore] = useState(false);
  const [busyAfter, setBusyAfter] = useState(false);

  const loadState = useCallback(async () => {
    try {
      const s = await getBenchmarkState();
      setState(s);
      if (s.before && s.after) {
        const cmp = await getBenchmarkComparison();
        setComparison(cmp);
      } else {
        setComparison(null);
      }
    } catch {
      /* ignore */
    }
  }, []);

  useEffect(() => {
    loadState();
  }, [loadState]);

  const snapBefore = useCallback(async () => {
    setBusyBefore(true);
    try {
      await takeSnapshot("Before", "before");
      toast.success("Before snapshot taken");
      await loadState();
    } catch (e) {
      toast.error("Snapshot failed", { description: String(e) });
    } finally {
      setBusyBefore(false);
    }
  }, [loadState]);

  const snapAfter = useCallback(async () => {
    if (!state.before) {
      toast.warning("Take a Before snapshot first.");
      return;
    }
    setBusyAfter(true);
    try {
      await takeSnapshot("After", "after");
      toast.success("After snapshot taken");
      await loadState();
    } catch (e) {
      toast.error("Snapshot failed", { description: String(e) });
    } finally {
      setBusyAfter(false);
    }
  }, [state.before, loadState]);

  const reset = useCallback(async () => {
    await clearBenchmark();
    setState({ before: null, after: null });
    setComparison(null);
    toast.success("Benchmark session cleared");
  }, []);

  return (
    <div className="tools-card">
      <div className="tools-card-header">
        <div className="tools-card-title-row">
          <BarChart2 size={16} className="tools-card-icon" style={{ color: "var(--warning)" }} />
          <div>
            <div className="tools-card-title">Benchmark Session</div>
            <div className="tools-card-desc">
              Capture before/after system snapshots to measure your tweaks
            </div>
          </div>
        </div>
        {(state.before || state.after) && (
          <button
            className="tools-icon-btn"
            onClick={reset}
            title="Clear session"
            style={{ color: "var(--danger)" }}
          >
            <RotateCcw size={13} />
          </button>
        )}
      </div>

      <div className="tools-card-body">
        <div className="tools-bench-snapshots">
          <SnapshotPanel
            label="Before"
            snap={state.before}
            busy={busyBefore}
            onTake={snapBefore}
            accent="var(--info)"
          />
          <SnapshotPanel
            label="After"
            snap={state.after}
            busy={busyAfter}
            onTake={snapAfter}
            accent="var(--success)"
            disabled={!state.before}
          />
        </div>

        {comparison && (
          <div className="tools-bench-compare">
            <p className="tools-section-label">Comparison</p>
            <div className="tools-bench-delta-grid">
              <DeltaCell
                label="RAM usage"
                delta={comparison.ram_delta_mb}
                unit="MB"
                lowerIsBetter
              />
              <DeltaCell
                label="Processes"
                delta={comparison.process_delta}
                unit=""
                lowerIsBetter
              />
              <div className="tools-bench-delta-cell">
                <span className="tools-bench-delta-label">Duration</span>
                <span className="tools-bench-delta-value" style={{ color: "var(--text)" }}>
                  {fmtDuration(comparison.duration_secs)}
                </span>
              </div>
              <div className="tools-bench-delta-cell">
                <span className="tools-bench-delta-label">Power Plan</span>
                <span className="tools-bench-delta-value" style={{ color: "var(--muted)", fontSize: 11 }}>
                  {comparison.after.power_plan_name || "—"}
                </span>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function SnapshotPanel({
  label,
  snap,
  busy,
  onTake,
  accent,
  disabled = false,
}: {
  label: string;
  snap: SystemSnapshot | null;
  busy: boolean;
  onTake: () => void;
  accent: string;
  disabled?: boolean;
}) {
  return (
    <div className="tools-bench-panel">
      <div className="tools-bench-panel-header" style={{ borderColor: accent }}>
        <span className="tools-bench-panel-label" style={{ color: accent }}>
          {label}
        </span>
        <button
          className="btn btn--ghost btn--xs"
          onClick={onTake}
          disabled={busy || disabled}
        >
          <Camera size={11} />
          {busy ? "Capturing…" : snap ? "Retake" : "Capture"}
        </button>
      </div>
      {snap ? (
        <div className="tools-bench-snap-stats">
          <div className="tools-bench-snap-stat">
            <span className="tools-bench-snap-label">Time</span>
            <span className="tools-bench-snap-value">{fmtTimestamp(snap.timestamp_ms)}</span>
          </div>
          <div className="tools-bench-snap-stat">
            <span className="tools-bench-snap-label">RAM Used</span>
            <span className="tools-bench-snap-value">{fmtMB(snap.ram_used_mb)}</span>
          </div>
          <div className="tools-bench-snap-stat">
            <span className="tools-bench-snap-label">Processes</span>
            <span className="tools-bench-snap-value">{snap.process_count}</span>
          </div>
          <div className="tools-bench-snap-stat">
            <span className="tools-bench-snap-label">Uptime</span>
            <span className="tools-bench-snap-value">{fmtUptime(snap.uptime_secs)}</span>
          </div>
          <div className="tools-bench-snap-stat">
            <span className="tools-bench-snap-label">Power</span>
            <span
              className="tools-bench-snap-value"
              style={{ fontSize: 10, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
            >
              {snap.power_plan_name || "—"}
            </span>
          </div>
        </div>
      ) : (
        <div className="tools-bench-snap-empty">
          No snapshot yet
        </div>
      )}
    </div>
  );
}

function DeltaCell({
  label,
  delta,
  unit,
  lowerIsBetter,
}: {
  label: string;
  delta: number;
  unit: string;
  lowerIsBetter: boolean;
}) {
  const improved = lowerIsBetter ? delta < 0 : delta > 0;
  const neutral = delta === 0;
  const color = neutral
    ? "var(--muted)"
    : improved
    ? "var(--success)"
    : "var(--danger)";
  const prefix = delta > 0 ? "+" : "";

  return (
    <div className="tools-bench-delta-cell">
      <span className="tools-bench-delta-label">{label}</span>
      <span className="tools-bench-delta-value" style={{ color }}>
        {prefix}
        {delta.toFixed(0)}
        {unit && ` ${unit}`}
        {!neutral && <span style={{ fontSize: 10, marginLeft: 2 }}>{improved ? "↓" : "↑"}</span>}
      </span>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// PAGE ROOT
// ═══════════════════════════════════════════════════════════════════════════

export function ToolsPage({ isAdmin, onRestartAsAdmin }: ToolsPageProps) {
  return (
    <div className="page-wrapper">
      <div className="page-scroll">
        <div className="content-container">
          <div className="content-header">
            <span className="content-header-icon">🔧</span>
            <div>
              <h1 className="content-header-title">Gaming Tools</h1>
              <p className="content-header-count">
                6 optimization utilities
              </p>
            </div>
          </div>

          {!isAdmin && (
            <AdminBanner onRestartAsAdmin={onRestartAsAdmin} />
          )}

          <div className="tools-grid">
            <OverlayDetectorCard />
            <BackgroundLoadCard />
            <ShaderCacheCard />
            <GameSessionCard isAdmin={isAdmin} />
            <MinecraftMonitorCard />
            <BenchmarkSessionCard />
          </div>
        </div>
      </div>
    </div>
  );
}
