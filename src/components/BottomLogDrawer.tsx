import { useState } from "react";
import {
  Info, CheckCircle2, XCircle, AlertTriangle,
  ChevronUp, ChevronDown, Trash2,
} from "lucide-react";
import { cn } from "../lib/cn";
import type { LogEntry } from "../types";

interface BottomLogDrawerProps {
  logs: LogEntry[];
  onClear: () => void;
}

const LEVEL_ICON: Record<string, React.ReactNode> = {
  info:    <Info size={10} strokeWidth={2} />,
  success: <CheckCircle2 size={10} strokeWidth={2} />,
  error:   <XCircle size={10} strokeWidth={2} />,
  warning: <AlertTriangle size={10} strokeWidth={2} />,
};

function formatTime(date: Date): string {
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

export function BottomLogDrawer({ logs, onClear }: BottomLogDrawerProps) {
  const [open, setOpen] = useState(false);

  return (
    <div className={cn("log-drawer", open ? "log-drawer--open" : "log-drawer--closed")}>
      {/* Bar — always visible */}
      <div className="log-drawer-bar" onClick={() => setOpen((v) => !v)}>
        <span className="log-drawer-bar-left">
          {open
            ? <ChevronDown size={10} strokeWidth={2} style={{ color: "var(--subtle)" }} />
            : <ChevronUp   size={10} strokeWidth={2} style={{ color: "var(--subtle)" }} />
          }
          <span className="log-drawer-title">Activity Log</span>
          {logs.length > 0 && (
            <span className="log-drawer-count">{logs.length}</span>
          )}
        </span>

        {open && logs.length > 0 && (
          <button
            className="btn btn--ghost btn--sm"
            style={{ height: 22, fontSize: 11 }}
            onClick={(e) => { e.stopPropagation(); onClear(); }}
            title="Clear log"
          >
            <Trash2 size={10} strokeWidth={2} />
            Clear
          </button>
        )}
      </div>

      {/* Body — only rendered when open */}
      {open && (
        <div className="log-drawer-body">
          {logs.length === 0 ? (
            <p className="log-drawer-empty">No activity yet.</p>
          ) : (
            <ul className="log-drawer-list">
              {logs.map((entry) => (
                <li key={entry.id} className={cn("log-drawer-entry", `log-drawer-entry--${entry.level}`)}>
                  <span className="log-drawer-icon">{LEVEL_ICON[entry.level]}</span>
                  <span className="log-drawer-time">{formatTime(entry.timestamp)}</span>
                  <span className="log-drawer-msg">{entry.message}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}
