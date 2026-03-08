import { useCallback, useEffect, useMemo, useState } from 'react';
import { Compass, Layers3, LocateFixed, Orbit, RefreshCw } from 'lucide-react';
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
import { useResource } from '../lib/hooks';
import ConnectionStatus, { getConnectionError } from '../components/ConnectionStatus';
import { fetchSpaces, type SpaceRecord } from '../lib/admin-api';

export default function SpaceViewer() {
  const spacesResource = useResource(useCallback(fetchSpaces, []));
  const [selectedSpaceId, setSelectedSpaceId] = useState<number | null>(null);

  useEffect(() => {
    const spaces = spacesResource.data?.spaces ?? [];
    if (spaces.length > 0 && selectedSpaceId === null) {
      setSelectedSpaceId(spaces[0]!.world_id);
    }
  }, [spacesResource.data, selectedSpaceId]);

  const selectedSpace = useMemo<SpaceRecord | undefined>(
    () => spacesResource.data?.spaces.find((space) => space.world_id === selectedSpaceId),
    [spacesResource.data, selectedSpaceId],
  );

  return (
    <div className="space-y-6">
      <PageHeader
        actions={
          <>
            <Button onClick={() => spacesResource.refetch()} variant="secondary">
              <RefreshCw className="size-4" />
              Refresh spaces
            </Button>
            <Button disabled variant="outline">
              <LocateFixed className="size-4" />
              Follow entity
            </Button>
          </>
        }
        badge="Space Catalog"
        description="The space viewer now uses the real space catalog from the backend instead of a static placeholder layer list."
        eyebrow="World View"
        title="Spatial monitoring surface"
      />

      {getConnectionError(spacesResource.error) && (
        <ConnectionStatus onRetry={() => spacesResource.refetch()} />
      )}

      <section className="grid gap-4 xl:grid-cols-[1.25fr_0.75fr]">
        <Card className="overflow-hidden">
          <CardHeader className="space-y-3">
            <Badge variant="secondary">Viewport</Badge>
            <CardTitle>{selectedSpace?.world ?? 'Loading space catalog'}</CardTitle>
            <CardDescription>
              Backed by `/api/spaces`.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="relative overflow-hidden rounded-[28px] border border-white/8 bg-slate-950">
              <div className="absolute inset-0 bg-[radial-gradient(circle_at_20%_15%,rgba(245,170,49,0.2),transparent_18%),radial-gradient(circle_at_75%_22%,rgba(19,162,164,0.22),transparent_24%),linear-gradient(180deg,rgba(15,27,39,0.82),rgba(7,12,20,1))]" />
              <div className="absolute inset-0 bg-[linear-gradient(rgba(255,255,255,0.045)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.045)_1px,transparent_1px)] bg-[size:42px_42px] opacity-40" />
              <div className="relative flex h-[520px] flex-col justify-between p-6">
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <p className="text-xs font-semibold uppercase tracking-[0.26em] text-muted-foreground">
                      Loaded space
                    </p>
                    <h3 className="mt-2 text-2xl font-semibold tracking-[-0.06em] text-foreground">
                      {selectedSpace?.world ?? 'Loading...'}
                    </h3>
                    <p className="mt-2 text-sm text-muted-foreground">
                      Client map: {selectedSpace?.client_map ?? 'n/a'}
                    </p>
                  </div>
                  <Badge variant={selectedSpace?.has_script ? 'success' : 'outline'}>
                    {selectedSpace?.has_script ? 'Script enabled' : 'No script'}
                  </Badge>
                </div>

                <div className="grid gap-3 sm:grid-cols-3">
                  <div className="rounded-[24px] border border-white/8 bg-black/25 p-4">
                    <p className="text-xs uppercase tracking-[0.22em] text-muted-foreground">World ID</p>
                    <p className="mt-2 text-2xl font-semibold tracking-[-0.06em] text-foreground">
                      {selectedSpace?.world_id ?? '—'}
                    </p>
                  </div>
                  <div className="rounded-[24px] border border-white/8 bg-black/25 p-4">
                    <p className="text-xs uppercase tracking-[0.22em] text-muted-foreground">Mission Links</p>
                    <p className="mt-2 text-2xl font-semibold tracking-[-0.06em] text-foreground">
                      {selectedSpace?.mission_count ?? 0}
                    </p>
                  </div>
                  <div className="rounded-[24px] border border-white/8 bg-black/25 p-4">
                    <p className="text-xs uppercase tracking-[0.22em] text-muted-foreground">Flags</p>
                    <p className="mt-2 text-2xl font-semibold tracking-[-0.06em] text-foreground">
                      {selectedSpace?.flags ?? 0}
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        <div className="space-y-4">
          <Card>
            <CardHeader className="space-y-3">
              <Badge variant="outline">Loaded Spaces</Badge>
              <CardTitle>Catalog</CardTitle>
              <CardDescription>
                Select a world row to inspect its current metadata.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {spacesResource.error && !getConnectionError(spacesResource.error) && (
                <div className="text-sm text-destructive">{spacesResource.error.message}</div>
              )}
              {spacesResource.loading && !spacesResource.data && (
                <div className="text-sm text-muted-foreground">Loading space catalog...</div>
              )}
              {spacesResource.data && !spacesResource.data.available && (
                <div className="text-sm text-muted-foreground">{spacesResource.data.reason}</div>
              )}
              {(spacesResource.data?.spaces ?? []).map((space) => (
                <button
                  key={space.world_id}
                  className={`w-full rounded-[24px] border p-4 text-left transition ${
                    selectedSpaceId === space.world_id
                      ? 'border-primary/30 bg-primary/10'
                      : 'border-white/8 bg-white/[0.03]'
                  }`}
                  onClick={() => setSelectedSpaceId(space.world_id)}
                  type="button"
                >
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <p className="text-sm font-medium text-foreground">{space.world}</p>
                      <p className="mt-1 text-sm text-muted-foreground">
                        {space.mission_count} linked missions
                      </p>
                    </div>
                    <Badge variant={space.has_script ? 'success' : 'outline'}>
                      {space.has_script ? 'Script' : 'Data only'}
                    </Badge>
                  </div>
                  <div className="mt-4 space-y-2">
                    <div className="flex items-center justify-between text-xs uppercase tracking-[0.22em] text-muted-foreground">
                      <span>Mission density</span>
                      <span>{Math.min(space.mission_count, 100)}%</span>
                    </div>
                    <Progress value={Math.min(space.mission_count, 100)} />
                  </div>
                </button>
              ))}
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="space-y-3">
              <Badge variant="secondary">Inspection</Badge>
              <CardTitle>Camera modes</CardTitle>
              <CardDescription>
                UI reserved for the eventual scene integration.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <Button className="w-full justify-start" disabled variant="outline">
                <Compass className="size-4" />
                Recenter on selected zone
              </Button>
              <Button className="w-full justify-start" disabled variant="outline">
                <Layers3 className="size-4" />
                Toggle nav wireframe
              </Button>
              <Button className="w-full justify-start" disabled variant="outline">
                <Orbit className="size-4" />
                Lock to player path
              </Button>
            </CardContent>
          </Card>
        </div>
      </section>
    </div>
  );
}
