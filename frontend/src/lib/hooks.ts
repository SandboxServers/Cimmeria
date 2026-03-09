import { useCallback, useEffect, useRef, useState } from 'react';

/** True when the error is a connection-level failure (server unreachable). */
export function isNetworkError(error: unknown): boolean {
  if (!error || typeof error !== 'object') return false;
  return (error as Error).name === 'ConnectionError';
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
