import { useState, useEffect, useCallback } from 'react';
import { X, Database, Loader2 } from 'lucide-react';
import type { DbStatus } from '../lib/types';

interface DbConnectionDialogProps {
  status: DbStatus | null;
  onConnect: (host: string, port: number, dbname: string, username: string, password: string) => void;
  onDisconnect: () => void;
  onClose: () => void;
}

const LS_KEY = 'scene-editor-db-conn';

interface SavedFields {
  host: string;
  port: string;
  dbname: string;
  username: string;
}

function loadSaved(): SavedFields {
  try {
    const raw = localStorage.getItem(LS_KEY);
    if (raw) return JSON.parse(raw);
  } catch { /* ignore */ }
  return { host: 'localhost', port: '5432', dbname: 'sgw', username: 'w-testing' };
}

function saveLast(fields: SavedFields) {
  try {
    localStorage.setItem(LS_KEY, JSON.stringify(fields));
  } catch { /* ignore */ }
}

export function DbConnectionDialog({ status, onConnect, onDisconnect, onClose }: DbConnectionDialogProps) {
  const saved = loadSaved();
  const [host, setHost] = useState(saved.host);
  const [port, setPort] = useState(saved.port);
  const [dbname, setDbname] = useState(saved.dbname);
  const [username, setUsername] = useState(saved.username);
  const [password, setPassword] = useState('');
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Close on Escape
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [onClose]);

  const handleConnect = useCallback(async () => {
    const portNum = parseInt(port, 10);
    if (isNaN(portNum) || portNum < 1 || portNum > 65535) {
      setError('Invalid port number');
      return;
    }
    setError(null);
    setConnecting(true);
    try {
      saveLast({ host, port, dbname, username });
      await onConnect(host, portNum, dbname, username, password);
    } catch (e) {
      setError(String(e));
    } finally {
      setConnecting(false);
    }
  }, [host, port, dbname, username, password, onConnect]);

  const isConnected = status?.connected ?? false;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="w-[440px] rounded-lg border bg-card shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between border-b px-5 py-3">
          <div className="flex items-center gap-2">
            <Database size={16} className="text-accent" />
            <h2 className="text-sm font-semibold text-foreground">Database Connection</h2>
          </div>
          <button
            className="rounded p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground"
            onClick={onClose}
          >
            <X size={16} />
          </button>
        </div>

        {/* Connected status */}
        {isConnected && status && (
          <div className="border-b px-5 py-3">
            <div className="flex items-center gap-2">
              <span className="h-2 w-2 rounded-full bg-green-500" />
              <span className="text-xs text-foreground">
                Connected to <span className="font-mono font-medium">{status.dbname}</span> on{' '}
                <span className="font-mono">{status.host}</span>
              </span>
            </div>
            <button
              className="mt-2 rounded bg-destructive/10 px-3 py-1 text-xs font-medium text-destructive hover:bg-destructive/20"
              onClick={() => {
                onDisconnect();
                setError(null);
              }}
            >
              Disconnect
            </button>
          </div>
        )}

        {/* Connection form */}
        {!isConnected && (
          <div className="space-y-3 px-5 py-4">
            <div className="flex gap-3">
              <div className="flex-1">
                <label className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                  Host
                </label>
                <input
                  type="text"
                  className="w-full rounded border bg-input px-2.5 py-1.5 font-mono text-xs text-foreground outline-none focus:border-primary"
                  value={host}
                  onChange={e => setHost(e.target.value)}
                  placeholder="localhost"
                />
              </div>
              <div className="w-20">
                <label className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                  Port
                </label>
                <input
                  type="text"
                  className="w-full rounded border bg-input px-2.5 py-1.5 font-mono text-xs text-foreground outline-none focus:border-primary"
                  value={port}
                  onChange={e => setPort(e.target.value)}
                  placeholder="5432"
                />
              </div>
            </div>

            <div>
              <label className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                Database
              </label>
              <input
                type="text"
                className="w-full rounded border bg-input px-2.5 py-1.5 font-mono text-xs text-foreground outline-none focus:border-primary"
                value={dbname}
                onChange={e => setDbname(e.target.value)}
                placeholder="sgw"
              />
            </div>

            <div className="flex gap-3">
              <div className="flex-1">
                <label className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                  Username
                </label>
                <input
                  type="text"
                  className="w-full rounded border bg-input px-2.5 py-1.5 font-mono text-xs text-foreground outline-none focus:border-primary"
                  value={username}
                  onChange={e => setUsername(e.target.value)}
                  placeholder="w-testing"
                />
              </div>
              <div className="flex-1">
                <label className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                  Password
                </label>
                <input
                  type="password"
                  className="w-full rounded border bg-input px-2.5 py-1.5 font-mono text-xs text-foreground outline-none focus:border-primary"
                  value={password}
                  onChange={e => setPassword(e.target.value)}
                  onKeyDown={e => { if (e.key === 'Enter') handleConnect(); }}
                  placeholder=""
                />
              </div>
            </div>

            {/* Error display */}
            {error && (
              <div className="rounded border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive">
                {error}
              </div>
            )}

            <button
              className="flex w-full items-center justify-center gap-2 rounded bg-primary px-4 py-2 text-xs font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
              onClick={handleConnect}
              disabled={connecting || !host.trim() || !dbname.trim() || !username.trim()}
            >
              {connecting ? (
                <>
                  <Loader2 size={14} className="animate-spin" />
                  Connecting...
                </>
              ) : (
                'Connect'
              )}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
