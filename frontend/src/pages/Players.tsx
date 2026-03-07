import { For, createMemo, createSignal } from 'solid-js';
import { Crosshair, Search, Shield, TimerReset } from 'lucide-solid';
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
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '../components/ui/table';
import { players } from '../data/admin';

const getStatusVariant = (status: string) => {
  switch (status) {
    case 'Combat':
      return 'destructive';
    case 'In mission':
      return 'default';
    case 'Crafting':
      return 'secondary';
    default:
      return 'success';
  }
};

export default function Players() {
  const [query, setQuery] = createSignal('');
  const filteredPlayers = createMemo(() =>
    players.filter((player) =>
      `${player.name} ${player.archetype} ${player.zone}`
        .toLowerCase()
        .includes(query().trim().toLowerCase()),
    ),
  );

  return (
    <div class="space-y-6">
      <PageHeader
        actions={
          <>
            <Button variant="secondary">
              <TimerReset class="size-4" />
              Sync sessions
            </Button>
            <Button>
              <Crosshair class="size-4" />
              Inspect selected
            </Button>
          </>
        }
        badge="Live Player Index"
        description="This view covers the core phase 4 monitoring requirement: who is online, where they are, and whether their current gameplay state needs intervention."
        eyebrow="Players"
        title="Session and presence tracking"
      />

      <section class="grid gap-4 xl:grid-cols-[1.35fr_0.85fr]">
        <Card>
          <CardHeader class="gap-4 lg:flex-row lg:items-end lg:justify-between">
            <div class="space-y-2">
              <Badge variant="secondary">Roster</Badge>
              <CardTitle>Live shard population</CardTitle>
              <CardDescription>
                Searchable player list with status, location, level, and latency.
              </CardDescription>
            </div>
            <div class="relative w-full max-w-sm">
              <Search class="pointer-events-none absolute left-4 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                class="pl-11"
                onInput={(event) => setQuery(event.currentTarget.value)}
                placeholder="Search by player, archetype, or zone"
                value={query()}
              />
            </div>
          </CardHeader>
          <CardContent class="px-0 pb-0">
            <div class="subtle-scrollbar overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Name</TableHead>
                    <TableHead>Archetype</TableHead>
                    <TableHead>Level</TableHead>
                    <TableHead>Zone</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Ping</TableHead>
                    <TableHead>Session</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  <For each={filteredPlayers()}>
                    {(player) => (
                      <TableRow>
                        <TableCell class="font-medium text-foreground">{player.name}</TableCell>
                        <TableCell class="text-muted-foreground">{player.archetype}</TableCell>
                        <TableCell>{player.level}</TableCell>
                        <TableCell class="max-w-[220px] truncate text-muted-foreground">
                          {player.zone}
                        </TableCell>
                        <TableCell>
                          <Badge variant={getStatusVariant(player.status)}>{player.status}</Badge>
                        </TableCell>
                        <TableCell>{player.ping}ms</TableCell>
                        <TableCell class="text-muted-foreground">{player.session}</TableCell>
                      </TableRow>
                    )}
                  </For>
                </TableBody>
              </Table>
            </div>
          </CardContent>
        </Card>

        <div class="space-y-4">
          <Card>
            <CardHeader class="space-y-3">
              <Badge variant="outline">Watchlist</Badge>
              <CardTitle>Intervention cues</CardTitle>
              <CardDescription>
                Targeted actions to speed up debugging and live support.
              </CardDescription>
            </CardHeader>
            <CardContent class="space-y-3">
              <div class="rounded-[24px] border border-destructive/20 bg-destructive/8 p-4">
                <p class="text-sm font-medium text-foreground">Combat spike</p>
                <p class="mt-2 text-sm leading-6 text-muted-foreground">
                  Kael Mercer has been in uninterrupted combat for 9m 12s in Castle / Cellblock.
                </p>
              </div>
              <div class="rounded-[24px] border border-white/8 bg-white/[0.03] p-4">
                <p class="text-sm font-medium text-foreground">Latency drift</p>
                <p class="mt-2 text-sm leading-6 text-muted-foreground">
                  Round-trip pings remain stable overall, with one player above 60ms.
                </p>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader class="space-y-3">
              <Badge variant="secondary">Operator Actions</Badge>
              <CardTitle>Session controls</CardTitle>
              <CardDescription>
                Action patterns the backend can attach to commands or websocket flows later.
              </CardDescription>
            </CardHeader>
            <CardContent class="space-y-3">
              <Button class="w-full justify-start" variant="outline">
                <Shield class="size-4" />
                Elevate player to GM monitor
              </Button>
              <Button class="w-full justify-start" variant="outline">
                <Crosshair class="size-4" />
                Teleport camera to player
              </Button>
              <Button class="w-full justify-start" variant="outline">
                <TimerReset class="size-4" />
                Reset stuck mission state
              </Button>
            </CardContent>
          </Card>
        </div>
      </section>
    </div>
  );
}
