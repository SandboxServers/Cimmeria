import { RefreshCw, WifiOff } from 'lucide-react';
import { Button } from './ui/button';
import { isNetworkError } from '../lib/hooks';

/**
 * Checks a list of resource errors and returns a friendly message if the server
 * is unreachable, or null if errors are non-network (API-level) failures.
 */
export function getConnectionError(...errors: (Error | null | undefined)[]): Error | null {
  for (const err of errors) {
    if (err && isNetworkError(err)) return err;
  }
  return null;
}

/**
 * Banner shown when the backend server is unreachable.
 * Displays a friendly message with auto-retry indication and a manual retry button.
 */
export default function ConnectionStatus({ onRetry }: { onRetry: () => void }) {
  return (
    <div className="flex items-center gap-4 rounded-2xl border border-amber-400/20 bg-amber-400/5 px-5 py-4">
      <div className="rounded-xl border border-amber-400/20 bg-amber-400/10 p-2.5">
        <WifiOff className="size-5 text-amber-400" />
      </div>
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium text-foreground">Server unreachable</p>
        <p className="text-sm text-muted-foreground">
          The admin API is not responding. Make sure the Cimmeria server is running. Retrying automatically...
        </p>
      </div>
      <Button onClick={onRetry} variant="outline" size="sm" className="shrink-0">
        <RefreshCw className="size-3.5" />
        Retry
      </Button>
    </div>
  );
}
