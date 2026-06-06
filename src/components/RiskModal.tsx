import { useState } from "react";
import type { UnifiedTweak } from "../types";

interface RiskModalProps {
  tweak: UnifiedTweak;
  onConfirm: () => void;
  onCancel: () => void;
}

const RISK_LABELS: Record<string, string> = {
  "safe": "Safe",
  "low-risk": "Low Risk",
  "medium-risk": "Medium Risk",
  "high-risk": "High Risk",
  "dangerous": "Dangerous",
  "unproven": "Unproven",
};

const BENEFIT_LABELS: Record<string, string> = {
  none: "No measurable benefit",
  low: "Minor",
  medium: "Moderate",
  high: "Significant",
};

export function RiskModal({ tweak, onConfirm, onCancel }: RiskModalProps) {
  const [understood, setUnderstood] = useState(false);

  return (
    <div className="risk-modal-overlay" onClick={onCancel}>
      <div className="risk-modal" onClick={(e) => e.stopPropagation()}>

        <div className="risk-modal-header">
          <div className="risk-modal-header-left">
            <span className={`risk-badge risk-badge--${tweak.riskLevel}`}>
              {RISK_LABELS[tweak.riskLevel]}
            </span>
            <h2 className="risk-modal-title">{tweak.name}</h2>
          </div>
          <button className="risk-modal-close" onClick={onCancel} aria-label="Close">✕</button>
        </div>

        <div className="risk-modal-body">
          <div className="risk-modal-section">
            <p className="risk-modal-section-label">Risk Explanation</p>
            <p className="risk-modal-text">{tweak.riskExplanation}</p>
          </div>

          {tweak.sideEffects.length > 0 && (
            <div className="risk-modal-section">
              <p className="risk-modal-section-label">Possible Side Effects</p>
              <ul className="risk-modal-list">
                {tweak.sideEffects.map((se, i) => (
                  <li key={i} className="risk-modal-list-item">{se}</li>
                ))}
              </ul>
            </div>
          )}

          <div className="risk-modal-meta-row">
            <span className="risk-modal-meta-label">Expected Benefit</span>
            <span className={`benefit-chip benefit-chip--${tweak.expectedBenefit}`}>
              {BENEFIT_LABELS[tweak.expectedBenefit]}
            </span>
            {tweak.requiresRestart && (
              <span className="exp-badge exp-badge--restart">↻ Requires Restart</span>
            )}
            {tweak.requiresRestorePoint && (
              <span className="exp-badge exp-badge--restore-point">🛡 Restore Point Advised</span>
            )}
          </div>

          {tweak.requiresRestorePoint && (
            <div className="risk-modal-restore-warn">
              <strong>⚠</strong> Create a System Restore Point before applying.
              Go to <em>Backup &amp; Restore → System Restore Point</em>.
            </div>
          )}

          <label className="risk-modal-understand">
            <input
              type="checkbox"
              checked={understood}
              onChange={(e) => setUnderstood(e.currentTarget.checked)}
            />
            <span>I have read the side effects and understand the risks</span>
          </label>
        </div>

        <div className="risk-modal-footer">
          <button className="btn btn--ghost" onClick={onCancel}>Cancel</button>
          <button
            className="btn btn--apply"
            disabled={!understood}
            onClick={onConfirm}
          >
            Apply Tweak
          </button>
        </div>

      </div>
    </div>
  );
}
