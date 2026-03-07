import { For, createMemo, createSignal } from 'solid-js';
import { Bug, Search, ShieldAlert, Waves } from 'lucide-solid';
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
import { logEntries } from '../data/admin';

export default function Logs() {
  const [level, setLevel] = createSignal('ALL');
  const [query, setQuery] = createSignal('');

  const filteredEntries = createMemo(() =>
    logEntries.filter((entry) => {
      const matchesLevel = level() === 'ALL' || entry.level === level();
      const haystack = `${entry.source} ${entry.message}`.toLowerCase();
      const matchesQuery = haystack.includes(query().trim().toLowerCase());
      return matchesLevel && matchesQuery;
    }),
  );

  return (
    <div class="space-y-6">
      <PageHeader
        actions={
          <>
            <Button variant="secondary">
              <Bug class="size-4" />
              Export trace
            </Button>
            <Button>
              <ShieldAlert class="size-4" />
              Create incident
            </Button>
          </>
        }
        badge="Structured Logging"
        description="This page now supports the phase 4 logging story: clear filtering, live-tail styling, and reserved space for incident workflows."
        eyebrow="Logs"
        title="Server event stream"
      />

      <section class="grid gap-4 xl:grid-cols-[1.2fr_0.8fr]">
        <Card>
          <CardHeader class="space-y-4">
            <div class="space-y-2">
              <Badge variant="secondary">Tail View</Badge>
              <CardTitle>Stream filter controls</CardTitle>
              <CardDescription>
                Prepared for websocket-driven log hydration without changing the page structure.
              </CardDescription>
            </div>
            <div class="flex flex-col gap-3">
              <Tabs defaultValue="ALL" onValueChange={setLevel}>
                <TabsList class="justify-start">
                  <TabsTrigger value="ALL">All</TabsTrigger>
                  <TabsTrigger value="INFO">Info</TabsTrigger>
                  <TabsTrigger value="WARN">Warn</TabsTrigger>
                  <TabsTrigger value="ERROR">Error</TabsTrigger>
                  <TabsTrigger value="DEBUG">Debug</TabsTrigger>
                </TabsList>
              </Tabs>
              <div class="relative max-w-md">
                <Search class="pointer-events-none absolute left-4 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  class="pl-11"
                  onInput={(event) => setQuery(event.currentTarget.value)}
                  placeholder="Search by source or message"
                  value={query()}
                />
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <div class="subtle-scrollbar rounded-[28px] border border-white/8 bg-[#09131d] p-4 font-mono text-xs text-slate-100 shadow-inner shadow-black/35">
              <div class="mb-4 flex items-center gap-3">
                <div class="size-2 rounded-full bg-emerald-300 shadow-[0_0_14px_rgba(110,231,183,0.85)]" />
                <span class="uppercase tracking-[0.24em] text-slate-400">Live tail connected</span>
              </div>
              <div class="space-y-3">
                <For each={filteredEntries()}>
                  {(entry) => (
                    <div class="rounded-[20px] border border-white/6 bg-white/[0.03] p-3">
                      <div class="flex flex-wrap items-center gap-2">
                        <span class="text-slate-500">{entry.timestamp}</span>
                        <span
                          class={`rounded-full px-2 py-1 text-[10px] font-semibold tracking-[0.22em] ${
                            entry.level === 'ERROR'
                              ? 'bg-red-400/15 text-red-200'
                              : entry.level === 'WARN'
                                ? 'bg-amber-300/15 text-amber-100'
                                : entry.level === 'DEBUG'
                                  ? 'bg-sky-300/12 text-sky-100'
                                  : 'bg-emerald-400/12 text-emerald-100'
                          }`}
                        >
                          {entry.level}
                        </span>
                        <span class="text-slate-300">{entry.source}</span>
                      </div>
                      <p class="mt-2 leading-6 text-slate-100">{entry.message}</p>
                    </div>
                  )}
                </For>
              </div>
            </div>
          </CardContent>
        </Card>

        <div class="space-y-4">
          <Card>
            <CardHeader class="space-y-3">
              <Badge variant="outline">Incidents</Badge>
              <CardTitle>Current focus</CardTitle>
              <CardDescription>
                Summary panels for the issues worth escalation.
              </CardDescription>
            </CardHeader>
            <CardContent class="space-y-3">
              <div class="rounded-[24px] border border-destructive/18 bg-destructive/8 p-4">
                <p class="text-sm font-medium text-foreground">Content validation failure</p>
                <p class="mt-2 text-sm leading-6 text-muted-foreground">
                  One mission reward group still fails chain validation after the latest publish attempt.
                </p>
              </div>
              <div class="rounded-[24px] border border-primary/18 bg-primary/8 p-4">
                <p class="text-sm font-medium text-foreground">Spawn pressure alert</p>
                <p class="mt-2 text-sm leading-6 text-muted-foreground">
                  Cellblock density is above budget and should be watched during mission testing.
                </p>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader class="space-y-3">
              <Badge variant="secondary">Transport</Badge>
              <CardTitle>Log stream roadmap</CardTitle>
              <CardDescription>
                Layout space for the websocket-driven implementation.
              </CardDescription>
            </CardHeader>
            <CardContent class="space-y-3">
              <div class="flex items-center gap-3 rounded-[24px] border border-white/8 bg-white/[0.03] p-4">
                <Waves class="size-4 text-accent" />
                <p class="text-sm text-muted-foreground">
                  Tail this view via `ws/logs`, preserving severity filters client-side.
                </p>
              </div>
              <div class="flex items-center gap-3 rounded-[24px] border border-white/8 bg-white/[0.03] p-4">
                <ShieldAlert class="size-4 text-primary" />
                <p class="text-sm text-muted-foreground">
                  Incident creation can bind directly to selected log records or grouped windows.
                </p>
              </div>
            </CardContent>
          </Card>
        </div>
      </section>
    </div>
  );
}
