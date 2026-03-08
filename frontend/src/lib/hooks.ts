import { useCallback, useEffect, useRef, useState } from 'react';

/** True when the error is a network-level failure (server unreachable). */
export function isNetworkError(error: unknown): boolean {
  if (!(error instanceof Error)) return false;
  if (error instanceof DOMException && error.name === 'AbortError') return false;
  // Raw fetch failure (no proxy)
  if (error instanceof TypeError && /failed to fetch|network/i.test(error.message)) return true;
  // Node/proxy error codes
  if (/ECONNREFUSED|ERR_CONNECTION_REFUSED/i.test(error.message)) return true;
  // Vite proxy returns 502 Bad Gateway when backend is down
  if (/failed with (502|503|504)/i.test(error.message)) return true;
  return false;
}

export function useResource<T>(fetcher: () => Promise<T>) {
  const [data, setData] = useState<T | undefined>();
  const [error, setError] = useState<Error | null>(null);
  const [loading, setLoading] = useState(true);
  const retryTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const load = useCallback(() => {
    setLoading(true);
    setError(null);
    fetcher().then(setData).catch(setError).finally(() => setLoading(false));
  }, [fetcher]);

  useEffect(() => { load(); }, [load]);

  // Auto-retry on network errors (server offline) every 5 seconds
  useEffect(() => {
    if (error && isNetworkError(error)) {
      retryTimer.current = setTimeout(load, 5000);
    }
    return () => clearTimeout(retryTimer.current);
  }, [error, load]);

  return { data, error, loading, refetch: load } as const;
}
