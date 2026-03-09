import { useCallback, useEffect, useState } from 'react';
import { Power, Square } from 'lucide-react';
import { Button } from './ui/button';
import { fetchAdminStatus, type AdminStatusResponse } from '../lib/admin-api';

type ServerHealth = 'offline' | 'degraded' | 'healthy';

const LED_STYLES: Record<ServerHealth, string> = {
  offline: 'bg-red-400 shadow-[0_0_12px_rgba(248,113,113,0.8)]',
  degraded: 'bg-amber-400 shadow-[0_0_12px_rgba(251,191,36,0.8)]',
  healthy: 'bg-emerald-400 shadow-[0_0_12px_rgba(52,211,153,0.8)]',
};

const STATUS_LABEL: Record<ServerHealth, string> = {
  offline: 'Offline',
  degraded: 'Degraded',
  healthy: 'Online',
};

function deriveHealth(status: AdminStatusResponse | null): ServerHealth {
  if (!status) return 'offline';
  const s = status.services;
  if (s.auth && s.base && s.cell && s.database) return 'healthy';
  return 'degraded';
}

async function postAction(path: string): Promise<{ status: string; action: string; message: string }> {
  const res = await fetch(path, { method: 'POST' });
  return res.json();
}

/**
 * Persistent server status LED + Start/Stop/HotReload controls.
 * Shown in the AppShell header on every page.
 */
export default function ServerControls() {
  const [status, setStatus] = useState<AdminStatusResponse | null>(null);
  const [health, setHealth] = useState<ServerHealth>('offline');
  const [busy, setBusy] = useState(false);

  const poll = useCallback(async () => {
    try {
      const data = await fetchAdminStatus();
      setStatus(data);
      setHealth(deriveHealth(data));
    } catch {
      setStatus(null);
      setHealth('offline');
    }
  }, []);

  // Poll every 5 seconds
  useEffect(() => {
    poll();
    const id = setInterval(poll, 5000);
    return () => clearInterval(id);
  }, [poll]);

  const handleStart = async () => {
    setBusy(true);
    try {
      await postAction('/api/config/start');
      // Poll immediately to update status
      await poll();
    } finally {
      setBusy(false);
    }
  };

  const handleStop = async () => {
    setBusy(true);
    try {
      await postAction('/api/config/stop');
      await poll();
    } finally {
      setBusy(false);
    }
  };

  // TODO: Wire up hot reload logic (return to this when logic is supplied)
  // The specific reload command will be provided by the lead dev.
  const handleHotReload = () => {};

  const servicesRunning = status?.services.auth || status?.services.base || status?.services.cell;

  return (
    <div className="flex flex-wrap items-center gap-3">
      {/* Status LED */}
      <div className="flex items-center gap-2.5 rounded-full border border-white/8 bg-white/5 px-4 py-2.5">
        <div className={`size-2.5 rounded-full transition-all ${LED_STYLES[health]}`} />
        <span className="text-sm font-medium text-foreground">{STATUS_LABEL[health]}</span>
      </div>

      {/* Start / Stop */}
      <Button
        onClick={handleStart}
        variant="outline"
        size="sm"
        disabled={busy || (health !== 'offline' && !!servicesRunning)}
        title="Start server services"
      >
        <Power className="size-3.5" />
        {busy ? 'Starting...' : 'Start'}
      </Button>
      <Button
        onClick={handleStop}
        variant="outline"
        size="sm"
        disabled={busy || health === 'offline' || !servicesRunning}
        title="Stop server services"
      >
        <Square className="size-3.5" />
        {busy ? 'Stopping...' : 'Stop'}
      </Button>

      {/* Hot Reload */}
      <Button
        onClick={handleHotReload}
        variant="secondary"
        size="sm"
        disabled={health === 'offline'}
        title="Hot reload server"
      >
        🔥 Hot Reload
      </Button>
    </div>
  );
}
