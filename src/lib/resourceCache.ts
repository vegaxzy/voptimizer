// Runtime, in-memory cache for expensive system reads.
//
// This deliberately lives in module scope (not React state) so cached results
// survive component unmount/remount — i.e. navigating away from a page and back
// does NOT re-run the underlying scan. Nothing here is persisted to disk; the
// cache is cleared when the app process exits.

export interface CacheEntry<T> {
  data: T;
  lastUpdated: number; // epoch ms of the successful fetch
}

const store = new Map<string, CacheEntry<unknown>>();

export function getCachedEntry<T>(key: string): CacheEntry<T> | undefined {
  return store.get(key) as CacheEntry<T> | undefined;
}

/** Stores a successful result and returns the timestamp it was stored at. */
export function setCachedEntry<T>(key: string, data: T): number {
  const lastUpdated = Date.now();
  store.set(key, { data, lastUpdated });
  return lastUpdated;
}

export function clearCachedEntry(key: string): void {
  store.delete(key);
}

export function clearAllCache(): void {
  store.clear();
}
