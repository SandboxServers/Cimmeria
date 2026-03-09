import { useState } from 'react';
import { Database, Server, Loader2 } from 'lucide-react';

interface ConnectDialogProps {
  onConnect: (connectionString: string, serverUrl: string) => void;
  connecting: boolean;
  error: string | null;
}

export function ConnectDialog({ onConnect, connecting, error }: ConnectDialogProps) {
  const [connectionString, setConnectionString] = useState(
    'postgresql://w-testing:w-testing@localhost:5433/sgw'
  );
  const [serverUrl, setServerUrl] = useState('http://localhost:8443');

  return (
    <div className="flex h-screen items-center justify-center">
      <div className="w-[480px] rounded-lg border border-border bg-card p-8">
        <h1 className="mb-1 text-xl font-semibold text-foreground">
          Cimmeria Content Editor
        </h1>
        <p className="mb-6 text-sm text-muted-foreground">
          Connect to the database to begin editing content.
        </p>

        <div className="space-y-4">
          <div>
            <label className="mb-1.5 flex items-center gap-2 text-sm font-medium text-foreground">
              <Database className="h-4 w-4 text-muted-foreground" />
              PostgreSQL Connection
            </label>
            <input
              type="text"
              value={connectionString}
              onChange={(e) => setConnectionString(e.target.value)}
              className="w-full rounded border border-border bg-input px-3 py-2 font-mono text-sm text-foreground outline-none focus:border-primary"
              placeholder="postgresql://user:pass@host:port/db"
            />
          </div>

          <div>
            <label className="mb-1.5 flex items-center gap-2 text-sm font-medium text-foreground">
              <Server className="h-4 w-4 text-muted-foreground" />
              Game Server URL (for hot reload)
            </label>
            <input
              type="text"
              value={serverUrl}
              onChange={(e) => setServerUrl(e.target.value)}
              className="w-full rounded border border-border bg-input px-3 py-2 font-mono text-sm text-foreground outline-none focus:border-primary"
              placeholder="http://localhost:8443"
            />
          </div>

          {error && (
            <div className="rounded border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {error}
            </div>
          )}

          <button
            onClick={() => onConnect(connectionString, serverUrl)}
            disabled={connecting || !connectionString}
            className="flex w-full items-center justify-center gap-2 rounded bg-primary px-4 py-2.5 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-50"
          >
            {connecting ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin" />
                Connecting...
              </>
            ) : (
              'Connect'
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
