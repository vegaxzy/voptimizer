import { useState, useMemo } from "react";
import {
  createColumnHelper,
  flexRender,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
  type SortingState,
} from "@tanstack/react-table";
import {
  Archive,
  RefreshCw,
  ChevronUp,
  ChevronDown,
  ChevronsUpDown,
  RotateCcw,
  Trash2,
  CheckCircle2,
  XCircle,
  HardDrive,
  History,
} from "lucide-react";
import { cn } from "../lib/cn";
import type { BackupEntry, HistoryEntry } from "../types/backup";
import { ACTION_LABELS, CATEGORY_LABELS, PRESET_REGISTRY_KEYS } from "../types/backup";
import { BottomLogDrawer } from "../components/BottomLogDrawer";
import { AdminBanner } from "../components/AdminBanner";
import { useBackup } from "../hooks/useBackup";

// ── Helpers ───────────────────────────────────────────────────────────────────

function fmtDate(ms: number): string {
  return new Date(ms).toLocaleString([], {
    month: "short", day: "numeric",
    hour: "2-digit", minute: "2-digit",
  });
}

function fmtBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(2)} MB`;
}

function SortIcon({ direction }: { direction: "asc" | "desc" | false }) {
  if (direction === "asc")  return <ChevronUp size={11} strokeWidth={2} />;
  if (direction === "desc") return <ChevronDown size={11} strokeWidth={2} />;
  return <ChevronsUpDown size={11} strokeWidth={1.8} style={{ opacity: 0.35 }} />;
}

// ── Section wrapper ───────────────────────────────────────────────────────────

function Section({ title, icon, children }: { title: string; icon: React.ReactNode; children: React.ReactNode }) {
  return (
    <section className="br-section">
      <h2 className="br-section-title">
        {icon} {title}
      </h2>
      {children}
    </section>
  );
}

// ── Restore Point card ────────────────────────────────────────────────────────

interface RestorePointCardProps {
  status: { enabled: boolean; message: string } | null;
  isBusy: boolean;
  onCreate: (desc: string) => void;
}

function RestorePointCard({ status, isBusy, onCreate }: RestorePointCardProps) {
  const [desc, setDesc] = useState("VOptimizer pre-change checkpoint");

  return (
    <div className="br-card">
      {status ? (
        <div className={cn(
          "restore-status-chip",
          status.enabled ? "restore-status-chip--ok" : "restore-status-chip--warn"
        )}>
          {status.enabled
            ? <><CheckCircle2 size={11} strokeWidth={2} /> System Restore detected</>
            : <><XCircle size={11} strokeWidth={2} /> System Restore unavailable</>
          }
        </div>
      ) : (
        <div className="restore-status-chip restore-status-chip--loading">Checking…</div>
      )}

      {status && !status.enabled && (
        <p className="br-warning-text">{status.message}</p>
      )}
      {status?.enabled && (
        <p className="br-hint-text">{status.message}</p>
      )}

      <div className="br-inline-form">
        <input
          className="br-input"
          value={desc}
          onChange={(e) => setDesc(e.currentTarget.value)}
          placeholder="Restore point description"
          maxLength={120}
        />
        <button
          className="btn btn--apply btn--sm"
          disabled={isBusy || !desc.trim() || !status?.enabled}
          onClick={() => onCreate(desc.trim())}
          title={!status?.enabled ? "System Restore must be enabled first" : undefined}
        >
          {isBusy ? "Creating…" : "Create Restore Point"}
        </button>
      </div>
    </div>
  );
}

// ── Create Backup form ────────────────────────────────────────────────────────

interface CreateBackupFormProps {
  isBusy: boolean;
  onCreate: (label: string, key: string) => void;
}

function CreateBackupForm({ isBusy, onCreate }: CreateBackupFormProps) {
  const [label, setLabel] = useState("");
  const [keyChoice, setKeyChoice] = useState(PRESET_REGISTRY_KEYS[0].key);
  const [customKey, setCustomKey] = useState("");
  const useCustom = keyChoice === "__custom__";
  const resolvedKey = useCustom ? customKey.trim() : keyChoice;
  const canSubmit = label.trim().length > 0 && resolvedKey.length > 0 && !isBusy;

  return (
    <div className="br-card">
      <p className="br-card-label">Create New Backup</p>
      <div className="br-form-grid">
        <label className="br-form-label">Label</label>
        <input
          className="br-input"
          value={label}
          onChange={(e) => setLabel(e.currentTarget.value)}
          placeholder="e.g. Before startup tweak"
          maxLength={80}
        />
        <label className="br-form-label">Registry Key</label>
        <select
          className="br-select"
          value={keyChoice}
          onChange={(e) => setKeyChoice(e.currentTarget.value)}
        >
          {PRESET_REGISTRY_KEYS.map((p) => (
            <option key={p.key} value={p.key}>{p.label}</option>
          ))}
          <option value="__custom__">Custom key…</option>
        </select>
        {useCustom && (
          <>
            <label className="br-form-label">Custom Key</label>
            <input
              className="br-input br-input--mono"
              value={customKey}
              onChange={(e) => setCustomKey(e.currentTarget.value)}
              placeholder="HKCU\Software\..."
            />
          </>
        )}
      </div>
      <button
        className="btn btn--apply btn--sm"
        disabled={!canSubmit}
        onClick={() => { onCreate(label.trim(), resolvedKey); setLabel(""); }}
      >
        {isBusy ? "Creating…" : "Create Backup"}
      </button>
    </div>
  );
}

// ── Backup list (react-table) ─────────────────────────────────────────────────

const backupHelper = createColumnHelper<BackupEntry>();

interface BackupListProps {
  backups: BackupEntry[];
  busyIds: Set<string>;
  onRestore: (id: string) => void;
  onDelete: (id: string) => void;
}

function BackupList({ backups, busyIds, onRestore, onDelete }: BackupListProps) {
  const [confirmId, setConfirmId] = useState<string | null>(null);
  const [sorting, setSorting] = useState<SortingState>([{ id: "timestamp", desc: true }]);

  const columns = useMemo(
    () => [
      backupHelper.accessor("label", { header: "Label" }),
      backupHelper.accessor("registry_key", {
        header: "Registry Key",
        enableSorting: false,
        cell: (info) => (
          <span className="br-key-text" title={info.getValue()}>{info.getValue()}</span>
        ),
      }),
      backupHelper.accessor("timestamp", {
        header: "Date",
        cell: (info) => fmtDate(info.getValue()),
      }),
      backupHelper.accessor("size_bytes", {
        header: "Size",
        cell: (info) => fmtBytes(info.getValue()),
      }),
      backupHelper.display({
        id: "actions",
        header: "Actions",
        cell: (info) => {
          const b = info.row.original;
          const busy = busyIds.has(b.id);
          const isConfirming = confirmId === b.id;
          return isConfirming ? (
            <span className="confirm-inline">
              <span className="confirm-label">Delete?</span>
              <button
                className="btn btn--danger btn--sm"
                onClick={() => { setConfirmId(null); onDelete(b.id); }}
              >Yes</button>
              <button className="btn btn--ghost btn--sm" onClick={() => setConfirmId(null)}>No</button>
            </span>
          ) : (
            <span className="br-action-row">
              <button
                className="btn btn--enable btn--sm"
                disabled={busy}
                onClick={() => onRestore(b.id)}
                title="Restore this registry backup"
              >
                <RotateCcw size={10} strokeWidth={2.5} />
                {busy ? "…" : "Restore"}
              </button>
              <button
                className="btn btn--ghost btn--sm"
                disabled={busy}
                onClick={() => setConfirmId(b.id)}
              >
                <Trash2 size={10} strokeWidth={2.5} />
                Delete
              </button>
            </span>
          );
        },
      }),
    ],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [busyIds, confirmId]
  );

  const table = useReactTable({
    data: backups,
    columns,
    state: { sorting },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  if (backups.length === 0) {
    return <p className="br-empty">No backups yet. Create one before applying changes.</p>;
  }

  return (
    <div className="br-history-wrap">
      <table className="br-table">
        <thead>
          <tr className="br-thead-row">
            {table.getFlatHeaders().map((header) => {
              const canSort = header.column.getCanSort();
              return (
                <th
                  key={header.id}
                  className={cn("br-th", canSort && "startup-th--sortable")}
                  onClick={canSort ? header.column.getToggleSortingHandler() : undefined}
                >
                  <span className="startup-th-inner">
                    {flexRender(header.column.columnDef.header, header.getContext())}
                    {canSort && <SortIcon direction={header.column.getIsSorted()} />}
                  </span>
                </th>
              );
            })}
          </tr>
        </thead>
        <tbody>
          {table.getRowModel().rows.map((row) => (
            <tr key={row.id} className="br-row">
              {row.getVisibleCells().map((cell) => (
                <td key={cell.id} className={cn("br-td", `br-td--${cell.column.id}`)}>
                  {flexRender(cell.column.columnDef.cell, cell.getContext())}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

// ── History table (react-table) ───────────────────────────────────────────────

const historyHelper = createColumnHelper<HistoryEntry>();

interface HistoryTableProps {
  entries: HistoryEntry[];
  onClear: () => void;
}

function HistoryTable({ entries, onClear }: HistoryTableProps) {
  const [categoryFilter, setCategoryFilter] = useState<string>("all");
  const [sorting, setSorting] = useState<SortingState>([{ id: "timestamp", desc: true }]);

  const filteredData = useMemo(
    () => categoryFilter === "all" ? entries : entries.filter((e) => e.category === categoryFilter),
    [entries, categoryFilter]
  );

  const columns = useMemo(
    () => [
      historyHelper.accessor("timestamp", {
        header: "Time",
        cell: (info) => fmtDate(info.getValue()),
      }),
      historyHelper.accessor("action", {
        header: "Action",
        cell: (info) => ACTION_LABELS[info.getValue()] ?? info.getValue(),
      }),
      historyHelper.accessor("category", {
        header: "Category",
        enableSorting: false,
        cell: (info) => (
          <span className={cn("br-cat-badge", `br-cat-badge--${info.getValue()}`)}>
            {CATEGORY_LABELS[info.getValue()] ?? info.getValue()}
          </span>
        ),
      }),
      historyHelper.accessor("target", {
        header: "Target",
        enableSorting: false,
        cell: (info) => (
          <span className="br-td--target" title={info.getValue()}>{info.getValue()}</span>
        ),
      }),
      historyHelper.accessor("success", {
        header: "Result",
        cell: (info) => info.getValue()
          ? <span className="status-chip status-chip--enabled"><CheckCircle2 size={10} strokeWidth={2} /> OK</span>
          : <span className="status-chip status-chip--disabled"><XCircle size={10} strokeWidth={2} /> Failed</span>,
      }),
    ],
    []
  );

  const table = useReactTable({
    data: filteredData,
    columns,
    state: { sorting },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  return (
    <div className="br-history-wrap">
      <div className="br-history-toolbar">
        <div className="br-filter-chips">
          {["all", "startup", "tweak", "backup"].map((f) => (
            <button
              key={f}
              className={cn("br-filter-chip", categoryFilter === f && "br-filter-chip--active")}
              onClick={() => setCategoryFilter(f)}
            >
              {f === "all" ? "All" : CATEGORY_LABELS[f] ?? f}
            </button>
          ))}
        </div>
        {entries.length > 0 && (
          <button className="btn btn--ghost btn--sm" onClick={onClear}>
            <Trash2 size={11} strokeWidth={2} />
            Clear All
          </button>
        )}
      </div>

      {table.getRowModel().rows.length === 0 ? (
        <p className="br-empty">No history entries.</p>
      ) : (
        <table className="br-table br-history-table">
          <thead>
            <tr className="br-thead-row">
              {table.getFlatHeaders().map((header) => {
                const canSort = header.column.getCanSort();
                return (
                  <th
                    key={header.id}
                    className={cn("br-th", canSort && "startup-th--sortable")}
                    onClick={canSort ? header.column.getToggleSortingHandler() : undefined}
                  >
                    <span className="startup-th-inner">
                      {flexRender(header.column.columnDef.header, header.getContext())}
                      {canSort && <SortIcon direction={header.column.getIsSorted()} />}
                    </span>
                  </th>
                );
              })}
            </tr>
          </thead>
          <tbody>
            {table.getRowModel().rows.map((row) => (
              <tr
                key={row.id}
                className={cn("br-row", !row.original.success && "br-row--failed")}
                title={row.original.message}
              >
                {row.getVisibleCells().map((cell) => (
                  <td key={cell.id} className="br-td">
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

// ── Page ──────────────────────────────────────────────────────────────────────

interface BackupRestorePageProps {
  isAdmin: boolean;
  onRestartAsAdmin: () => void;
}

export function BackupRestorePage({ isAdmin, onRestartAsAdmin }: BackupRestorePageProps) {
  const {
    backups, history, restoreStatus, logs,
    isLoading, busyIds, refreshAll,
    createBackup, restoreBackup, deleteBackup,
    createRestorePoint, doClearHistory, clearLogs,
  } = useBackup();

  const rpBusy     = [...busyIds].some((id) => id.startsWith("rp-"));
  const createBusy = [...busyIds].some((id) => id.startsWith("create-"));

  return (
    <div className="br-page">
      <div className="page-scroll">
        <div className="content-container">
          {/* ── Admin banner ─────────────────────────────────────── */}
          {!isAdmin && <AdminBanner onRestartAsAdmin={onRestartAsAdmin} />}

          {/* ── Header ───────────────────────────────────────────── */}
          <header className="startup-header">
            <div className="startup-header-left">
              <Archive size={14} strokeWidth={1.8} style={{ color: "var(--accent)", flexShrink: 0 }} />
              <h1 className="content-header-title">Backup & Restore</h1>
              <span className="content-header-count">{backups.length} backups</span>
              <span className="startup-stat startup-stat--enabled">{history.length} history entries</span>
            </div>
            <div className="startup-header-right">
              <button
                className="btn btn--ghost btn--sm"
                onClick={refreshAll}
                disabled={isLoading}
                title="Refresh"
              >
                <RefreshCw size={11} strokeWidth={2} className={isLoading ? "spin" : ""} />
                {isLoading ? "Loading…" : "Refresh"}
              </button>
            </div>
          </header>

          {/* ── Body ─────────────────────────────────────────────── */}
          <div className="br-top-row">
            <Section title="System Restore Point" icon={<RotateCcw size={13} strokeWidth={1.8} />}>
              <RestorePointCard status={restoreStatus} isBusy={rpBusy} onCreate={createRestorePoint} />
            </Section>
            <Section title="Create Registry Backup" icon={<HardDrive size={13} strokeWidth={1.8} />}>
              <CreateBackupForm isBusy={createBusy} onCreate={createBackup} />
            </Section>
          </div>

          <Section title="Saved Backups" icon={<Archive size={13} strokeWidth={1.8} />}>
            <BackupList
              backups={backups}
              busyIds={busyIds}
              onRestore={restoreBackup}
              onDelete={deleteBackup}
            />
          </Section>

          <Section title="Action History" icon={<History size={13} strokeWidth={1.8} />}>
            <HistoryTable entries={history} onClear={doClearHistory} />
          </Section>
        </div>
      </div>

      {/* ── Log drawer ─────────────────────────────────────────── */}
      <BottomLogDrawer logs={logs} onClear={clearLogs} />
    </div>
  );
}
