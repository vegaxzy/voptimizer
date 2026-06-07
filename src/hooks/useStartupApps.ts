import { useState, useCallback, useEffect, useRef } from "react";
import { toast } from "sonner";
import type { StartupApp } from "../types/startup";
import type { LogEntry, LogLevel } from "../types";
import { listStartupApps, disableStartupApp, enableStartupApp } from "../invoke/startup";
import { useAppStore } from "../store/useAppStore";
import { getCachedEntry, setCachedEntry } from "../lib/resourceCache";

const CACHE_KEY = "startup-apps";

function makeLog(message: string, level: LogLevel, appId?: string): LogEntry {
  return {
    id: `${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
    timestamp: new Date(),
    message,
    level,
    tweakId: appId,
    source: "startup",
  };
}

export function useStartupApps() {
  // Seed synchronously from the runtime cache so revisiting the page is instant
  // and does NOT trigger a fresh registry/folder scan.
  const cached = getCachedEntry<StartupApp[]>(CACHE_KEY);
  const [apps, setApps] = useState<StartupApp[]>(cached?.data ?? []);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [isLoading, setIsLoading] = useState(!cached);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [lastUpdated, setLastUpdated] = useState<number | null>(
    cached?.lastUpdated ?? null
  );
  const [error, setError] = useState<string | null>(null);
  const [busyIds, setBusyIds] = useState<Set<string>>(new Set());
  const didInit = useRef(false);

  const storeAddLog = useAppStore((s) => s.addLog);

  const addLog = useCallback(
    (msg: string, level: LogLevel, appId?: string) => {
      setLogs((prev) => [makeLog(msg, level, appId), ...prev]);
      storeAddLog(msg, level, "startup");
    },
    [storeAddLog]
  );

  /** Re-scan all startup sources. Keeps previous data visible while running. */
  const refresh = useCallback(async () => {
    const hasData = getCachedEntry<StartupApp[]>(CACHE_KEY) !== undefined;
    if (hasData) setIsRefreshing(true);
    else setIsLoading(true);
    setError(null);
    addLog("Refreshing startup apps list…", "info");
    try {
      const result = await listStartupApps();
      setApps(result);
      setLastUpdated(setCachedEntry(CACHE_KEY, result));
      addLog(`Found ${result.length} startup entries.`, "success");
    } catch (err) {
      // Keep previous data; surface a non-blocking error.
      setError(String(err));
      addLog(`Failed to list startup apps: ${String(err)}`, "error");
      toast.error("Failed to load startup apps", { description: String(err) });
    } finally {
      setIsLoading(false);
      setIsRefreshing(false);
    }
  }, [addLog]);

  // Only scan on first mount when there's no cached data.
  useEffect(() => {
    if (didInit.current) return;
    didInit.current = true;
    if (!cached) void refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const setBusy = (id: string, busy: boolean) =>
    setBusyIds((prev) => {
      const next = new Set(prev);
      busy ? next.add(id) : next.delete(id);
      return next;
    });

  // Apply an updated entry to both local state and the runtime cache so they
  // never drift apart (the cache is what later page visits read from).
  const applyUpdate = useCallback((updated: StartupApp) => {
    setApps((prev) => {
      const next = prev.map((a) => (a.id === updated.id ? updated : a));
      setCachedEntry(CACHE_KEY, next);
      return next;
    });
  }, []);

  const disable = useCallback(
    async (id: string) => {
      if (busyIds.has(id)) return;
      setBusy(id, true);
      const app = apps.find((a) => a.id === id);
      addLog(`Disabling: ${app?.name ?? id}`, "info", id);
      try {
        const result = await disableStartupApp(id);
        if (result.success && result.data) {
          applyUpdate(result.data);
          addLog(result.message, "success", id);
          toast.success(`Disabled: ${app?.name ?? id}`);
        } else {
          const msg = result.error ?? result.message ?? "Unknown error";
          addLog(msg, "error", id);
          toast.error("Failed to disable", { description: app?.name });
        }
      } catch (err) {
        addLog(`Disable failed: ${String(err)}`, "error", id);
        toast.error("Disable failed", { description: String(err) });
      } finally {
        setBusy(id, false);
      }
    },
    [apps, busyIds, addLog, applyUpdate]
  );

  const enable = useCallback(
    async (id: string) => {
      if (busyIds.has(id)) return;
      setBusy(id, true);
      const app = apps.find((a) => a.id === id);
      addLog(`Enabling: ${app?.name ?? id}`, "info", id);
      try {
        const result = await enableStartupApp(id);
        if (result.success && result.data) {
          applyUpdate(result.data);
          addLog(result.message, "success", id);
          toast.success(`Enabled: ${app?.name ?? id}`);
        } else {
          const msg = result.error ?? result.message ?? "Unknown error";
          addLog(msg, "error", id);
          toast.error("Failed to enable", { description: app?.name });
        }
      } catch (err) {
        addLog(`Enable failed: ${String(err)}`, "error", id);
        toast.error("Enable failed", { description: String(err) });
      } finally {
        setBusy(id, false);
      }
    },
    [apps, busyIds, addLog, applyUpdate]
  );

  const clearLogs = useCallback(() => setLogs([]), []);

  return {
    apps,
    logs,
    isLoading,
    isRefreshing,
    lastUpdated,
    error,
    busyIds,
    refresh,
    disable,
    enable,
    clearLogs,
  };
}
