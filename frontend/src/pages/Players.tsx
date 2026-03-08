import { useCallback, useMemo, useState } from 'react';
import { Crosshair, RefreshCw, Shield, TimerReset } from 'lucide-react';
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
import { useResource } from '../lib/hooks';
import { fetchPlayers } from '../lib/admin-api';
import { getPlayerStatusVariant } from '../lib/view-models';

export default function Players() {
  const [query, setQuery] = useState('');
  const playersResource = useResource(useCallback(fetchPlayers, []));

  const filteredPlayers = useMemo(() => {
    const players = playersResource.data?.players ?? [];
    const needle = query.trim().toLowerCase();

    if (!needle) {
      return players;
    }

    return players.filter((player) =>
      `${player.name} ${player.archetype} ${player.zone}`
        .toLowerCase()
        .includes(needle),
    );
  }, [playersResource.data, query]);

  return (
    <div className="space-y-6">
      <PageHeader
        actions={
          <>
            <Button onClick={() => playersResource.refetch()} variant="secondary">
              <RefreshCw className="size-4" />
              Sync sessions
            </Button>
            <Button disabled variant="outline">
              <Crosshair className="size-4" />
              Inspect selected
            </Button>
          </>
        }
        badge="Live Player Index"
        description="The player page now reads from the backend player route. Until the live roster feed exists, it reports the true service state instead of a fake sample roster."
        eyebrow="Players"
        title="Session and presence tracking"
      />

      <section className="grid gap-4 xl:grid-cols-[1.35fr_0.85fr]">
        <Card>
          <CardHeader className="gap-4 lg:flex-row lg:items-end lg:justify-between">
            <div className="space-y-2">
              <Badge variant="secondary">Roster</Badge>
              <CardTitle>Live shard population</CardTitle>
              <CardDescription>
                Searchable player list backed by `/api/players`.
              </CardDescription>
            </div>
            <div className="w-full max-w-sm">
              <Input
                onChange={(event) => setQuery(event.currentTarget.value)}
                placeholder="Search by player, archetype, or zone"
                value={query}
              />
            </div>
          </CardHeader>
          <CardContent className="px-0 pb-0">
            {playersResource.error && (
              <div className="px-6 pb-6 text-sm text-destructive">
                Failed to load player data: {playersResource.error.message}
              </div>
            )}
            {playersResource.loading && !playersResource.data && (
              <div className="px-6 pb-6 text-sm text-muted-foreground">Loading live roster...</div>
            )}
            {playersResource.data && !playersResource.data.available && (
              <div className="px-6 pb-6 text-sm text-muted-foreground">
                {playersResource.data.reason}
              </div>
            )}
            {filteredPlayers.length > 0 && (
              <div className="subtle-scrollbar overflow-x-auto">
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
                    {filteredPlayers.map((player) => (
                      <TableRow key={player.name}>
                        <TableCell className="font-medium text-foreground">{player.name}</TableCell>
                        <TableCell className="text-muted-foreground">{player.archetype}</TableCell>
                        <TableCell>{player.level}</TableCell>
                        <TableCell className="max-w-[220px] truncate text-muted-foreground">
                          {player.zone}
                        </TableCell>
                        <TableCell>
                          <Badge variant={getPlayerStatusVariant(player.status)}>
                            {player.status}
                          </Badge>
                        </TableCell>
                        <TableCell>{player.ping ?? 'n/a'}</TableCell>
                        <TableCell className="text-muted-foreground">{player.session}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            )}
          </CardContent>
        </Card>

        <div className="space-y-4">
          <Card>
            <CardHeader className="space-y-3">
              <Badge variant="outline">Watchlist</Badge>
              <CardTitle>Backend readiness</CardTitle>
              <CardDescription>
                What the server can currently tell the page about sessions.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="rounded-[24px] border border-white/8 bg-white/[0.03] p-4">
                <p className="text-sm font-medium text-foreground">Roster route</p>
                <p className="mt-2 text-sm leading-6 text-muted-foreground">
                  {playersResource.data?.available
                    ? 'Live roster feed is available.'
                    : playersResource.data?.reason ?? 'Waiting for player route response.'}
                </p>
              </div>
              <div className="rounded-[24px] border border-white/8 bg-white/[0.03] p-4">
                <p className="text-sm font-medium text-foreground">Base and cell services</p>
                <p className="mt-2 text-sm leading-6 text-muted-foreground">
                  {playersResource.data?.summary.ready
                    ? 'Runtime services are up, but detailed roster enumeration is still pending.'
                    : 'Runtime services are not fully ready for a roster feed.'}
                </p>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="space-y-3">
              <Badge variant="secondary">Operator Actions</Badge>
              <CardTitle>Session controls</CardTitle>
              <CardDescription>
                Reserved for the backend commands once the player route exposes real entities.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <Button className="w-full justify-start" disabled variant="outline">
                <Shield className="size-4" />
                Elevate player to GM monitor
              </Button>
              <Button className="w-full justify-start" disabled variant="outline">
                <Crosshair className="size-4" />
                Teleport camera to player
              </Button>
              <Button className="w-full justify-start" disabled variant="outline">
                <TimerReset className="size-4" />
                Reset stuck mission state
              </Button>
            </CardContent>
          </Card>
        </div>
      </section>
    </div>
  );
}
