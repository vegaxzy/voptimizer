import { useState, useEffect, useCallback, useMemo } from "react";
import { toast } from "sonner";
import {
  Trash2,
  RefreshCw,
  Sparkles,
  AlertTriangle,
  ShieldAlert,
  ShieldCheck,
  Lock,
  Package,
  CheckCircle2,
  Loader2,
} from "lucide-react";
import {
  scanCleanup,
  cleanCleanup,
  listAppx,
  removeAppx,
  type CleanCategory,
  type AppxPackage,
} from "../invoke/debloat";
import { AdminBanner } from "../components/AdminBanner";
import { cn } from "../lib/cn";

interface DebloatPageProps {
  isAdmin: boolean;
  onRestartAsAdmin: () => void;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function fmtSize(mb: number): string {
  if (mb <= 0) return "Empty";
  if (mb >= 1024) return `${(mb / 1024).toFixed(2)} GB`;
  if (mb >= 10) return `${mb.toFixed(0)} MB`;
  return `${mb.toFixed(1)} MB`;
}
function fmtFiles(n: number): string {
  if (n === 0) return "no files";
  if (n === 1) return "1 file";
  return `${n.toLocaleString()} files`;
}

// ══════════════════════════════════════════════════════════════════════════════
//  Page shell — header + segmented view switcher
// ══════════════════════════════════════════════════════════════════════════════

export function DebloatPage({ isAdmin, onRestartAsAdmin }: DebloatPageProps) {
  const [view, setView] = useState<"clean" | "bloat">("clean");

  return (
    <div className="page-wrapper">
      <div className="page-scroll">
        <div className="content-container">
          <div className="sov-header">
            <div className="sov-header-left">
              <div className="sov-header-icon">
                <Trash2 size={20} strokeWidth={1.8} />
              </div>
              <div>
                <h1 className="content-header-title">Debloat</h1>
                <p className="content-header-count">
                  Free up space &amp; remove preinstalled apps
                </p>
              </div>
            </div>
          </div>

          <div className="dbl-seg">
            <button
              className={cn("dbl-seg-btn", view === "clean" && "dbl-seg-btn--active")}
              onClick={() => setView("clean")}
            >
              <Sparkles size={13} /> Temp &amp; Cache
            </button>
            <button
              className={cn("dbl-seg-btn", view === "bloat" && "dbl-seg-btn--active")}
              onClick={() => setView("bloat")}
            >
              <Package size={13} /> Bloatware
            </button>
          </div>

          {!isAdmin && <AdminBanner onRestartAsAdmin={onRestartAsAdmin} />}

          {view === "clean" ? (
            <TempCleanerView isAdmin={isAdmin} />
          ) : (
            <BloatwareView />
          )}
        </div>
      </div>
    </div>
  );
}

// ══════════════════════════════════════════════════════════════════════════════
//  View 1 — Temp & Cache cleaner
// ══════════════════════════════════════════════════════════════════════════════

function TempCleanerView({ isAdmin }: { isAdmin: boolean }) {
  const [cats, setCats] = useState<CleanCategory[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [scanning, setScanning] = useState(true);
  const [cleaning, setCleaning] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);

  const scan = useCallback(async () => {
    setScanning(true);
    try {
      const result = await scanCleanup();
      setCats(result);
      setSelected(
        new Set(
          result
            .filter((c) => c.default_selected && c.exists && c.size_mb > 0)
            .map((c) => c.id)
        )
      );
    } catch (e) {
      toast.error("Scan failed", { description: String(e) });
    } finally {
      setScanning(false);
    }
  }, []);

  useEffect(() => {
    scan();
  }, [scan]);

  const toggle = useCallback((id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      next.has(id) ? next.delete(id) : next.add(id);
      return next;
    });
  }, []);

  const totalReclaimable = useMemo(
    () => cats.reduce((s, c) => s + c.size_mb, 0),
    [cats]
  );
  const selectedCats = useMemo(
    () => cats.filter((c) => selected.has(c.id)),
    [cats, selected]
  );
  const selectedSize = selectedCats.reduce((s, c) => s + c.size_mb, 0);
  const hasIrreversible = selectedCats.some((c) => c.irreversible);

  const runClean = useCallback(async () => {
    setConfirmOpen(false);
    setCleaning(true);
    try {
      const result = await cleanCleanup([...selected]);
      result.success
        ? toast.success(result.message, { description: "Debloat complete" })
        : toast.error(result.message || "Nothing was cleaned");
      await scan();
    } catch (e) {
      toast.error("Clean failed", { description: String(e) });
    } finally {
      setCleaning(false);
    }
  }, [selected, scan]);

  return (
    <>
      <div className="dbl-toolbar">
        <span className="dbl-toolbar-info">
          {scanning ? "Scanning…" : `${fmtSize(totalReclaimable)} reclaimable`}
        </span>
        <button className="tools-icon-btn" onClick={scan} disabled={scanning || cleaning} title="Rescan">
          <RefreshCw size={13} className={scanning ? "spin" : ""} />
        </button>
      </div>

      <div className="dbl-list">
        {cats.map((c) => {
          const isSel = selected.has(c.id);
          const empty = !c.exists || c.size_mb <= 0;
          const adminBlocked = c.requires_admin && !isAdmin;
          return (
            <button
              key={c.id}
              className={cn("dbl-row", isSel && "dbl-row--selected", empty && "dbl-row--empty")}
              onClick={() => !empty && toggle(c.id)}
              disabled={empty || cleaning}
            >
              <span className={cn("dbl-check", isSel && "dbl-check--on")}>
                {isSel && <CheckCircle2 size={14} strokeWidth={2.4} />}
              </span>
              <span className="dbl-info">
                <span className="dbl-name-row">
                  <span className="dbl-name">{c.name}</span>
                  {c.irreversible && (
                    <span className="dbl-tag dbl-tag--danger">
                      <AlertTriangle size={10} /> Irreversible
                    </span>
                  )}
                  {c.requires_admin && (
                    <span className={cn("dbl-tag", adminBlocked ? "dbl-tag--warn" : "dbl-tag--muted")}>
                      {adminBlocked ? <ShieldAlert size={10} /> : <Lock size={10} />} Admin
                    </span>
                  )}
                </span>
                <span className="dbl-desc">{c.description}</span>
              </span>
              <span className="dbl-meta">
                <span className="dbl-size">{fmtSize(c.size_mb)}</span>
                <span className="dbl-count">{c.exists ? fmtFiles(c.file_count) : "not present"}</span>
              </span>
            </button>
          );
        })}
      </div>

      <div className="dbl-actionbar">
        <div className="dbl-actionbar-summary">
          <Sparkles size={14} style={{ color: "var(--accent)" }} />
          <span>
            {selected.size === 0
              ? "Select categories to clean"
              : `${selected.size} selected · ${fmtSize(selectedSize)} to free`}
          </span>
        </div>
        <button
          className="btn btn--accent dbl-clean-btn"
          onClick={() => setConfirmOpen(true)}
          disabled={selected.size === 0 || cleaning || scanning}
        >
          {cleaning ? (
            <><Loader2 size={13} className="spin" /> Cleaning…</>
          ) : (
            <><Trash2 size={13} /> Clean Selected</>
          )}
        </button>
      </div>

      {confirmOpen && (
        <div className="dbl-modal-overlay" onClick={() => setConfirmOpen(false)}>
          <div className="dbl-modal" onClick={(e) => e.stopPropagation()}>
            <div className="dbl-modal-header">
              <Trash2 size={16} style={{ color: "var(--accent)" }} />
              <span>Clean {selected.size} categor{selected.size === 1 ? "y" : "ies"}?</span>
            </div>
            <p className="dbl-modal-sub">
              Frees about <strong>{fmtSize(selectedSize)}</strong>. Files in use are skipped automatically.
            </p>
            <ul className="dbl-modal-list">
              {selectedCats.map((c) => (
                <li key={c.id}>
                  <span>{c.name}</span>
                  <span className="dbl-modal-list-size">{fmtSize(c.size_mb)}</span>
                </li>
              ))}
            </ul>
            {hasIrreversible && (
              <div className="dbl-modal-warn">
                <AlertTriangle size={13} />
                <span>A selected item is <strong>irreversible</strong> (e.g. the Recycle Bin) — those items can't be recovered.</span>
              </div>
            )}
            <div className="dbl-modal-actions">
              <button className="btn" onClick={() => setConfirmOpen(false)}>Cancel</button>
              <button className={cn("btn", hasIrreversible ? "btn--danger" : "btn--accent")} onClick={runClean}>
                <Trash2 size={13} /> Clean Now
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

// ══════════════════════════════════════════════════════════════════════════════
//  View 2 — Bloatware (UWP) remover
// ══════════════════════════════════════════════════════════════════════════════

function BloatwareView() {
  const [apps, setApps] = useState<AppxPackage[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);
  const [removing, setRemoving] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [showProtected, setShowProtected] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const result = await listAppx();
      setApps(result);
      setSelected(new Set());
    } catch (e) {
      toast.error("Failed to list apps", { description: String(e) });
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const toggle = useCallback((id: string, removable: boolean) => {
    if (!removable) return;
    setSelected((prev) => {
      const next = new Set(prev);
      next.has(id) ? next.delete(id) : next.add(id);
      return next;
    });
  }, []);

  const bloat = useMemo(() => apps.filter((a) => a.category === "bloat"), [apps]);
  const otherApps = useMemo(() => apps.filter((a) => a.category === "app"), [apps]);
  const protectedApps = useMemo(() => apps.filter((a) => a.category === "system"), [apps]);
  const selectedApps = useMemo(() => apps.filter((a) => selected.has(a.id)), [apps, selected]);

  const runRemove = useCallback(async () => {
    setConfirmOpen(false);
    setRemoving(true);
    try {
      const result = await removeAppx([...selected]);
      result.success
        ? toast.success(result.message, { description: "Bloatware removed" })
        : toast.error(result.message || "Nothing was removed");
      await load();
    } catch (e) {
      toast.error("Removal failed", { description: String(e) });
    } finally {
      setRemoving(false);
    }
  }, [selected, load]);

  const renderRow = (a: AppxPackage) => {
    const isSel = selected.has(a.id);
    return (
      <button
        key={a.id}
        className={cn("dbl-row", isSel && "dbl-row--selected", !a.removable && "dbl-row--empty")}
        onClick={() => toggle(a.id, a.removable)}
        disabled={!a.removable || removing}
      >
        <span className={cn("dbl-check", isSel && "dbl-check--on")}>
          {isSel && <CheckCircle2 size={14} strokeWidth={2.4} />}
        </span>
        <span className="dbl-info">
          <span className="dbl-name-row">
            <span className="dbl-name">{a.name}</span>
            <span className="dbl-pub">· {a.publisher}</span>
            {a.recommended && (
              <span className="dbl-tag dbl-tag--good"><Sparkles size={10} /> Recommended</span>
            )}
            {!a.removable && (
              <span className="dbl-tag dbl-tag--ok"><ShieldCheck size={10} /> Protected</span>
            )}
          </span>
          <span className="dbl-desc">{a.note}</span>
        </span>
      </button>
    );
  };

  if (loading) {
    return (
      <div className="dbl-toolbar" style={{ justifyContent: "center", padding: 40 }}>
        <Loader2 size={16} className="spin" />
        <span className="dbl-toolbar-info">Listing installed apps…</span>
      </div>
    );
  }

  return (
    <>
      <div className="dbl-toolbar">
        <span className="dbl-toolbar-info">
          {bloat.length} recommended · {otherApps.length} other apps
        </span>
        <button className="tools-icon-btn" onClick={load} disabled={removing} title="Refresh">
          <RefreshCw size={13} />
        </button>
      </div>

      {bloat.length > 0 && <div className="dbl-group-label">Recommended to remove</div>}
      <div className="dbl-list">{bloat.map(renderRow)}</div>

      {otherApps.length > 0 && <div className="dbl-group-label">Other installed apps</div>}
      <div className="dbl-list">{otherApps.map(renderRow)}</div>

      {protectedApps.length > 0 && (
        <>
          <button
            className="dbl-group-label"
            style={{ background: "none", border: "none", cursor: "pointer", display: "block" }}
            onClick={() => setShowProtected((v) => !v)}
          >
            {showProtected ? "▾" : "▸"} Protected — system components ({protectedApps.length})
          </button>
          {showProtected && <div className="dbl-list">{protectedApps.map(renderRow)}</div>}
        </>
      )}

      <div className="dbl-actionbar">
        <div className="dbl-actionbar-summary">
          <Package size={14} style={{ color: "var(--accent)" }} />
          <span>
            {selected.size === 0
              ? "Select apps to remove"
              : `${selected.size} app${selected.size === 1 ? "" : "s"} selected`}
          </span>
        </div>
        <button
          className="btn btn--accent dbl-clean-btn"
          onClick={() => setConfirmOpen(true)}
          disabled={selected.size === 0 || removing}
        >
          {removing ? (
            <><Loader2 size={13} className="spin" /> Removing…</>
          ) : (
            <><Trash2 size={13} /> Remove Selected</>
          )}
        </button>
      </div>

      {confirmOpen && (
        <div className="dbl-modal-overlay" onClick={() => setConfirmOpen(false)}>
          <div className="dbl-modal" onClick={(e) => e.stopPropagation()}>
            <div className="dbl-modal-header">
              <Package size={16} style={{ color: "var(--accent)" }} />
              <span>Remove {selected.size} app{selected.size === 1 ? "" : "s"}?</span>
            </div>
            <p className="dbl-modal-sub">
              These are removed for your account only and can be <strong>reinstalled from the Microsoft Store</strong> anytime.
            </p>
            <ul className="dbl-modal-list">
              {selectedApps.map((a) => (
                <li key={a.id}>
                  <span>{a.name}</span>
                  <span className="dbl-modal-list-size">{a.publisher}</span>
                </li>
              ))}
            </ul>
            <div className="dbl-modal-actions">
              <button className="btn" onClick={() => setConfirmOpen(false)}>Cancel</button>
              <button className="btn btn--accent" onClick={runRemove}>
                <Trash2 size={13} /> Remove Now
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
