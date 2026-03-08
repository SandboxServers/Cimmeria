import { useCallback, useMemo, useState } from 'react';
import { Bug, RefreshCw, ShieldAlert, Waves } from 'lucide-react';
import PageHeader from '../components/PageHeader';
import { Badge } from '../components/ui/badge';
import { Button } from '../components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '../components/ui/card';
import { Input } from '../components/ui/input';
import { Tabs, TabsList, TabsTrigger } from '../components/ui/tabs';
import { useResource } from '../lib/hooks';
import { fetchAdminStatus, fetchPlayers, formatUptime } from '../lib/admin-api';

export default function Logs() {
  const [level, setLevel] = useState('ALL');
  const [query, setQuery] = useState('');
  const status = useResource(useCallback(fetchAdminStatus, []));
  const players = useResource(useCallback(fetchPlayers, []));

  const entries = useMemo(() => {
    if (!status.data) {
      return [];
    }

    return [
      {
        level: status.data.services.database ? 'INFO' : 'WARN',
        timestamp: 'now',
        source: 'admin-api',
        message: `Orchestrator uptime is ${formatUptime(status.data.uptime_seconds)}.`,
      },
      {
        level: status.data.services.database ? 'INFO' : 'ERROR',
        timestamp: 'now',
        source: 'database',
        message: status.data.services.database
          ? 'PostgreSQL is reachable for space and mission-backed surfaces.'
          : 'Database is unavailable; data-backed pages are degraded.',
      },
      {
        level: players.data?.available ? 'INFO' : 'WARN',
        timestamp: 'now',
        source: 'players',
        message: players.data?.available
          ? 'Live player roster is available.'
          : players.data?.reason ?? 'Player roster route has not reported yet.',
      },
      {
        level: 'WARN' as const,
        timestamp: 'now',
        source: 'ws/logs',
        message: 'Log streaming transport is not implemented yet; this page currently exposes backend readiness instead of a live tail.',
      },
    ];
  }, [status.data, players.data]);

  const filteredEntries = useMemo(() =>
    entries.filter((entry) => {
      const matchesLevel = level === 'ALL' || entry.level === level;
      const haystack = `${entry.source} ${entry.message}`.toLowerCase();
      return matchesLevel && haystack.includes(query.trim().toLowerCase());
    }),
  [entries, level, query]);

  const refreshAll = () => {
    status.refetch();
    players.refetch();
  };

  return (
    <div className="space-y-6">
      <PageHeader
        actions={
          <>
            <Button onClick={refreshAll} variant="secondary">
              <RefreshCw className="size-4" />
              Refresh snapshot
            </Button>
            <Button disabled variant="outline">
              <ShieldAlert className="size-4" />
              Create incident
            </Button>
          </>
        }
        badge="Transport Readiness"
        description="The old fake log tail is gone. This page now shows live backend-backed readiness and explicitly calls out that `/ws/logs` is not implemented yet."
        eyebrow="Logs"
        title="Server event stream"
      />

      <section className="grid gap-4 xl:grid-cols-[1.2fr_0.8fr]">
        <Card>
          <CardHeader className="space-y-4">
            <div className="space-y-2">
              <Badge variant="secondary">Tail View</Badge>
              <CardTitle>Transport and readiness snapshot</CardTitle>
              <CardDescription>
                Backed by `/api/config/status` and `/api/players`.
              </CardDescription>
            </div>
            <div className="flex flex-col gap-3">
              <Tabs defaultValue="ALL" onValueChange={setLevel}>
                <TabsList className="justify-start">
                  <TabsTrigger value="ALL">All</TabsTrigger>
                  <TabsTrigger value="INFO">Info</TabsTrigger>
                  <TabsTrigger value="WARN">Warn</TabsTrigger>
                  <TabsTrigger value="ERROR">Error</TabsTrigger>
                </TabsList>
              </Tabs>
              <div className="max-w-md">
                <Input
                  onChange={(event) => setQuery(event.currentTarget.value)}
                  placeholder="Search by source or message"
                  value={query}
                />
              </div>
            </div>
          </CardHeader>
          <CardContent>
            {(status.error || players.error) && (
              <div className="rounded-[28px] border border-destructive/20 bg-destructive/8 p-4 text-sm text-destructive">
                {(status.error ?? players.error)?.message}
              </div>
            )}
            <div className="subtle-scrollbar rounded-[28px] border border-white/8 bg-[#09131d] p-4 font-mono text-xs text-slate-100 shadow-inner shadow-black/35">
              <div className="mb-4 flex items-center gap-3">
                <div
                  className={`size-2 rounded-full ${
                    status.data?.services.database
                      ? 'bg-emerald-300 shadow-[0_0_14px_rgba(110,231,183,0.85)]'
                      : 'bg-amber-300 shadow-[0_0_14px_rgba(252,211,77,0.85)]'
                  }`}
                />
                <span className="uppercase tracking-[0.24em] text-slate-400">
                  {status.data ? 'Backend snapshot loaded' : 'Loading backend snapshot'}
                </span>
              </div>
              <div className="space-y-3">
                {filteredEntries.map((entry) => (
                  <div key={entry.source} className="rounded-[20px] border border-white/6 bg-white/[0.03] p-3">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="text-slate-500">{entry.timestamp}</span>
                      <span
                        className={`rounded-full px-2 py-1 text-[10px] font-semibold tracking-[0.22em] ${
                          entry.level === 'ERROR'
                            ? 'bg-red-400/15 text-red-200'
                            : entry.level === 'WARN'
                              ? 'bg-amber-300/15 text-amber-100'
                              : 'bg-emerald-400/12 text-emerald-100'
                        }`}
                      >
                        {entry.level}
                      </span>
                      <span className="text-slate-300">{entry.source}</span>
                    </div>
                    <p className="mt-2 leading-6 text-slate-100">{entry.message}</p>
                  </div>
                ))}
              </div>
            </div>
          </CardContent>
        </Card>

        <div className="space-y-4">
          <Card>
            <CardHeader className="space-y-3">
              <Badge variant="outline">Incidents</Badge>
              <CardTitle>Current focus</CardTitle>
              <CardDescription>
                Live warnings based on current backend readiness.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="rounded-[24px] border border-destructive/18 bg-destructive/8 p-4">
                <p className="text-sm font-medium text-foreground">Log stream unavailable</p>
                <p className="mt-2 text-sm leading-6 text-muted-foreground">
                  `/ws/logs` does not yet provide a live event stream, so this page cannot tail real server logs today.
                </p>
              </div>
              {status.data?.services.database === false && (
                <div className="rounded-[24px] border border-primary/18 bg-primary/8 p-4">
                  <p className="text-sm font-medium text-foreground">Database degraded</p>
                  <p className="mt-2 text-sm leading-6 text-muted-foreground">
                    Database-backed admin surfaces are running in degraded mode until PostgreSQL is reachable again.
                  </p>
                </div>
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="space-y-3">
              <Badge variant="secondary">Transport</Badge>
              <CardTitle>Stream roadmap</CardTitle>
              <CardDescription>
                Reserved space for the eventual websocket log implementation.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="flex items-center gap-3 rounded-[24px] border border-white/8 bg-white/[0.03] p-4">
                <Waves className="size-4 text-accent" />
                <p className="text-sm text-muted-foreground">
                  The client-side filtering controls are live; the transport is the remaining missing piece.
                </p>
              </div>
              <div className="flex items-center gap-3 rounded-[24px] border border-white/8 bg-white/[0.03] p-4">
                <Bug className="size-4 text-primary" />
                <p className="text-sm text-muted-foreground">
                  Once `/ws/logs` exists, this page can swap from readiness snapshots to a real tail without a layout rewrite.
                </p>
              </div>
            </CardContent>
          </Card>
        </div>
      </section>
    </div>
  );
}
