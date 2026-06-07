import { useState, useMemo } from "react";
import {
  createColumnHelper,
  flexRender,
  getCoreRowModel,
  getSortedRowModel,
  getFilteredRowModel,
  useReactTable,
  type SortingState,
  type ColumnFiltersState,
} from "@tanstack/react-table";
import {
  Rocket,
  RefreshCw,
  Search,
  ChevronUp,
  ChevronDown,
  ChevronsUpDown,
  TriangleAlert,
  LockKeyhole,
  Zap,
  EyeOff,
  ShieldCheck,
} from "lucide-react";
import { cn } from "../lib/cn";
import type { StartupApp } from "../types/startup";
import { ELEVATED_SOURCES } from "../types/startup";
import { BottomLogDrawer } from "../components/BottomLogDrawer";
import { AdminBanner } from "../components/AdminBanner";
import { useStartupApps } from "../hooks/useStartupApps";

// ── Column definitions ────────────────────────────────────────────────────────

function fmtClock(ms: number) {
  return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

const helper = createColumnHelper<StartupApp>();

const SOURCE_CLASSES: Record<string, string> = {
  hkcu_run:       "source-badge--hkcu",
  hklm_run:       "source-badge--hklm",
  user_startup:   "source-badge--user",
  common_startup: "source-badge--common",
};

// ── Sort icon helper ──────────────────────────────────────────────────────────

function SortIcon({ direction }: { direction: "asc" | "desc" | false }) {
  if (direction === "asc")  return <ChevronUp size={11} strokeWidth={2} />;
  if (direction === "desc") return <ChevronDown size={11} strokeWidth={2} />;
  return <ChevronsUpDown size={11} strokeWidth={1.8} style={{ opacity: 0.35 }} />;
}

// ── Inline confirm ────────────────────────────────────────────────────────────

interface ActionCellProps {
  app: StartupApp;
  isBusy: boolean;
  isPendingConfirm: boolean;
  onDisableClick: (app: StartupApp) => void;
  onEnable: (id: string) => void;
  onConfirm: (id: string) => void;
  onCancelConfirm: () => void;
}

function ActionCell({
  app, isBusy, isPendingConfirm,
  onDisableClick, onEnable, onConfirm, onCancelConfirm,
}: ActionCellProps) {
  if (isPendingConfirm) {
    return (
      <span className="confirm-inline">
        <span className="confirm-label">Disable?</span>
        <button className="btn btn--danger btn--sm" onClick={() => onConfirm(app.id)}>Yes</button>
        <button className="btn btn--ghost btn--sm" onClick={onCancelConfirm}>No</button>
      </span>
    );
  }
  if (app.status === "enabled") {
    return (
      <button
        className="btn btn--disable btn--sm"
        disabled={isBusy}
        onClick={() => onDisableClick(app)}
      >
        {isBusy ? "…" : "Disable"}
      </button>
    );
  }
  return (
    <button
      className="btn btn--enable btn--sm"
      disabled={isBusy}
      onClick={() => onEnable(app.id)}
    >
      {isBusy ? "…" : "Enable"}
    </button>
  );
}

// ── Page ──────────────────────────────────────────────────────────────────────

interface StartupAppsPageProps {
  isAdmin: boolean;
  onRestartAsAdmin: () => void;
}

export function StartupAppsPage({ isAdmin, onRestartAsAdmin }: StartupAppsPageProps) {
  const { apps, logs, isLoading, isRefreshing, lastUpdated, error, busyIds, refresh, disable, enable, clearLogs } =
    useStartupApps();

  const [search, setSearch] = useState("");
  const [pendingConfirmId, setPendingConfirmId] = useState<string | null>(null);
  const [sorting, setSorting] = useState<SortingState>([]);
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);

  const handleDisableClick = (app: StartupApp) => {
    const needsConfirm = app.is_sensitive || ELEVATED_SOURCES.has(app.source as any);
    if (needsConfirm) setPendingConfirmId(app.id);
    else disable(app.id);
  };

  const handleConfirm = (id: string) => {
    setPendingConfirmId(null);
    disable(id);
  };

  // Client-side global search filter on top of react-table
  const filtered = useMemo(
    () =>
      search.trim()
        ? apps.filter(
            (a) =>
              a.name.toLowerCase().includes(search.toLowerCase()) ||
              a.command.toLowerCase().includes(search.toLowerCase())
          )
        : apps,
    [apps, search]
  );

  const columns = useMemo(
    () => [
      helper.accessor("name", {
        header: "Name",
        cell: (info) => {
          const app = info.row.original;
          return (
            <span className="startup-name-cell">
              {app.is_sensitive && (
                <span title="Contains keywords associated with system/security software — double-check before disabling">
                  <TriangleAlert size={11} strokeWidth={2} className="sensitive-icon" />
                </span>
              )}
              <span className="startup-name-text">{info.getValue()}</span>
            </span>
          );
        },
      }),
      helper.accessor("source_display", {
        header: "Source",
        cell: (info) => {
          const app = info.row.original;
          return (
            <span className="startup-source-cell">
              <span className={cn("source-badge", SOURCE_CLASSES[app.source] ?? "")}>
                {info.getValue()}
              </span>
              {(app.source === "hklm_run" || app.source === "common_startup") && (
                <span title="Modifying this entry requires administrator privileges">
                  <LockKeyhole size={10} strokeWidth={2} className="admin-hint" />
                </span>
              )}
            </span>
          );
        },
      }),
      helper.accessor("command", {
        header: "Command / Path",
        enableSorting: false,
        cell: (info) => (
          <span className="startup-command-text" title={info.getValue()}>
            {info.getValue()}
          </span>
        ),
      }),
      helper.accessor("status", {
        header: "Status",
        cell: (info) => (
          <span className={cn("status-chip", `status-chip--${info.getValue()}`)}>
            {info.getValue() === "enabled" ? "Enabled" : "Disabled"}
          </span>
        ),
      }),
      helper.display({
        id: "actions",
        header: "Action",
        cell: (info) => {
          const app = info.row.original;
          return (
            <ActionCell
              app={app}
              isBusy={busyIds.has(app.id)}
              isPendingConfirm={pendingConfirmId === app.id}
              onDisableClick={handleDisableClick}
              onEnable={enable}
              onConfirm={handleConfirm}
              onCancelConfirm={() => setPendingConfirmId(null)}
            />
          );
        },
      }),
    ],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [busyIds, pendingConfirmId, enable]
  );

  const table = useReactTable({
    data: filtered,
    columns,
    state: { sorting, columnFilters },
    onSortingChange: setSorting,
    onColumnFiltersChange: setColumnFilters,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
  });

  const enabledCount   = apps.filter((a) => a.status === "enabled").length;
  const disabledCount  = apps.filter((a) => a.status === "disabled").length;
  const protectedCount = apps.filter((a) => a.is_sensitive).length;

  return (
    <div className="startup-page">
      <div className="page-scroll">
      <div className="content-container">
        {/* ── Admin banner ─────────────────────────────────────────── */}
        {!isAdmin && <AdminBanner onRestartAsAdmin={onRestartAsAdmin} />}

        {/* ── Header ─────────────────────────────────────────────── */}
        <header className="startup-header">
          <div className="startup-header-left">
            <Rocket size={14} strokeWidth={1.8} style={{ color: "var(--accent)", flexShrink: 0 }} />
            <h1 className="content-header-title">Startup Apps</h1>
            <span className="content-header-count">{apps.length} entries</span>
          </div>
          <div className="startup-header-right">
            <span className="startup-search-wrap">
              <Search size={12} strokeWidth={2} className="startup-search-icon" />
              <input
                className="startup-search"
                type="search"
                placeholder="Search by name or path…"
                value={search}
                onChange={(e) => setSearch(e.currentTarget.value)}
              />
            </span>
            <button
              className="btn btn--ghost btn--sm"
              onClick={refresh}
              disabled={isLoading || isRefreshing}
              title="Refresh list"
            >
              <RefreshCw size={11} strokeWidth={2} className={isLoading || isRefreshing ? "spin" : ""} />
              {isLoading || isRefreshing ? "Refreshing…" : "Refresh"}
            </button>
          </div>
        </header>

        {lastUpdated && (
          <div className="startup-meta-row">
            <span className="refresh-indicator">Updated {fmtClock(lastUpdated)}</span>
            {error && (
              <span className="startup-inline-error">
                <TriangleAlert size={11} strokeWidth={2} /> Refresh failed — showing last known data
              </span>
            )}
          </div>
        )}

        {/* ── Summary stat cards ──────────────────────────────────── */}
        <div className="stat-grid">
          <div className="stat-card">
            <div className="stat-card-icon stat-card-icon--enabled">
              <Zap size={15} strokeWidth={2} />
            </div>
            <div className="stat-card-body">
              <span className="stat-card-value">{enabledCount}</span>
              <span className="stat-card-label">Enabled</span>
            </div>
          </div>
          <div className="stat-card">
            <div className="stat-card-icon stat-card-icon--disabled">
              <EyeOff size={15} strokeWidth={2} />
            </div>
            <div className="stat-card-body">
              <span className="stat-card-value">{disabledCount}</span>
              <span className="stat-card-label">Disabled</span>
            </div>
          </div>
          <div className="stat-card">
            <div className="stat-card-icon stat-card-icon--protected">
              <ShieldCheck size={15} strokeWidth={2} />
            </div>
            <div className="stat-card-body">
              <span className="stat-card-value">{protectedCount}</span>
              <span className="stat-card-label">Sensitive</span>
            </div>
          </div>
        </div>

        {/* ── Table ──────────────────────────────────────────────── */}
        <div className="startup-table-wrap">
          {isLoading && apps.length === 0 ? (
            <div className="startup-table-inner">
              <div className="startup-skeleton-list">
                {Array.from({ length: 8 }).map((_, i) => (
                  <div key={i} className="startup-skeleton-row">
                    <div className="skeleton" style={{ width: "32%", height: 13 }} />
                    <div className="skeleton" style={{ width: "18%", height: 13 }} />
                    <div className="skeleton" style={{ width: "26%", height: 13 }} />
                    <div className="skeleton" style={{ width: 64, height: 20, borderRadius: 20 }} />
                  </div>
                ))}
              </div>
            </div>
          ) : error && apps.length === 0 ? (
            <div className="startup-placeholder">
              <TriangleAlert size={20} strokeWidth={1.8} style={{ color: "var(--subtle)", marginBottom: 10 }} />
              <p style={{ marginBottom: 12 }}>Couldn&apos;t read startup entries.</p>
              <button className="btn btn--ghost btn--sm" onClick={refresh}>
                <RefreshCw size={11} strokeWidth={2} /> Try again
              </button>
            </div>
          ) : table.getRowModel().rows.length === 0 ? (
            <div className="startup-placeholder">
              {search ? `No results for "${search}"` : "No startup entries found."}
            </div>
          ) : (
            <div className="startup-table-inner content-fade-in">
              <table className="startup-table">
                <thead>
                  <tr className="startup-thead-row">
                    {table.getFlatHeaders().map((header) => {
                      const canSort = header.column.getCanSort();
                      return (
                        <th
                          key={header.id}
                          className={cn(
                            "startup-th",
                            `startup-th--${header.id}`,
                            canSort && "startup-th--sortable"
                          )}
                          onClick={canSort ? header.column.getToggleSortingHandler() : undefined}
                        >
                          <span className="startup-th-inner">
                            {flexRender(header.column.columnDef.header, header.getContext())}
                            {canSort && (
                              <SortIcon direction={header.column.getIsSorted()} />
                            )}
                          </span>
                        </th>
                      );
                    })}
                  </tr>
                </thead>
                <tbody>
                  {table.getRowModel().rows.map((row) => {
                    const app = row.original;
                    return (
                      <tr
                        key={row.id}
                        className={cn(
                          "startup-row",
                          app.status === "disabled" && "startup-row--disabled",
                          app.is_sensitive && "startup-row--sensitive"
                        )}
                      >
                        {row.getVisibleCells().map((cell) => (
                          <td
                            key={cell.id}
                            className={cn("startup-td", `startup-td--${cell.column.id}`)}
                          >
                            {flexRender(cell.column.columnDef.cell, cell.getContext())}
                          </td>
                        ))}
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </div>
      </div>

      {/* ── Log drawer ─────────────────────────────────────────── */}
      <BottomLogDrawer logs={logs} onClear={clearLogs} />
    </div>
  );
}
