import { create } from "zustand";
import type { LogEntry, LogLevel } from "../types";

interface AppStore {
  /** Whether the process is running with administrator privileges. */
  isAdmin: boolean;
  /** Visible version string shown in the sidebar. */
  appVersion: string;
  /** Currently active page / category id. */
  currentPage: string;
  /** Global activity log aggregated from all features. */
  logs: LogEntry[];

  setIsAdmin: (v: boolean) => void;
  setCurrentPage: (page: string) => void;
  /** Append a log entry (capped at 300 entries). */
  addLog: (message: string, level: LogLevel, source?: LogEntry["source"]) => void;
  clearLogs: () => void;
}

export const useAppStore = create<AppStore>((set) => ({
  isAdmin: false,
  appVersion: "v1.9.0",
  currentPage: "",
  logs: [],

  setIsAdmin: (v) => set({ isAdmin: v }),
  setCurrentPage: (page) => set({ currentPage: page }),

  addLog: (message, level, source) =>
    set((state) => {
      const entry: LogEntry = {
        id: `${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
        timestamp: new Date(),
        message,
        level,
        source,
      };
      return { logs: [entry, ...state.logs.slice(0, 299)] };
    }),

  clearLogs: () => set({ logs: [] }),
}));
