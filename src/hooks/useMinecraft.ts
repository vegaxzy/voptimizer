import { useState, useCallback } from "react";
import type { SystemInfo, ProcessInfo, PingResult, DnsInfo } from "../invoke/minecraft";
import {
  getSystemInfo,
  listProcesses,
  killProcess,
  flushDns,
  getDnsInfo,
  pingHost,
} from "../invoke/minecraft";

export type McStatus = "idle" | "loading" | "ok" | "error";

export function useMinecraft() {
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
  const [systemInfoStatus, setSystemInfoStatus] = useState<McStatus>("idle");

  const [processes, setProcesses] = useState<ProcessInfo[]>([]);
  const [processesStatus, setProcessesStatus] = useState<McStatus>("idle");

  const [dnsInfo, setDnsInfo] = useState<DnsInfo | null>(null);
  const [dnsStatus, setDnsStatus] = useState<McStatus>("idle");

  const [pingResults, setPingResults] = useState<Record<string, PingResult>>({});
  const [pingStatus, setPingStatus] = useState<McStatus>("idle");

  const [mcLog, setMcLog] = useState<string[]>([]);

  const addLog = useCallback((msg: string) => {
    setMcLog((prev) => [`[${new Date().toLocaleTimeString()}] ${msg}`, ...prev.slice(0, 49)]);
  }, []);

  const refreshSystemInfo = useCallback(async () => {
    setSystemInfoStatus("loading");
    try {
      const info = await getSystemInfo();
      setSystemInfo(info);
      setSystemInfoStatus("ok");
    } catch (err) {
      setSystemInfoStatus("error");
      addLog(`System info error: ${String(err)}`);
    }
  }, [addLog]);

  const refreshProcesses = useCallback(async () => {
    setProcessesStatus("loading");
    try {
      const procs = await listProcesses();
      setProcesses(procs);
      setProcessesStatus("ok");
    } catch (err) {
      setProcessesStatus("error");
      addLog(`Process list error: ${String(err)}`);
    }
  }, [addLog]);

  const terminateProcess = useCallback(
    async (pid: number, name: string) => {
      addLog(`Killing ${name} (PID ${pid})…`);
      try {
        const result = await killProcess(pid, name);
        addLog(result.success ? result.message : `Failed: ${result.message}`);
        if (result.success) {
          setProcesses((prev) => prev.filter((p) => p.pid !== pid));
        }
        return result;
      } catch (err) {
        addLog(`Kill error: ${String(err)}`);
        return { success: false, message: String(err), error: String(err) };
      }
    },
    [addLog]
  );

  const doFlushDns = useCallback(async () => {
    addLog("Flushing DNS cache…");
    try {
      const result = await flushDns();
      addLog(result.success ? result.message : `Failed: ${result.message}`);
      return result;
    } catch (err) {
      addLog(`DNS flush error: ${String(err)}`);
      return { success: false, message: String(err), error: String(err) };
    }
  }, [addLog]);

  const refreshDnsInfo = useCallback(async () => {
    setDnsStatus("loading");
    try {
      const info = await getDnsInfo();
      setDnsInfo(info);
      setDnsStatus("ok");
    } catch (err) {
      setDnsStatus("error");
      addLog(`DNS info error: ${String(err)}`);
    }
  }, [addLog]);

  const runPing = useCallback(
    async (hosts: string[]) => {
      setPingStatus("loading");
      addLog(`Pinging ${hosts.join(", ")}…`);
      try {
        const results = await Promise.all(hosts.map((h) => pingHost(h)));
        const map: Record<string, PingResult> = {};
        for (let i = 0; i < hosts.length; i++) {
          map[hosts[i]] = results[i];
        }
        setPingResults(map);
        setPingStatus("ok");
        for (const [h, r] of Object.entries(map)) {
          addLog(r.success ? `${h}: ${r.latency_ms}ms` : `${h}: ${r.error ?? "timeout"}`);
        }
      } catch (err) {
        setPingStatus("error");
        addLog(`Ping error: ${String(err)}`);
      }
    },
    [addLog]
  );

  return {
    systemInfo,
    systemInfoStatus,
    refreshSystemInfo,
    processes,
    processesStatus,
    refreshProcesses,
    terminateProcess,
    dnsInfo,
    dnsStatus,
    refreshDnsInfo,
    doFlushDns,
    pingResults,
    pingStatus,
    runPing,
    mcLog,
  };
}
