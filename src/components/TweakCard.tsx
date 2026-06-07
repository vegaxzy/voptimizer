import { ShieldAlert, Check, AlertTriangle, RotateCw, Lock } from "lucide-react";
import type { TweakState } from "../types";

interface TweakCardProps {
  state: TweakState;
  hasNvidia: boolean;
  hasAmd: boolean;
  isAdmin: boolean;
  onRequestApply: (id: string) => void;
  onRevert: (id: string) => void;
  onRestartAsAdmin: () => void;
}

const RISK_LABELS: Record<string, string> = {
  "safe": "Safe",
  "low-risk": "Low Risk",
  "medium-risk": "Medium Risk",
  "high-risk": "High Risk",
  "dangerous": "Dangerous",
  "unproven": "Unproven",
};

export function TweakCard({
  state,
  hasNvidia,
  hasAmd,
  isAdmin,
  onRequestApply,
  onRevert,
  onRestartAsAdmin,
}: TweakCardProps) {
  const { tweak, status, isApplied } = state;
  const isBusy = status === "applying" || status === "reverting";

  const isNvidiaBlocked = tweak.nvidiaOnly === true && !hasNvidia;
  const isAmdBlocked = tweak.amdOnly === true && !hasAmd;
  const isAdminLocked = tweak.requiresAdmin === true && !isAdmin;
  const isDisabled = !tweak.isImplemented || isNvidiaBlocked || isAmdBlocked || isAdminLocked;

  const cardClasses = [
    "tweak-card",
    `tweak-card--risk-${tweak.riskLevel}`,
    isApplied ? "tweak-card--applied" : "",
    status === "error" ? "tweak-card--error" : "",
    isAdminLocked ? "tweak-card--admin-locked" : (isDisabled ? "tweak-card--placeholder" : ""),
    tweak.isExperimental && !isDisabled ? "tweak-card--experimental" : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <article className={cardClasses}>
      <div className="tweak-card-header">
        <span className={`risk-badge risk-badge--${tweak.riskLevel}`}>
          {RISK_LABELS[tweak.riskLevel]}
        </span>
        {tweak.isExperimental && (
          <span className="exp-badge exp-badge--exp-tag">Experimental</span>
        )}
        {tweak.requiresAdmin && (
          <span className="exp-badge exp-badge--admin"><ShieldAlert size={10} strokeWidth={2} /> Admin</span>
        )}
        {tweak.nvidiaOnly && (
          <span className="exp-badge exp-badge--nvidia">NVIDIA</span>
        )}
        {tweak.amdOnly && (
          <span className="exp-badge exp-badge--amd">AMD</span>
        )}
        {!tweak.isImplemented && (
          <span className="exp-badge exp-badge--placeholder">Coming Soon</span>
        )}
        {isApplied && <span className="applied-indicator"><Check size={11} strokeWidth={2.5} /> Applied</span>}
        {status === "error" && <span className="error-indicator"><AlertTriangle size={11} strokeWidth={2} /> Error</span>}
      </div>

      <h3 className="tweak-card-name">{tweak.name}</h3>
      <p className="tweak-card-description">{tweak.description}</p>

      {(tweak.isExperimental || tweak.riskLevel === "high-risk" || tweak.riskLevel === "dangerous") &&
        tweak.isImplemented && !isAdminLocked && (
          <div className="exp-card-risk-text">{tweak.riskExplanation}</div>
        )}

      {tweak.sideEffects.length > 0 && (tweak.isExperimental || !tweak.isImplemented) && !isAdminLocked && (
        <div className="exp-card-effects">
          <p className="exp-card-effects-label">Side Effects</p>
          <ul className="exp-card-effects-list">
            {tweak.sideEffects.map((se, i) => (
              <li key={i}>{se}</li>
            ))}
          </ul>
        </div>
      )}

      {tweak.tags && tweak.tags.length > 0 && (
        <div className="tweak-card-tags">
          {tweak.tags.map((tag) => (
            <span key={tag} className="tag">
              {tag}
            </span>
          ))}
          {tweak.requiresRestart && (
            <span className="exp-badge exp-badge--restart"><RotateCw size={10} strokeWidth={2} /> Restart</span>
          )}
        </div>
      )}

      <div className="tweak-card-actions">
        {isAdminLocked ? (
          <div className="admin-lock-notice">
            <span className="admin-lock-icon"><Lock size={12} strokeWidth={2} /></span>
            <span className="admin-lock-text">Requires administrator</span>
            <button className="btn btn--restart-admin" onClick={onRestartAsAdmin}>
              Restart as Admin
            </button>
          </div>
        ) : isNvidiaBlocked ? (
          <p className="exp-no-gpu-note">NVIDIA GPU required</p>
        ) : isAmdBlocked ? (
          <p className="exp-no-gpu-note">AMD GPU required</p>
        ) : !tweak.isImplemented ? (
          <p className="exp-no-gpu-note">Not yet available</p>
        ) : !isApplied ? (
          <button
            className="btn btn--apply"
            onClick={() => onRequestApply(tweak.id)}
            disabled={isBusy}
          >
            {status === "applying" ? "Applying…" : "Apply"}
          </button>
        ) : (
          <button
            className="btn btn--revert"
            onClick={() => onRevert(tweak.id)}
            disabled={isBusy}
          >
            {status === "reverting" ? "Reverting…" : "Revert"}
          </button>
        )}
      </div>
    </article>
  );
}
