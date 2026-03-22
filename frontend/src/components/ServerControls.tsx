import { useCallback, useEffect, useState } from 'react';
import { Hammer, Power, Square, AlertTriangle } from 'lucide-react';
import { Button } from './ui/button';
import {
  fetchAdminStatus,
  fetchSupervisorStatus,
  supervisorStart,
  supervisorStop,
  supervisorRebuild,
  type SupervisorStatusResponse,
} from '../lib/admin-api';

type ServerHealth = 'offline' | 'building' | 'degraded' | 'healthy';

const LED_STYLES: Record<ServerHealth, string> = {
  offline: 'bg-red-400 shadow-[0_0_12px_rgba(248,113,113,0.8)]',
  building: 'bg-primary shadow-[0_0_12px_rgba(242,202,80,0.8)]',
  degraded: 'bg-primary shadow-[0_0_12px_rgba(242,202,80,0.8)]',
  healthy: 'bg-[#7fdedd] shadow-[0_0_12px_rgba(127,222,221,0.8)]',
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
    if (s.auth && s.base && s.cell) return 'healthy';
    return 'degraded';
  }
  return 'degraded';
}

/**
 * Persistent server status LED + Start/Stop/Rebuild controls.
 * Polls the supervisor (port 8444) and admin API (port 8443).
 */
export default function ServerControls() {
  const [sv, setSv] = useState<SupervisorStatusResponse | null>(null);
  const [health, setHealth] = useState<ServerHealth>('offline');
  const [serverReachable, setServerReachable] = useState(false);
  const [busy, setBusy] = useState(false);

  const poll = useCallback(async () => {
    // Poll supervisor
    try {
      const data = await fetchSupervisorStatus();
      setSv(data);
      setHealth(deriveHealth(data));
    } catch {
      setSv(null);
      setHealth('offline');
    }

    // Poll admin API (game server)
    try {
      await fetchAdminStatus();
      setServerReachable(true);
    } catch {
      setServerReachable(false);
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

  const supervisorReachable = sv !== null;
  const isBuilding = sv?.building ?? false;
  const processRunning = sv?.process_running ?? false;

  // Derive status banner
  const statusBanner = (() => {
    if (serverReachable && supervisorReachable) return null; // all good
    if (!serverReachable && !supervisorReachable)
      return { text: 'Server offline — Supervisor unreachable', severity: 'critical' as const };
    if (!serverReachable && supervisorReachable)
      return { text: 'Server offline', severity: 'warning' as const };
    // serverReachable && !supervisorReachable
    return { text: 'Server online — Supervisor unreachable', severity: 'warning' as const };
  })();

  return (
    <div className="flex flex-col items-end gap-2">
      <div className="flex flex-wrap items-center gap-3">
        {/* Status LED */}
        <div className="flex items-center gap-2.5 border border-[rgba(77,70,53,0.15)] bg-surface-high px-4 py-2.5">
          <div className={`size-2.5 rounded-full transition-all ${LED_STYLES[health]}`} />
          <span className="text-sm font-medium text-on-surface">{STATUS_LABEL[health]}</span>
        </div>

        {/* Start */}
        <Button
          onClick={handleStart}
          variant="outline"
          size="sm"
          disabled={busy || isBuilding || processRunning || !supervisorReachable}
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
          disabled={busy || isBuilding || !processRunning || !supervisorReachable}
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
          disabled={busy || isBuilding || !supervisorReachable}
          title="Stop, rebuild, and restart server"
        >
          <Hammer className="size-3.5" />
          Rebuild
        </Button>
      </div>

      {/* Status banner */}
      {statusBanner && (
        <div
          className={`flex items-center gap-2 px-3 py-1.5 font-dense text-xs uppercase tracking-widest ${
            statusBanner.severity === 'critical'
              ? 'border border-error/20 bg-error/8 text-error'
              : 'border border-primary/20 bg-primary/8 text-primary'
          }`}
        >
          <AlertTriangle className="size-3" />
          {statusBanner.text}
        </div>
      )}
    </div>
  );
}
