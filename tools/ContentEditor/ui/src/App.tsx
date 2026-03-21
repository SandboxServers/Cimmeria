import { useState, useCallback } from 'react';
import { AppLayout } from './components/AppLayout';
import { ConnectDialog } from './components/ConnectDialog';
import { invoke } from './lib/tauri';

interface ConnectionStatus {
  connected: boolean;
  database: string | null;
}

export interface ConnectionMeta {
  db: string;
  server: string;
}

export default function App() {
  const [connected, setConnected] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [connectionMeta, setConnectionMeta] = useState<ConnectionMeta | null>(null);

  const handleConnect = useCallback(async (connectionString: string, serverUrl: string) => {
    setConnecting(true);
    setError(null);
    try {
      const status = await invoke<ConnectionStatus>('connect_db', {
        connectionString,
        serverUrl: serverUrl || undefined,
      });
      if (status.connected) {
        // Extract db name from connection string (e.g. postgresql://user:pass@host:port/dbname)
        const dbMatch = connectionString.match(/\/([^/?]+)(?:\?|$)/);
        const dbName = dbMatch?.[1] ?? 'unknown';
        const hostMatch = connectionString.match(/@([^/]+)/);
        const host = hostMatch?.[1] ?? 'localhost';
        setConnectionMeta({ db: `${dbName}@${host}`, server: serverUrl });
      }
      setConnected(status.connected);
    } catch (e) {
      setError(String(e));
    } finally {
      setConnecting(false);
    }
  }, []);

  const handleDisconnect = useCallback(() => {
    setConnected(false);
    setConnectionMeta(null);
  }, []);

  if (!connected) {
    return (
      <ConnectDialog
        onConnect={handleConnect}
        connecting={connecting}
        error={error}
      />
    );
  }

  return (
    <AppLayout
      connectionMeta={connectionMeta}
      onDisconnect={handleDisconnect}
    />
  );
}
