import { useState, useMemo } from "react";
import type { TweakCategory, TweakState, LogEntry, UnifiedTweak } from "../types";
import { ALL_TWEAKS } from "../data/tweaks";
import { CompactTweakRow } from "./CompactTweakRow";
import { BottomLogDrawer } from "./BottomLogDrawer";
import { AdminBanner } from "./AdminBanner";
import { RiskModal } from "./RiskModal";
import { categoryIcon } from "../lib/categoryIcons";
import { cn } from "../lib/cn";

type FilterMode = "all" | "implemented" | "applied" | "safe" | "high-risk" | "experimental";

const FILTER_LABELS: { id: FilterMode; label: string }[] = [
  { id: "all",          label: "All" },
  { id: "implemented",  label: "Ready" },
  { id: "applied",      label: "Applied" },
  { id: "safe",         label: "Safe" },
  { id: "high-risk",    label: "High Risk" },
  { id: "experimental", label: "Experimental" },
];

function applyFilter(tweaks: UnifiedTweak[], mode: FilterMode, tweakStates: Record<string, TweakState>): UnifiedTweak[] {
  switch (mode) {
    case "implemented":  return tweaks.filter((t) => t.isImplemented);
    case "applied":      return tweaks.filter((t) => tweakStates[t.id]?.isApplied);
    case "experimental": return tweaks.filter((t) => t.isExperimental);
    case "safe":         return tweaks.filter((t) => t.riskLevel === "safe" || t.riskLevel === "low-risk");
    case "high-risk":    return tweaks.filter((t) =>
      t.riskLevel === "high-risk" || t.riskLevel === "dangerous" ||
      t.riskLevel === "medium-risk" || t.riskLevel === "unproven"
    );
    default: return tweaks;
  }
}

interface CategoryPageProps {
  category: TweakCategory;
  tweakStates: Record<string, TweakState>;
  hasNvidia: boolean;
  hasAmd: boolean;
  isAdmin: boolean;
  logs: LogEntry[];
  pendingApplyId: string | null;
  onRequestApply: (id: string) => void;
  onRevert: (id: string) => void;
  onConfirmApply: () => void;
  onCancelApply: () => void;
  onClearLogs: () => void;
  onRestartAsAdmin: () => void;
}

export function CategoryPage({
  category,
  tweakStates,
  hasNvidia,
  hasAmd,
  isAdmin,
  logs,
  pendingApplyId,
  onRequestApply,
  onRevert,
  onConfirmApply,
  onCancelApply,
  onClearLogs,
  onRestartAsAdmin,
}: CategoryPageProps) {
  const [filter, setFilter] = useState<FilterMode>("all");

  const categoryTweaks = useMemo(
    () => ALL_TWEAKS.filter((t) => t.category === category.id),
    [category.id]
  );

  const visibleTweaks = useMemo(
    () => applyFilter(categoryTweaks, filter, tweakStates),
    [categoryTweaks, filter, tweakStates]
  );

  const pendingTweak     = pendingApplyId ? tweakStates[pendingApplyId]?.tweak : null;
  const implementedCount = categoryTweaks.filter((t) => t.isImplemented).length;
  const appliedCount     = categoryTweaks.filter((t) => tweakStates[t.id]?.isApplied).length;

  return (
    <div className="category-page">
      <div className="page-scroll">
        <div className="content-container">
          {/* ── Admin banner ──────────────────────────────────── */}
          {!isAdmin && <AdminBanner onRestartAsAdmin={onRestartAsAdmin} />}

          {/* ── Header ──────────────────────────────────────── */}
          <header className="content-header">
            <span className="content-header-icon">{categoryIcon(category.id)}</span>
            <div className="content-header-text">
              <h1 className="content-header-title">{category.name}</h1>
              <span className="content-header-count">
                {implementedCount} ready
                {appliedCount > 0 && ` · ${appliedCount} applied`}
              </span>
            </div>
          </header>

          {/* ── Filter bar ──────────────────────────────────── */}
          <div className="filter-toolbar">
            {FILTER_LABELS.map(({ id, label }) => (
              <button
                key={id}
                className={cn("filter-chip", filter === id && "filter-chip--active")}
                onClick={() => setFilter(id)}
              >
                {label}
              </button>
            ))}
            <span className="filter-toolbar-spacer" />
            <span style={{ fontSize: "11px", color: "var(--subtle)" }}>
              {visibleTweaks.length} shown
            </span>
          </div>

          {/* ── Tweak list ──────────────────────────────────── */}
          {visibleTweaks.length === 0 ? (
            <p className="cat-empty">No tweaks match this filter.</p>
          ) : (
            <div className="tweak-list">
              {visibleTweaks.map((tweak) => (
                <CompactTweakRow
                  key={tweak.id}
                  state={tweakStates[tweak.id]}
                  hasNvidia={hasNvidia}
                  hasAmd={hasAmd}
                  isAdmin={isAdmin}
                  onRequestApply={onRequestApply}
                  onRevert={onRevert}
                  onRestartAsAdmin={onRestartAsAdmin}
                />
              ))}
            </div>
          )}
        </div>
      </div>

      {/* ── Log drawer — fixed to page bottom ──────────────── */}
      <BottomLogDrawer logs={logs} onClear={onClearLogs} />

      {pendingTweak && (
        <RiskModal
          tweak={pendingTweak}
          onConfirm={onConfirmApply}
          onCancel={onCancelApply}
        />
      )}
    </div>
  );
}
