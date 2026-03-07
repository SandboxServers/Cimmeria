import { For } from 'solid-js';
import {
  ArrowUpRight,
  Gauge,
  RefreshCw,
  ShieldAlert,
  Sparkles,
  Waves,
} from 'lucide-solid';
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
import { Progress } from '../components/ui/progress';
import { Separator } from '../components/ui/separator';
import {
  dashboardStats,
  liveActivity,
  players,
  releaseChecklist,
  serviceHealth,
} from '../data/admin';

export default function Dashboard() {
  return (
    <div class="space-y-6">
      <PageHeader
        actions={
          <>
            <Button variant="secondary">
              <RefreshCw class="size-4" />
              Refresh telemetry
            </Button>
            <Button>
              <ArrowUpRight class="size-4" />
              Open live shard
            </Button>
          </>
        }
        badge="Realtime Monitoring"
        description="Phase 4 centers on a true operator surface: live shard health, player visibility, content shipping, and enough fidelity to spot trouble before it propagates into gameplay."
        eyebrow="Operations"
        title="Command deck overview"
      />

      <section class="grid gap-4 xl:grid-cols-4">
        <For each={dashboardStats}>
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
              <CardContent class="flex items-center justify-between pt-0">
                <span class="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">
                  Delta
                </span>
                <span class="rounded-full bg-primary/12 px-3 py-1 text-sm font-medium text-primary">
                  {stat.delta}
                </span>
              </CardContent>
            </Card>
          )}
        </For>
      </section>

      <section class="grid gap-4 xl:grid-cols-[1.35fr_0.95fr]">
        <Card>
          <CardHeader class="flex-row items-start justify-between gap-3">
            <div class="space-y-2">
              <Badge variant="secondary">Service Mesh</Badge>
              <CardTitle>Runtime health envelope</CardTitle>
              <CardDescription>
                Poll-ready cards for the auth, base, cell, and admin services.
              </CardDescription>
            </div>
            <div class="rounded-2xl border border-white/8 bg-white/5 p-3 text-accent">
              <Gauge class="size-5" />
            </div>
          </CardHeader>
          <CardContent class="grid gap-4 md:grid-cols-2">
            <For each={serviceHealth}>
              {(service) => (
                <div class="rounded-[24px] border border-white/6 bg-white/[0.03] p-4">
                  <div class="flex items-start justify-between gap-3">
                    <div>
                      <p class="text-base font-medium text-foreground">{service.name}</p>
                      <p class="mt-1 text-sm text-muted-foreground">{service.status}</p>
                    </div>
                    <Badge
                      variant={service.load > 75 ? 'destructive' : 'success'}
                    >
                      {service.latency}
                    </Badge>
                  </div>
                  <div class="mt-4 space-y-2">
                    <div class="flex items-center justify-between text-xs uppercase tracking-[0.22em] text-muted-foreground">
                      <span>Utilization</span>
                      <span>{service.load}%</span>
                    </div>
                    <Progress value={service.load} />
                  </div>
                </div>
              )}
            </For>
          </CardContent>
        </Card>

        <Card>
          <CardHeader class="space-y-3">
            <Badge variant="outline">Immediate Attention</Badge>
            <CardTitle>Release readiness</CardTitle>
            <CardDescription>
              Phase 4 scope translated into visible delivery gates.
            </CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <For each={releaseChecklist}>
              {(item, index) => (
                <>
                  <div class="flex items-center justify-between gap-3">
                    <div>
                      <p class="text-sm font-medium text-foreground">{item.label}</p>
                      <p class="mt-1 text-xs uppercase tracking-[0.2em] text-muted-foreground">
                        {item.complete ? 'Ready' : 'Pending'}
                      </p>
                    </div>
                    <div
                      class={`size-3 rounded-full ${
                        item.complete ? 'bg-emerald-300 shadow-[0_0_20px_rgba(110,231,183,0.8)]' : 'bg-amber-300 shadow-[0_0_18px_rgba(252,211,77,0.65)]'
                      }`}
                    />
                  </div>
                  <For each={index() < releaseChecklist.length - 1 ? [0] : []}>
                    {() => <Separator />}
                  </For>
                </>
              )}
            </For>
            <div class="rounded-[24px] border border-primary/16 bg-primary/8 p-4">
              <div class="flex items-start gap-3">
                <ShieldAlert class="mt-0.5 size-4 text-primary" />
                <div>
                  <p class="text-sm font-medium text-foreground">Outstanding polish</p>
                  <p class="mt-1 text-sm leading-6 text-muted-foreground">
                    Remote auth and transport hooks still need backend integration, but the frontend surface is now structured for them.
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
              Recent events aligned for logs, entity streams, and content publishes.
            </CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <For each={liveActivity}>
              {(event, index) => (
                <div class="flex gap-4">
                  <div class="flex flex-col items-center">
                    <div class="rounded-full border border-accent/35 bg-accent/10 px-3 py-1 text-xs font-semibold tracking-[0.22em] text-accent">
                      {event.time}
                    </div>
                    <For each={index() < liveActivity.length - 1 ? [0] : []}>
                      {() => <div class="mt-2 h-full w-px bg-border/70" />}
                    </For>
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
          </CardContent>
        </Card>

        <Card class="overflow-hidden">
          <CardHeader class="space-y-3">
            <Badge variant="outline">Player Pulse</Badge>
            <CardTitle>Active squad sample</CardTitle>
            <CardDescription>
              A compact view of what phase 4 monitoring can surface before opening the full player page.
            </CardDescription>
          </CardHeader>
          <CardContent class="space-y-3">
            <For each={players}>
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
                      {player.ping}ms
                    </p>
                  </div>
                </div>
              )}
            </For>
            <div class="rounded-[26px] border border-accent/20 bg-gradient-to-r from-accent/12 via-transparent to-primary/10 p-4">
              <div class="flex items-center gap-3">
                <Waves class="size-4 text-accent" />
                <p class="text-sm text-muted-foreground">
                  Entity and log websocket cards can hydrate these sections without layout changes.
                </p>
              </div>
            </div>
          </CardContent>
        </Card>
      </section>
    </div>
  );
}
