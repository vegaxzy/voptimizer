import { useState, useCallback, useEffect } from "react";
import { toast } from "sonner";
import type { TweakState, LogEntry, LogLevel, UnifiedTweak } from "../types";
import { ALL_TWEAKS } from "../data/tweaks";
import { applyTweak, revertTweak, checkAllTweakStatuses, detectNvidia, detectAmd, pickExeFile, PER_EXE_APPLY } from "../invoke/tweaks";
import { applyMinecraftPreset } from "../invoke/minecraft";
import { isRunningAsAdmin, restartAsAdmin } from "../invoke/admin";
import { useAppStore } from "../store/useAppStore";

function buildInitialState(): Record<string, TweakState> {
  return Object.fromEntries(
    ALL_TWEAKS.map((t) => [t.id, { tweak: t, status: "idle", isApplied: false }])
  );
}

function makeLogEntry(message: string, level: LogLevel, tweakId?: string): LogEntry {
  return {
    id: `${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
    timestamp: new Date(),
    message,
    level,
    tweakId,
    source: "tweak",
  };
}

function needsModal(tweak: UnifiedTweak): boolean {
  return (
    tweak.isExperimental ||
    tweak.riskLevel === "medium-risk" ||
    tweak.riskLevel === "high-risk" ||
    tweak.riskLevel === "dangerous" ||
    tweak.riskLevel === "unproven"
  );
}

export function useTweaks() {
  const [tweakStates, setTweakStates] = useState<Record<string, TweakState>>(buildInitialState);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [hasNvidia, setHasNvidia] = useState(false);
  const [hasAmd, setHasAmd] = useState(false);
  const [isAdmin, setIsAdmin] = useState(false);
  const [pendingApplyId, setPendingApplyId] = useState<string | null>(null);

  const { setIsAdmin: storeSetIsAdmin, addLog: storeAddLog } = useAppStore();

  useEffect(() => {
    const implementedIds = ALL_TWEAKS.filter((t) => t.isImplemented).map((t) => t.id);
    checkAllTweakStatuses(implementedIds)
      .then((statuses) => {
        setTweakStates((prev) => {
          const next = { ...prev };
          for (const [id, applied] of Object.entries(statuses)) {
            if (next[id]) next[id] = { ...next[id], isApplied: applied, status: applied ? "applied" : "idle" };
          }
          return next;
        });
      })
      .catch(() => {});

    detectNvidia().then(setHasNvidia).catch(() => {});
    detectAmd().then(setHasAmd).catch(() => {});

    isRunningAsAdmin()
      .then((v) => {
        setIsAdmin(v);
        storeSetIsAdmin(v);
      })
      .catch(() => {
        setIsAdmin(false);
        storeSetIsAdmin(false);
      });
  }, [storeSetIsAdmin]);

  const addLog = useCallback(
    (message: string, level: LogLevel, tweakId?: string) => {
      const entry = makeLogEntry(message, level, tweakId);
      setLogs((prev) => [entry, ...prev]);
      storeAddLog(message, level, "tweak");
    },
    [storeAddLog]
  );

  const setStatus = useCallback((id: string, patch: Partial<TweakState>) => {
    setTweakStates((prev) => ({
      ...prev,
      [id]: { ...prev[id], ...patch },
    }));
  }, []);

  const doApply = useCallback(
    async (tweakId: string) => {
      const state = tweakStates[tweakId];
      if (!state || state.status === "applying" || state.status === "reverting") return;

      setStatus(tweakId, { status: "applying" });
      addLog(`Applying: ${state.tweak.name}`, "info", tweakId);

      try {
        const result = await applyTweak(tweakId);
        if (result.success) {
          setStatus(tweakId, { status: "applied", isApplied: true });
          addLog(result.message, "success", tweakId);
          toast.success(result.message, { description: state.tweak.name });
        } else {
          setStatus(tweakId, { status: "error" });
          addLog(`${state.tweak.name}: ${result.message}`, "error", tweakId);
          toast.error(result.message, { description: state.tweak.name });
        }
      } catch (err) {
        const msg = `Failed to apply ${state.tweak.name}: ${String(err)}`;
        setStatus(tweakId, { status: "error" });
        addLog(msg, "error", tweakId);
        toast.error(`Apply failed`, { description: state.tweak.name });
      }
    },
    [tweakStates, setStatus, addLog]
  );

  /** Special apply flow for per-exe tweaks (fullscreen opt, process priority,
   *  GPU preference): opens a native file picker first, then applies to the
   *  chosen exe via the matching invoke from PER_EXE_APPLY. */
  const doApplyExeTweak = useCallback(
    async (tweakId: string) => {
      const state = tweakStates[tweakId];
      if (!state || state.status === "applying" || state.status === "reverting") return;

      const applyFn = PER_EXE_APPLY[tweakId];
      if (!applyFn) return;

      setStatus(tweakId, { status: "applying" });

      try {
        const path = await pickExeFile();
        if (!path) {
          // User cancelled the file dialog — reset silently
          setStatus(tweakId, { status: "idle" });
          return;
        }

        addLog(`Applying: ${state.tweak.name} → ${path}`, "info", tweakId);
        const result = await applyFn(path);

        if (result.success) {
          setStatus(tweakId, { status: "applied", isApplied: true });
          addLog(result.message, "success", tweakId);
          toast.success(result.message, { description: state.tweak.name });
        } else {
          setStatus(tweakId, { status: "error" });
          addLog(`${state.tweak.name}: ${result.message}`, "error", tweakId);
          toast.error(result.message, { description: state.tweak.name });
        }
      } catch (err) {
        setStatus(tweakId, { status: "error" });
        addLog(`Failed: ${String(err)}`, "error", tweakId);
        toast.error("Apply failed", { description: state.tweak.name });
      }
    },
    [tweakStates, setStatus, addLog]
  );

  const requestApply = useCallback(
    (tweakId: string) => {
      const state = tweakStates[tweakId];
      if (!state) return;
      // Per-exe tweaks need a file picker before applying
      if (PER_EXE_APPLY[tweakId]) {
        doApplyExeTweak(tweakId);
      } else if (needsModal(state.tweak)) {
        setPendingApplyId(tweakId);
      } else {
        doApply(tweakId);
      }
    },
    [tweakStates, doApply, doApplyExeTweak]
  );

  const confirmApply = useCallback(() => {
    if (pendingApplyId) {
      const id = pendingApplyId;
      setPendingApplyId(null);
      doApply(id);
    }
  }, [pendingApplyId, doApply]);

  const cancelApply = useCallback(() => {
    setPendingApplyId(null);
  }, []);

  const revert = useCallback(
    async (tweakId: string) => {
      const state = tweakStates[tweakId];
      if (!state || state.status === "applying" || state.status === "reverting") return;

      setStatus(tweakId, { status: "reverting" });
      addLog(`Reverting: ${state.tweak.name}`, "info", tweakId);

      try {
        const result = await revertTweak(tweakId);
        if (result.success) {
          setStatus(tweakId, { status: "idle", isApplied: false });
          addLog(result.message, "success", tweakId);
          toast.success(result.message, { description: state.tweak.name });
        } else {
          setStatus(tweakId, { status: "error" });
          addLog(`${state.tweak.name}: ${result.message}`, "error", tweakId);
          toast.error(result.message, { description: state.tweak.name });
        }
      } catch (err) {
        const msg = `Failed to revert ${state.tweak.name}: ${String(err)}`;
        setStatus(tweakId, { status: "error" });
        addLog(msg, "error", tweakId);
        toast.error(`Revert failed`, { description: state.tweak.name });
      }
    },
    [tweakStates, setStatus, addLog]
  );

  const clearLogs = useCallback(() => setLogs([]), []);

  const doRestartAsAdmin = useCallback(async () => {
    try {
      await restartAsAdmin();
    } catch {
      // UAC cancelled or failed — app stays open
    }
  }, []);

  const applyPreset = useCallback(
    async (tweakIds: string[]) => {
      addLog(`Applying preset (${tweakIds.length} tweaks)…`, "info");
      let successCount = 0;
      let failCount = 0;
      try {
        const results = await applyMinecraftPreset(tweakIds);
        for (const r of results) {
          if (r.success) {
            setStatus(r.tweak_id, { status: "applied", isApplied: true });
            addLog(r.message, "success", r.tweak_id);
            successCount++;
          } else {
            setStatus(r.tweak_id, { status: "error" });
            addLog(`${r.tweak_id}: ${r.message}`, "error", r.tweak_id);
            failCount++;
          }
        }
        if (failCount === 0) {
          toast.success(`Preset applied — ${successCount} tweaks`);
        } else {
          toast.warning(`Preset finished: ${successCount} ok, ${failCount} failed`);
        }
      } catch (err) {
        addLog(`Preset failed: ${String(err)}`, "error");
        toast.error("Preset failed", { description: String(err) });
      }
    },
    [addLog, setStatus]
  );

  return {
    tweakStates,
    logs,
    hasNvidia,
    hasAmd,
    isAdmin,
    pendingApplyId,
    requestApply,
    confirmApply,
    cancelApply,
    revert,
    clearLogs,
    applyPreset,
    restartAsAdmin: doRestartAsAdmin,
  };
}
