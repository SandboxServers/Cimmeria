import { For, Show, createMemo, createResource } from 'solid-js';
import { ArrowUpRight, Gauge, RefreshCw, ShieldAlert, Sparkles } from 'lucide-solid';
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
import {
  buildDashboardActivity,
  buildDashboardStats,
  buildServiceHealth,
  fetchAdminStatus,
  fetchContentSummary,
  fetchPlayers,
  fetchSpaces,
} from '../lib/admin-api';

function LoadingCard(props: { label: string }) {
  return (
    <Card class="overflow-hidden">
      <CardHeader class="pb-4">
        <div class="flex items-center justify-between">
          <Badge variant="outline">{props.label}</Badge>
          <Sparkles class="size-4 text-primary/80" />
        </div>
        <div class="h-10 w-28 animate-pulse rounded-2xl bg-white/8" />
        <div class="h-4 w-3/4 animate-pulse rounded-full bg-white/6" />
      </CardHeader>
      <CardContent class="pt-0">
        <div class="h-8 w-24 animate-pulse rounded-full bg-white/6" />
      </CardContent>
    </Card>
  );
}

function ErrorCard(props: { title: string; detail: string }) {
  return (
    <Card class="border-destructive/22 bg-destructive/8">
      <CardHeader>
        <Badge variant="destructive">Load Failed</Badge>
        <CardTitle>{props.title}</CardTitle>
        <CardDescription>{props.detail}</CardDescription>
      </CardHeader>
    </Card>
  );
}

export default function Dashboard() {
  const [status, { refetch: refetchStatus }] = createResource(fetchAdminStatus);
  const [players, { refetch: refetchPlayers }] = createResource(fetchPlayers);
  const [spaces, { refetch: refetchSpaces }] = createResource(fetchSpaces);
  const [content, { refetch: refetchContent }] = createResource(fetchContentSummary);

  const stats = createMemo(() =>
    status() && players() && spaces() && content()
      ? buildDashboardStats(status()!, players()!, spaces()!, content()!)
      : [],
  );
  const serviceHealth = createMemo(() =>
    status() ? buildServiceHealth(status()!) : [],
  );
  const liveActivity = createMemo(() =>
    status() && spaces() && content()
      ? buildDashboardActivity(status()!, spaces()!, content()!)
      : [],
  );
  const releaseChecklist = createMemo(() => {
    if (!status() || !players() || !spaces() || !content()) {
      return [];
    }

    return [
      { label: 'Service mesh reachable', complete: status()!.services.auth && status()!.services.base && status()!.services.cell },
      { label: 'Database-backed spaces loaded', complete: spaces()!.available },
      { label: 'Mission summary loaded', complete: content()!.available },
      { label: 'Live player roster ready', complete: players()!.available },
    ];
  });
  const isLoading = createMemo(
    () => status.loading || players.loading || spaces.loading || content.loading,
  );
  const loadError = createMemo(
    () =>
      status.error ??
      players.error ??
      spaces.error ??
      content.error ??
      null,
  );

  const refreshAll = () => {
    void refetchStatus();
    void refetchPlayers();
    void refetchSpaces();
    void refetchContent();
  };

  return (
    <div class="space-y-6">
      <PageHeader
        actions={
          <>
            <Button onClick={refreshAll} variant="secondary">
              <RefreshCw class="size-4" />
              Refresh telemetry
            </Button>
            <Button disabled variant="outline">
              <ArrowUpRight class="size-4" />
              Open live shard
            </Button>
          </>
        }
        badge="Realtime Monitoring"
        description="This dashboard now loads from the live admin API where the backend has real data, and degrades explicitly when a data source is not implemented yet."
        eyebrow="Operations"
        title="Command deck overview"
      />

      <Show when={loadError()} keyed>
        {(error) => (
          <ErrorCard
            detail={error.message}
            title="One or more dashboard data sources failed."
          />
        )}
      </Show>

      <section class="grid gap-4 xl:grid-cols-4">
        <Show
          when={stats().length > 0}
          fallback={
            <>
              <LoadingCard label="Uptime" />
              <LoadingCard label="Players Online" />
              <LoadingCard label="Known Spaces" />
              <LoadingCard label="Mission Records" />
            </>
          }
        >
          <For each={stats()}>
            {(stat) => (
              <Card class="overflow-hidden">
                <CardHeader class="pb-4">
                  <div class="flex items-center justify-between">
                    <Badge variant="outline">{stat.label}</Badge>
                    <Sparkles class="size-4 text-primary/80" />
                  </div>
                  <CardTitle class="text-4xl tracking-[-0.08em]">{stat.value}</CardTitle>
                  <CardDescription>{stat.detail}</CardDescription>
                </CardHeader>
                <CardContent class="pt-0">
                  <Badge variant="secondary">Live</Badge>
                </CardContent>
              </Card>
            )}
          </For>
        </Show>
      </section>

      <section class="grid gap-4 xl:grid-cols-[1.35fr_0.95fr]">
        <Card>
          <CardHeader class="flex-row items-start justify-between gap-3">
            <div class="space-y-2">
              <Badge variant="secondary">Service Mesh</Badge>
              <CardTitle>Runtime health envelope</CardTitle>
              <CardDescription>
                Directly backed by `/api/config/status`.
              </CardDescription>
            </div>
            <div class="rounded-2xl border border-white/8 bg-white/5 p-3 text-accent">
              <Gauge class="size-5" />
            </div>
          </CardHeader>
          <CardContent class="grid gap-4 md:grid-cols-2">
            <Show
              when={serviceHealth().length > 0}
              fallback={<div class="text-sm text-muted-foreground">{isLoading() ? 'Loading service status...' : 'Service status unavailable.'}</div>}
            >
              <For each={serviceHealth()}>
                {(service) => (
                  <div class="rounded-[24px] border border-white/6 bg-white/[0.03] p-4">
                    <div class="flex items-start justify-between gap-3">
                      <div>
                        <p class="text-base font-medium text-foreground">{service.name}</p>
                        <p class="mt-1 text-sm text-muted-foreground">{service.detail}</p>
                      </div>
                      <Badge variant={service.variant}>{service.status}</Badge>
                    </div>
                  </div>
                )}
              </For>
            </Show>
          </CardContent>
        </Card>

        <Card>
          <CardHeader class="space-y-3">
            <Badge variant="outline">Delivery Gates</Badge>
            <CardTitle>Backend-backed readiness</CardTitle>
            <CardDescription>
              These checks now reflect actual route availability, not a mock checklist.
            </CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <Show
              when={releaseChecklist().length > 0}
              fallback={<div class="text-sm text-muted-foreground">{isLoading() ? 'Loading readiness checks...' : 'Readiness data unavailable.'}</div>}
            >
              <For each={releaseChecklist()}>
                {(item, index) => (
                  <>
                    <div class="flex items-center justify-between gap-3">
                      <div>
                        <p class="text-sm font-medium text-foreground">{item.label}</p>
                        <p class="mt-1 text-xs uppercase tracking-[0.2em] text-muted-foreground">
                          {item.complete ? 'Ready' : 'Degraded'}
                        </p>
                      </div>
                      <div
                        class={`size-3 rounded-full ${
                          item.complete
                            ? 'bg-emerald-300 shadow-[0_0_20px_rgba(110,231,183,0.8)]'
                            : 'bg-amber-300 shadow-[0_0_18px_rgba(252,211,77,0.65)]'
                        }`}
                      />
                    </div>
                    <Show when={index() < releaseChecklist().length - 1}>
                      <Separator />
                    </Show>
                  </>
                )}
              </For>
            </Show>
            <div class="rounded-[24px] border border-primary/16 bg-primary/8 p-4">
              <div class="flex items-start gap-3">
                <ShieldAlert class="mt-0.5 size-4 text-primary" />
                <div>
                  <p class="text-sm font-medium text-foreground">What remains mocked</p>
                  <p class="mt-1 text-sm leading-6 text-muted-foreground">
                    Live players and websocket logs still surface explicit unavailable states because the underlying server feeds are not implemented yet.
                  </p>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      </section>

      <section class="grid gap-4 xl:grid-cols-[1.1fr_0.9fr]">
        <Card>
          <CardHeader class="space-y-3">
            <Badge variant="secondary">Live Feed</Badge>
            <CardTitle>Operational timeline</CardTitle>
            <CardDescription>
              Derived from the current service, space, and mission summaries.
            </CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <Show
              when={liveActivity().length > 0}
              fallback={<div class="text-sm text-muted-foreground">{isLoading() ? 'Loading activity...' : 'No live activity available.'}</div>}
            >
              <For each={liveActivity()}>
                {(event, index) => (
                  <div class="flex gap-4">
                    <div class="flex flex-col items-center">
                      <div class="rounded-full border border-accent/35 bg-accent/10 px-3 py-1 text-xs font-semibold tracking-[0.22em] text-accent">
                        {event.time}
                      </div>
                      <Show when={index() < liveActivity().length - 1}>
                        <div class="mt-2 h-full w-px bg-border/70" />
                      </Show>
                    </div>
                    <div class="pb-5">
                      <p class="text-sm font-medium text-foreground">{event.title}</p>
                      <p class="mt-1 max-w-2xl text-sm leading-6 text-muted-foreground">
                        {event.detail}
                      </p>
                    </div>
                  </div>
                )}
              </For>
            </Show>
          </CardContent>
        </Card>

        <Card class="overflow-hidden">
          <CardHeader class="space-y-3">
            <Badge variant="outline">Player Pulse</Badge>
            <CardTitle>Roster sample</CardTitle>
            <CardDescription>
              This uses the live player route when available and otherwise states clearly that the roster feed has not landed yet.
            </CardDescription>
          </CardHeader>
          <CardContent class="space-y-3">
            <Show
              when={players()?.players.length}
              fallback={
                <div class="rounded-[24px] border border-white/8 bg-white/[0.03] px-4 py-4 text-sm text-muted-foreground">
                  {players()?.reason ?? 'Live player roster is not available yet.'}
                </div>
              }
            >
              <For each={players()?.players ?? []}>
                {(player) => (
                  <div class="flex items-center justify-between gap-3 rounded-[24px] border border-white/6 bg-white/[0.03] px-4 py-3">
                    <div class="min-w-0">
                      <p class="truncate text-sm font-medium text-foreground">{player.name}</p>
                      <p class="truncate text-sm text-muted-foreground">
                        {player.archetype} • {player.zone}
                      </p>
                    </div>
                    <div class="text-right">
                      <p class="text-sm font-medium text-primary">{player.status}</p>
                      <p class="text-xs uppercase tracking-[0.18em] text-muted-foreground">
                        {player.ping ?? 0}ms
                      </p>
                    </div>
                  </div>
                )}
              </For>
            </Show>
          </CardContent>
        </Card>
      </section>
    </div>
  );
}
