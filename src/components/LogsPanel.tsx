import { useState } from "react";
import { Info, CheckCircle2, XCircle, AlertTriangle, ChevronDown, ChevronUp, Trash2 } from "lucide-react";
import { cn } from "../lib/cn";
import type { LogEntry } from "../types";

interface LogsPanelProps {
  logs: LogEntry[];
  onClear: () => void;
}

const LEVEL_ICON: Record<string, React.ReactNode> = {
  info:    <Info size={11} strokeWidth={2} />,
  success: <CheckCircle2 size={11} strokeWidth={2} />,
  error:   <XCircle size={11} strokeWidth={2} />,
  warning: <AlertTriangle size={11} strokeWidth={2} />,
};

function formatTime(date: Date): string {
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

export function LogsPanel({ logs, onClear }: LogsPanelProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  return (
    <section className={cn("logs-panel", isExpanded && "logs-panel--expanded")}>
      <div className="logs-panel-header">
        <button
          className="logs-toggle"
          onClick={() => setIsExpanded((v) => !v)}
          aria-expanded={isExpanded}
        >
          {isExpanded
            ? <ChevronDown size={11} strokeWidth={2} />
            : <ChevronUp size={11} strokeWidth={2} />
          }
          <span className="logs-title">Activity Log</span>
          <span className="logs-count">{logs.length}</span>
        </button>
        {isExpanded && logs.length > 0 && (
          <button className="btn btn--ghost btn--sm" onClick={onClear} title="Clear log">
            <Trash2 size={11} strokeWidth={2} />
            Clear
          </button>
        )}
      </div>

      {isExpanded && (
        <div className="logs-body">
          {logs.length === 0 ? (
            <p className="logs-empty">No activity yet. Apply a tweak to get started.</p>
          ) : (
            <ul className="logs-list">
              {logs.map((entry) => (
                <li
                  key={entry.id}
                  className={cn("log-entry", `log-entry--${entry.level}`)}
                >
                  <span className="log-icon">{LEVEL_ICON[entry.level]}</span>
                  <span className="log-time">{formatTime(entry.timestamp)}</span>
                  <span className="log-message">{entry.message}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </section>
  );
}
