import { useCallback, useEffect, useState } from 'react';

export function useResource<T>(fetcher: () => Promise<T>) {
  const [data, setData] = useState<T | undefined>();
  const [error, setError] = useState<Error | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(() => {
    setLoading(true);
    setError(null);
    fetcher().then(setData).catch(setError).finally(() => setLoading(false));
  }, [fetcher]);

  useEffect(() => { load(); }, [load]);

  return { data, error, loading, refetch: load } as const;
}
