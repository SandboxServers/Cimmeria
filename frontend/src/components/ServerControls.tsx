import { useCallback, useEffect, useState } from 'react';
import { Hammer, Power, Square } from 'lucide-react';
import { Button } from './ui/button';
import {
  fetchSupervisorStatus,
  supervisorStart,
  supervisorStop,
  supervisorRebuild,
  type SupervisorStatusResponse,
} from '../lib/admin-api';

type ServerHealth = 'offline' | 'building' | 'degraded' | 'healthy';

const LED_STYLES: Record<ServerHealth, string> = {
  offline: 'bg-red-400 shadow-[0_0_12px_rgba(248,113,113,0.8)]',
  building: 'bg-amber-400 shadow-[0_0_12px_rgba(251,191,36,0.8)]',
  degraded: 'bg-amber-400 shadow-[0_0_12px_rgba(251,191,36,0.8)]',
  healthy: 'bg-emerald-400 shadow-[0_0_12px_rgba(52,211,153,0.8)]',
};

const STATUS_LABEL: Record<ServerHealth, string> = {
  offline: 'Offline',
  building: 'Building...',
  degraded: 'Degraded',
  healthy: 'Online',
};

function deriveHealth(sv: SupervisorStatusResponse | null): ServerHealth {
  if (!sv) return 'offline';
  if (sv.building) return 'building';
  if (!sv.process_running) return 'offline';
  if (sv.services) {
    const s = sv.services;
    if (s.auth && s.base && s.cell && s.database) return 'healthy';
    return 'degraded';
  }
  // Process running but services not yet reported (startup)
  return 'degraded';
}

/**
 * Persistent server status LED + Start/Stop/Rebuild controls.
 * Polls the supervisor (port 8444) which is always available,
 * even when the game server is down.
 */
export default function ServerControls() {
  const [sv, setSv] = useState<SupervisorStatusResponse | null>(null);
  const [health, setHealth] = useState<ServerHealth>('offline');
  const [busy, setBusy] = useState(false);

  const poll = useCallback(async () => {
    try {
      const data = await fetchSupervisorStatus();
      setSv(data);
      setHealth(deriveHealth(data));
    } catch {
      setSv(null);
      setHealth('offline');
    }
  }, []);

  useEffect(() => {
    poll();
    const id = setInterval(poll, 5000);
    return () => clearInterval(id);
  }, [poll]);

  const handleStart = async () => {
    setBusy(true);
    try {
      await supervisorStart();
      await poll();
    } finally {
      setBusy(false);
    }
  };

  const handleStop = async () => {
    setBusy(true);
    try {
      await supervisorStop();
      await poll();
    } finally {
      setBusy(false);
    }
  };

  const handleRebuild = async () => {
    setBusy(true);
    try {
      await supervisorRebuild();
    } finally {
      setBusy(false);
      await poll();
    }
  };

  const isBuilding = sv?.building ?? false;
  const processRunning = sv?.process_running ?? false;

  return (
    <div className="flex flex-wrap items-center gap-3">
      {/* Status LED */}
      <div className="flex items-center gap-2.5 rounded-full border border-white/8 bg-white/5 px-4 py-2.5">
        <div className={`size-2.5 rounded-full transition-all ${LED_STYLES[health]}`} />
        <span className="text-sm font-medium text-foreground">{STATUS_LABEL[health]}</span>
      </div>

      {/* Start */}
      <Button
        onClick={handleStart}
        variant="outline"
        size="sm"
        disabled={busy || isBuilding || processRunning}
        title="Start server process"
      >
        <Power className="size-3.5" />
        Start
      </Button>

      {/* Stop */}
      <Button
        onClick={handleStop}
        variant="outline"
        size="sm"
        disabled={busy || isBuilding || !processRunning}
        title="Stop server process"
      >
        <Square className="size-3.5" />
        Stop
      </Button>

      {/* Rebuild */}
      <Button
        onClick={handleRebuild}
        variant="secondary"
        size="sm"
        disabled={busy || isBuilding}
        title="Stop, rebuild, and restart server"
      >
        <Hammer className="size-3.5" />
        Rebuild
      </Button>
    </div>
  );
}
