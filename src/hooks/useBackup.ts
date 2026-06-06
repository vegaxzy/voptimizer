import { useState, useCallback, useEffect } from "react";
import { toast } from "sonner";
import type { BackupEntry, HistoryEntry, BackupOpResult, RestorePointStatus } from "../types/backup";
import type { LogEntry, LogLevel } from "../types";
import * as api from "../invoke/backup";
import { useAppStore } from "../store/useAppStore";

function makeLog(message: string, level: LogLevel): LogEntry {
  return {
    id: `${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
    timestamp: new Date(),
    message,
    level,
    source: "backup",
  };
}

export function useBackup() {
  const [backups, setBackups] = useState<BackupEntry[]>([]);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [restoreStatus, setRestoreStatus] = useState<RestorePointStatus | null>(null);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [busyIds, setBusyIds] = useState<Set<string>>(new Set());

  const storeAddLog = useAppStore((s) => s.addLog);

  const addLog = useCallback(
    (msg: string, level: LogLevel) => {
      setLogs((prev) => [makeLog(msg, level), ...prev]);
      storeAddLog(msg, level, "backup");
    },
    [storeAddLog]
  );

  const setId = (id: string, busy: boolean) =>
    setBusyIds((prev) => {
      const next = new Set(prev);
      busy ? next.add(id) : next.delete(id);
      return next;
    });

  const handleResult = useCallback(
    (result: BackupOpResult, successMsg?: string): boolean => {
      if (result.success) {
        addLog(successMsg ?? result.message, "success");
      } else {
        addLog(result.error ?? result.message, "error");
      }
      return result.success;
    },
    [addLog]
  );

  // ── Load ──────────────────────────────────────────────────────────────────

  const refreshBackups = useCallback(async () => {
    try {
      const data = await api.listBackups();
      setBackups(data);
    } catch (err) {
      addLog(`Failed to load backups: ${String(err)}`, "error");
    }
  }, [addLog]);

  const refreshHistory = useCallback(async () => {
    try {
      const data = await api.listHistory();
      setHistory(data);
    } catch (err) {
      addLog(`Failed to load history: ${String(err)}`, "error");
    }
  }, [addLog]);

  const refreshRestoreStatus = useCallback(async () => {
    try {
      const status = await api.checkRestorePointStatus();
      setRestoreStatus(status);
    } catch (err) {
      addLog(`Failed to check restore point status: ${String(err)}`, "error");
    }
  }, [addLog]);

  const refreshAll = useCallback(async () => {
    setIsLoading(true);
    await Promise.all([refreshBackups(), refreshHistory(), refreshRestoreStatus()]);
    setIsLoading(false);
  }, [refreshBackups, refreshHistory, refreshRestoreStatus]);

  useEffect(() => {
    refreshAll();
  }, []);

  // ── Backup actions ────────────────────────────────────────────────────────

  const createBackup = useCallback(
    async (label: string, registryKey: string) => {
      const opId = `create-${Date.now()}`;
      setId(opId, true);
      addLog(`Creating backup: ${label} (${registryKey})`, "info");
      try {
        const result = await api.createRegistryBackup(label, registryKey);
        if (handleResult(result)) {
          if (result.data) setBackups((prev) => [result.data!, ...prev]);
          toast.success(`Backup created: ${label}`);
        } else {
          toast.error("Backup failed", { description: result.error ?? result.message });
        }
        await refreshHistory();
      } catch (err) {
        addLog(`Create backup failed: ${String(err)}`, "error");
        toast.error("Backup failed", { description: String(err) });
      } finally {
        setId(opId, false);
      }
    },
    [addLog, handleResult, refreshHistory]
  );

  const restoreBackup = useCallback(
    async (id: string) => {
      if (busyIds.has(id)) return;
      setId(id, true);
      const b = backups.find((x) => x.id === id);
      addLog(`Restoring backup: ${b?.label ?? id}`, "info");
      try {
        const result = await api.restoreRegistryFile(id);
        handleResult(result);
        if (result.success) {
          toast.success(`Restore complete: ${b?.label ?? id}`);
        } else {
          toast.error("Restore failed", { description: result.error ?? result.message });
        }
        await refreshHistory();
      } catch (err) {
        addLog(`Restore failed: ${String(err)}`, "error");
        toast.error("Restore failed", { description: String(err) });
      } finally {
        setId(id, false);
      }
    },
    [backups, busyIds, addLog, handleResult, refreshHistory]
  );

  const deleteBackup = useCallback(
    async (id: string) => {
      if (busyIds.has(id)) return;
      setId(id, true);
      const b = backups.find((x) => x.id === id);
      addLog(`Deleting backup: ${b?.label ?? id}`, "info");
      try {
        const result = await api.deleteBackup(id);
        if (handleResult(result)) {
          setBackups((prev) => prev.filter((x) => x.id !== id));
        }
        await refreshHistory();
      } catch (err) {
        addLog(`Delete failed: ${String(err)}`, "error");
        toast.error("Delete failed", { description: String(err) });
      } finally {
        setId(id, false);
      }
    },
    [backups, busyIds, addLog, handleResult, refreshHistory]
  );

  // ── Restore point ─────────────────────────────────────────────────────────

  const createRestorePoint = useCallback(
    async (description: string) => {
      const opId = `rp-${Date.now()}`;
      setId(opId, true);
      addLog(`Creating restore point: "${description}"`, "info");
      try {
        const result = await api.createRestorePoint(description);
        handleResult(result);
        if (result.success) {
          toast.success("Restore point created", { description });
        } else {
          toast.error("Restore point failed", { description: result.error ?? result.message });
        }
        await refreshHistory();
      } catch (err) {
        addLog(`Restore point failed: ${String(err)}`, "error");
        toast.error("Restore point failed", { description: String(err) });
      } finally {
        setId(opId, false);
      }
    },
    [addLog, handleResult, refreshHistory]
  );

  // ── History ───────────────────────────────────────────────────────────────

  const doClearHistory = useCallback(async () => {
    try {
      const result = await api.clearHistory();
      if (result.success) {
        setHistory([]);
        addLog("History cleared", "success");
      }
    } catch (err) {
      addLog(`Clear history failed: ${String(err)}`, "error");
    }
  }, [addLog]);

  const clearLogs = useCallback(() => setLogs([]), []);

  return {
    backups,
    history,
    restoreStatus,
    logs,
    isLoading,
    busyIds,
    refreshAll,
    createBackup,
    restoreBackup,
    deleteBackup,
    createRestorePoint,
    doClearHistory,
    clearLogs,
  };
}
