import { useCallback, useMemo } from 'react';
import { ArrowUpRight, Gauge, RefreshCw, ShieldAlert, Sparkles } from 'lucide-react';
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
import { Separator } from '../components/ui/separator';
import { useResource } from '../lib/hooks';
import ConnectionStatus, { getConnectionError } from '../components/ConnectionStatus';
import {
  buildDashboardActivity,
  buildDashboardStats,
  buildServiceHealth,
  fetchAdminStatus,
  fetchContentSummary,
  fetchPlayers,
  fetchSpaces,
} from '../lib/admin-api';

function LoadingCard({ label }: { label: string }) {
  return (
    <Card className="overflow-hidden">
      <CardHeader className="pb-4">
        <div className="flex items-center justify-between">
          <Badge variant="outline">{label}</Badge>
          <Sparkles className="size-4 text-primary/80" />
        </div>
        <div className="h-10 w-28 animate-pulse rounded-2xl bg-white/8" />
        <div className="h-4 w-3/4 animate-pulse rounded-full bg-white/6" />
      </CardHeader>
      <CardContent className="pt-0">
        <div className="h-8 w-24 animate-pulse rounded-full bg-white/6" />
      </CardContent>
    </Card>
  );
}

function ErrorCard({ title, detail }: { title: string; detail: string }) {
  return (
    <Card className="border-destructive/22 bg-destructive/8">
      <CardHeader>
        <Badge variant="destructive">Load Failed</Badge>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{detail}</CardDescription>
      </CardHeader>
    </Card>
  );
}


export default function Dashboard() {
  const status = useResource(useCallback(fetchAdminStatus, []));
  const players = useResource(useCallback(fetchPlayers, []));
  const spaces = useResource(useCallback(fetchSpaces, []));
  const content = useResource(useCallback(fetchContentSummary, []));

  const stats = useMemo(
    () =>
      status.data && players.data && spaces.data && content.data
        ? buildDashboardStats(status.data, players.data, spaces.data, content.data)
        : [],
    [status.data, players.data, spaces.data, content.data],
  );
  const serviceHealth = useMemo(
    () => (status.data ? buildServiceHealth(status.data) : []),
    [status.data],
  );
  const liveActivity = useMemo(
    () =>
      status.data && spaces.data && content.data
        ? buildDashboardActivity(status.data, spaces.data, content.data)
        : [],
    [status.data, spaces.data, content.data],
  );
  const releaseChecklist = useMemo(() => {
    if (!status.data || !players.data || !spaces.data || !content.data) {
      return [];
    }

    return [
      { label: 'Service mesh reachable', complete: status.data.services.auth && status.data.services.base && status.data.services.cell },
      { label: 'Database-backed spaces loaded', complete: spaces.data.available },
      { label: 'Mission summary loaded', complete: content.data.available },
      { label: 'Live player roster ready', complete: players.data.available },
    ];
  }, [status.data, players.data, spaces.data, content.data]);
  const isLoading = status.loading || players.loading || spaces.loading || content.loading;
  const loadError = status.error ?? players.error ?? spaces.error ?? content.error ?? null;
  const connectionError = getConnectionError(status.error, players.error, spaces.error, content.error);

  const refreshAll = () => {
    status.refetch();
    players.refetch();
    spaces.refetch();
    content.refetch();
  };

  return (
    <div className="space-y-6">
      <PageHeader
        actions={
          <>
            <Button onClick={refreshAll} variant="secondary">
              <RefreshCw className="size-4" />
              Refresh telemetry
            </Button>
            <Button disabled variant="outline">
              <ArrowUpRight className="size-4" />
              Open live shard
            </Button>
          </>
        }
        badge="Realtime Monitoring"
        description="This dashboard now loads from the live admin API where the backend has real data, and degrades explicitly when a data source is not implemented yet."
        eyebrow="Operations"
        title="Command deck overview"
      />

      {connectionError ? (
        <ConnectionStatus onRetry={refreshAll} />
      ) : loadError ? (
        <ErrorCard
          detail={loadError.message}
          title="One or more dashboard data sources failed."
        />
      ) : null}

      <section className="grid gap-4 xl:grid-cols-4">
        {stats.length > 0 ? (
          stats.map((stat) => (
            <Card key={stat.label} className="overflow-hidden">
              <CardHeader className="pb-4">
                <div className="flex items-center justify-between">
                  <Badge variant="outline">{stat.label}</Badge>
                  <Sparkles className="size-4 text-primary/80" />
                </div>
                <CardTitle className="text-4xl tracking-[-0.08em]">{stat.value}</CardTitle>
                <CardDescription>{stat.detail}</CardDescription>
              </CardHeader>
              <CardContent className="pt-0">
                <Badge variant="secondary">Live</Badge>
              </CardContent>
            </Card>
          ))
        ) : (
          <>
            <LoadingCard label="Uptime" />
            <LoadingCard label="Players Online" />
            <LoadingCard label="Known Spaces" />
            <LoadingCard label="Mission Records" />
          </>
        )}
      </section>

      <section className="grid gap-4 xl:grid-cols-[1.35fr_0.95fr]">
        <Card>
          <CardHeader className="flex-row items-start justify-between gap-3">
            <div className="space-y-2">
              <Badge variant="secondary">Service Mesh</Badge>
              <CardTitle>Runtime health envelope</CardTitle>
              <CardDescription>
                Directly backed by `/api/config/status`.
              </CardDescription>
            </div>
            <div className="rounded-2xl border border-white/8 bg-white/5 p-3 text-accent">
              <Gauge className="size-5" />
            </div>
          </CardHeader>
          <CardContent className="grid gap-4 md:grid-cols-2">
            {serviceHealth.length > 0 ? (
              serviceHealth.map((service) => (
                <div key={service.name} className="rounded-[24px] border border-white/6 bg-white/[0.03] p-4">
                  <div className="flex items-start justify-between gap-3">
                    <div>
                      <p className="text-base font-medium text-foreground">{service.name}</p>
                      <p className="mt-1 text-sm text-muted-foreground">{service.detail}</p>
                    </div>
                    <Badge variant={service.variant}>{service.status}</Badge>
                  </div>
                </div>
              ))
            ) : (
              <div className="text-sm text-muted-foreground">{isLoading ? 'Loading service status...' : 'Service status unavailable.'}</div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="space-y-3">
            <Badge variant="outline">Delivery Gates</Badge>
            <CardTitle>Backend-backed readiness</CardTitle>
            <CardDescription>
              These checks now reflect actual route availability, not a mock checklist.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {releaseChecklist.length > 0 ? (
              releaseChecklist.map((item, index) => (
                <div key={item.label}>
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <p className="text-sm font-medium text-foreground">{item.label}</p>
                      <p className="mt-1 text-xs uppercase tracking-[0.2em] text-muted-foreground">
                        {item.complete ? 'Ready' : 'Degraded'}
                      </p>
                    </div>
                    <div
                      className={`size-3 rounded-full ${
                        item.complete
                          ? 'bg-emerald-300 shadow-[0_0_20px_rgba(110,231,183,0.8)]'
                          : 'bg-amber-300 shadow-[0_0_18px_rgba(252,211,77,0.65)]'
                      }`}
                    />
                  </div>
                  {index < releaseChecklist.length - 1 && <Separator />}
                </div>
              ))
            ) : (
              <div className="text-sm text-muted-foreground">{isLoading ? 'Loading readiness checks...' : 'Readiness data unavailable.'}</div>
            )}
            <div className="rounded-[24px] border border-primary/16 bg-primary/8 p-4">
              <div className="flex items-start gap-3">
                <ShieldAlert className="mt-0.5 size-4 text-primary" />
                <div>
                  <p className="text-sm font-medium text-foreground">What remains mocked</p>
                  <p className="mt-1 text-sm leading-6 text-muted-foreground">
                    Live players and websocket logs still surface explicit unavailable states because the underlying server feeds are not implemented yet.
                  </p>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      </section>

      <section className="grid gap-4 xl:grid-cols-[1.1fr_0.9fr]">
        <Card>
          <CardHeader className="space-y-3">
            <Badge variant="secondary">Live Feed</Badge>
            <CardTitle>Operational timeline</CardTitle>
            <CardDescription>
              Derived from the current service, space, and mission summaries.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {liveActivity.length > 0 ? (
              liveActivity.map((event, index) => (
                <div key={event.title} className="flex gap-4">
                  <div className="flex flex-col items-center">
                    <div className="rounded-full border border-accent/35 bg-accent/10 px-3 py-1 text-xs font-semibold tracking-[0.22em] text-accent">
                      {event.time}
                    </div>
                    {index < liveActivity.length - 1 && (
                      <div className="mt-2 h-full w-px bg-border/70" />
                    )}
                  </div>
                  <div className="pb-5">
                    <p className="text-sm font-medium text-foreground">{event.title}</p>
                    <p className="mt-1 max-w-2xl text-sm leading-6 text-muted-foreground">
                      {event.detail}
                    </p>
                  </div>
                </div>
              ))
            ) : (
              <div className="text-sm text-muted-foreground">{isLoading ? 'Loading activity...' : 'No live activity available.'}</div>
            )}
          </CardContent>
        </Card>

        <Card className="overflow-hidden">
          <CardHeader className="space-y-3">
            <Badge variant="outline">Player Pulse</Badge>
            <CardTitle>Roster sample</CardTitle>
            <CardDescription>
              This uses the live player route when available and otherwise states clearly that the roster feed has not landed yet.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {players.data?.players.length ? (
              players.data.players.map((player) => (
                <div key={player.name} className="flex items-center justify-between gap-3 rounded-[24px] border border-white/6 bg-white/[0.03] px-4 py-3">
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium text-foreground">{player.name}</p>
                    <p className="truncate text-sm text-muted-foreground">
                      {player.archetype} • {player.zone}
                    </p>
                  </div>
                  <div className="text-right">
                    <p className="text-sm font-medium text-primary">{player.status}</p>
                    <p className="text-xs uppercase tracking-[0.18em] text-muted-foreground">
                      {player.ping ?? 0}ms
                    </p>
                  </div>
                </div>
              ))
            ) : (
              <div className="rounded-[24px] border border-white/8 bg-white/[0.03] px-4 py-4 text-sm text-muted-foreground">
                {players.data?.reason ?? 'Live player roster is not available yet.'}
              </div>
            )}
          </CardContent>
        </Card>
      </section>
    </div>
  );
}
