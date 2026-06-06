import { useState } from "react";
import { Play, RotateCcw, Lock, ChevronDown } from "lucide-react";
import { cn } from "../lib/cn";
import type { TweakState } from "../types";

interface CompactTweakRowProps {
  state: TweakState;
  hasNvidia: boolean;
  hasAmd: boolean;
  isAdmin: boolean;
  onRequestApply: (id: string) => void;
  onRevert: (id: string) => void;
  onRestartAsAdmin: () => void;
}

const RISK_LABELS: Record<string, string> = {
  "safe":        "Safe",
  "low-risk":    "Low risk",
  "medium-risk": "Medium risk",
  "high-risk":   "High risk",
  "dangerous":   "Dangerous",
  "unproven":    "Unproven",
};

export function CompactTweakRow({
  state,
  hasNvidia,
  hasAmd,
  isAdmin,
  onRequestApply,
  onRevert,
  onRestartAsAdmin,
}: CompactTweakRowProps) {
  const { tweak, status, isApplied } = state;
  const [expanded, setExpanded] = useState(false);

  const isBusy          = status === "applying" || status === "reverting";
  const isNvidiaBlocked = tweak.nvidiaOnly === true && !hasNvidia;
  const isAmdBlocked    = tweak.amdOnly === true && !hasAmd;
  const isAdminLocked   = tweak.requiresAdmin === true && !isAdmin;
  const isUnavailable   = !tweak.isImplemented || isNvidiaBlocked || isAmdBlocked;

  const hasDetail =
    tweak.riskExplanation ||
    tweak.sideEffects.length > 0 ||
    tweak.tags?.length ||
    tweak.isExperimental ||
    tweak.requiresAdmin ||
    tweak.nvidiaOnly ||
    tweak.amdOnly ||
    tweak.requiresRestart;

  return (
    <div
      className={cn(
        "compact-tweak-row",
        isApplied          && "compact-tweak-row--applied",
        status === "error" && "compact-tweak-row--error",
        isAdminLocked      && "compact-tweak-row--admin-locked",
        isUnavailable && !isAdminLocked && "compact-tweak-row--dimmed"
      )}
    >
      {/* ── Main row ───────────────────────────────────────────────────────── */}
      <div
        className="ctr-main"
        onClick={() => hasDetail && setExpanded((v) => !v)}
        style={{ cursor: hasDetail ? "pointer" : "default" }}
      >
        {/* Risk dot */}
        <span className={cn("risk-dot ctr-dot", `risk-dot--${tweak.riskLevel}`)} />

        {/* Info */}
        <div className="ctr-info">
          <span className="ctr-name">{tweak.name}</span>
          <span className="ctr-desc">{tweak.description}</span>
        </div>

        {/* Trailing: applied pill + expand arrow */}
        <div className="ctr-trailing">
          {isApplied && (
            <span className="applied-pill">
              <span style={{ fontSize: 10 }}>✓</span> Applied
            </span>
          )}
          {status === "error" && (
            <span style={{ fontSize: 11, color: "var(--danger)", fontWeight: 600 }}>Error</span>
          )}
          {hasDetail && (
            <ChevronDown
              size={14}
              strokeWidth={2}
              style={{
                color: "var(--text-faint)",
                transition: "transform 0.15s",
                transform: expanded ? "rotate(180deg)" : "none",
                flexShrink: 0,
              }}
            />
          )}
        </div>

        {/* Action — separated so it doesn't trigger expand */}
        <div className="ctr-action" onClick={(e) => e.stopPropagation()}>
          {isAdminLocked ? (
            <div className="ctr-admin-lock">
              <Lock size={12} strokeWidth={2} style={{ color: "var(--warning)" }} />
              <button className="btn btn--restart-admin" onClick={onRestartAsAdmin}>
                Run as Admin
              </button>
            </div>
          ) : isNvidiaBlocked ? (
            <span className="ctr-unavailable">No NVIDIA GPU</span>
          ) : isAmdBlocked ? (
            <span className="ctr-unavailable">No AMD GPU</span>
          ) : !tweak.isImplemented ? (
            <span className="ctr-unavailable">Coming soon</span>
          ) : !isApplied ? (
            <button
              className="btn btn--apply btn--sm"
              onClick={() => onRequestApply(tweak.id)}
              disabled={isBusy}
            >
              {status === "applying" ? (
                "Applying…"
              ) : (
                <><Play size={10} strokeWidth={2.5} /> Apply</>
              )}
            </button>
          ) : (
            <button
              className="btn btn--revert btn--sm"
              onClick={() => onRevert(tweak.id)}
              disabled={isBusy}
            >
              {status === "reverting" ? (
                "Reverting…"
              ) : (
                <><RotateCcw size={10} strokeWidth={2.5} /> Revert</>
              )}
            </button>
          )}
        </div>
      </div>

      {/* ── Expanded detail ─────────────────────────────────────────────────── */}
      {expanded && hasDetail && (
        <div className="ctr-detail">
          {/* Pills row */}
          <div className="ctr-detail-pills">
            <span className={cn("risk-badge", `risk-badge--${tweak.riskLevel}`)}>
              {RISK_LABELS[tweak.riskLevel]}
            </span>
            {tweak.isExperimental && (
              <span className="exp-badge exp-badge--exp-tag">Experimental</span>
            )}
            {tweak.requiresAdmin && (
              <span className="exp-badge exp-badge--admin">Admin required</span>
            )}
            {tweak.nvidiaOnly && (
              <span className="exp-badge exp-badge--nvidia">NVIDIA only</span>
            )}
            {tweak.amdOnly && (
              <span className="exp-badge exp-badge--amd">AMD only</span>
            )}
            {tweak.requiresRestart && (
              <span className="exp-badge exp-badge--restart">Restart required</span>
            )}
            {tweak.tags?.map((tag) => (
              <span key={tag} className="tag">{tag}</span>
            ))}
          </div>

          {/* Risk explanation */}
          {tweak.riskExplanation && (
            <p className="ctr-detail-text">{tweak.riskExplanation}</p>
          )}

          {/* Side effects */}
          {tweak.sideEffects.length > 0 && (
            <div>
              <p className="ctr-detail-effects-label">Side effects</p>
              <ul className="ctr-detail-effects-list">
                {tweak.sideEffects.map((se, i) => (
                  <li key={i}>{se}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
