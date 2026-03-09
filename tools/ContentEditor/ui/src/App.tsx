import { useState, useCallback } from 'react';
import { AppLayout } from './components/AppLayout';
import { ConnectDialog } from './components/ConnectDialog';
import { invoke } from './lib/tauri';

interface ConnectionStatus {
  connected: boolean;
  database: string | null;
}

export default function App() {
  const [connected, setConnected] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleConnect = useCallback(async (connectionString: string, serverUrl: string) => {
    setConnecting(true);
    setError(null);
    try {
      const status = await invoke<ConnectionStatus>('connect_db', {
        connectionString,
        serverUrl: serverUrl || undefined,
      });
      setConnected(status.connected);
    } catch (e) {
      setError(String(e));
    } finally {
      setConnecting(false);
    }
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

  return <AppLayout />;
}
