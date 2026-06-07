import { useState, useEffect, useCallback, useRef } from "react";
import { getCachedEntry, setCachedEntry } from "../lib/resourceCache";

/**
 * Canonical async-resource state model shared by every cached system read.
 *
 *  - `isLoading`    : true ONLY when there is no cached data yet and a fetch runs.
 *  - `isRefreshing` : true when cached data already exists and a (manual or
 *                     background) refresh is running — the UI keeps showing the
 *                     previous data, it never blanks.
 *  - on error, previous `data` is preserved and `error` is set (non-blocking).
 *  - `lastUpdated`  : epoch ms of the last successful fetch.
 */
export interface ResourceState<T> {
  data: T | null;
  isLoading: boolean;
  isRefreshing: boolean;
  lastUpdated: number | null;
  error: string | null;
  refresh: () => Promise<void>;
}

export interface CachedResourceOptions {
  /** Auto-refresh interval in ms (use for cheap live metrics). Omit = fetch once. */
  pollMs?: number;
  /** Re-fetch in the background on mount even if a cached value exists. */
  refreshOnMount?: boolean;
  /** When false, the hook is idle (no fetch, no poll). */
  enabled?: boolean;
}

export function useCachedResource<T>(
  key: string,
  fetcher: () => Promise<T>,
  options: CachedResourceOptions = {}
): ResourceState<T> {
  const { pollMs, refreshOnMount = false, enabled = true } = options;

  const cached = getCachedEntry<T>(key);
  const [data, setData] = useState<T | null>(cached?.data ?? null);
  const [lastUpdated, setLastUpdated] = useState<number | null>(
    cached?.lastUpdated ?? null
  );
  const [isLoading, setIsLoading] = useState<boolean>(!cached);
  const [isRefreshing, setIsRefreshing] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  // Keep the latest fetcher without making `run` change identity.
  const fetcherRef = useRef(fetcher);
  fetcherRef.current = fetcher;
  const inFlight = useRef(false);
  const mounted = useRef(true);

  const run = useCallback(async () => {
    if (inFlight.current) return;
    inFlight.current = true;
    // Cache presence (not local state) decides loading vs refreshing, so the
    // callback stays stable and polling/refresh classify correctly.
    const hasData = getCachedEntry<T>(key) !== undefined;
    if (hasData) setIsRefreshing(true);
    else setIsLoading(true);
    setError(null);
    try {
      const result = await fetcherRef.current();
      const ts = setCachedEntry(key, result);
      if (mounted.current) {
        setData(result);
        setLastUpdated(ts);
      }
    } catch (e) {
      // Keep previous data; expose a non-blocking error.
      if (mounted.current) setError(String(e));
    } finally {
      if (mounted.current) {
        setIsLoading(false);
        setIsRefreshing(false);
      }
      inFlight.current = false;
    }
  }, [key]);

  // Initial load / remount behaviour.
  useEffect(() => {
    mounted.current = true;
    if (!enabled) return;
    const existing = getCachedEntry<T>(key);
    if (!existing) {
      void run(); // first visit, no cache → real load
    } else {
      // Show cached data instantly; only re-scan if explicitly requested.
      setData(existing.data);
      setLastUpdated(existing.lastUpdated);
      setIsLoading(false);
      if (refreshOnMount) void run();
    }
    return () => {
      mounted.current = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, enabled]);

  // Optional background polling for live metrics.
  useEffect(() => {
    if (!enabled || !pollMs) return;
    const id = setInterval(() => void run(), pollMs);
    return () => clearInterval(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, pollMs, key]);

  const refresh = useCallback(async () => {
    await run();
  }, [run]);

  return { data, isLoading, isRefreshing, lastUpdated, error, refresh };
}
