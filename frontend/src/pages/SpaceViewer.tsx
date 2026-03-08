import { For, Show, createEffect, createMemo, createResource, createSignal } from 'solid-js';
import { Compass, Layers3, LocateFixed, Orbit, RefreshCw } from 'lucide-solid';
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
import { fetchSpaces, type SpaceRecord } from '../lib/admin-api';

export default function SpaceViewer() {
  const [spacesResource, { refetch }] = createResource(fetchSpaces);
  const [selectedSpaceId, setSelectedSpaceId] = createSignal<number | null>(null);

  createEffect(() => {
    const spaces = spacesResource()?.spaces ?? [];
    if (spaces.length > 0 && selectedSpaceId() === null) {
      setSelectedSpaceId(spaces[0]!.world_id);
    }
  });

  const selectedSpace = createMemo<SpaceRecord | undefined>(() =>
    spacesResource()?.spaces.find((space) => space.world_id === selectedSpaceId()),
  );

  return (
    <div class="space-y-6">
      <PageHeader
        actions={
          <>
            <Button onClick={() => void refetch()} variant="secondary">
              <RefreshCw class="size-4" />
              Refresh spaces
            </Button>
            <Button disabled variant="outline">
              <LocateFixed class="size-4" />
              Follow entity
            </Button>
          </>
        }
        badge="Space Catalog"
        description="The space viewer now uses the real space catalog from the backend instead of a static placeholder layer list."
        eyebrow="World View"
        title="Spatial monitoring surface"
      />

      <section class="grid gap-4 xl:grid-cols-[1.25fr_0.75fr]">
        <Card class="overflow-hidden">
          <CardHeader class="space-y-3">
            <Badge variant="secondary">Viewport</Badge>
            <CardTitle>{selectedSpace()?.world ?? 'Loading space catalog'}</CardTitle>
            <CardDescription>
              Backed by `/api/spaces`.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div class="relative overflow-hidden rounded-[28px] border border-white/8 bg-slate-950">
              <div class="absolute inset-0 bg-[radial-gradient(circle_at_20%_15%,rgba(245,170,49,0.2),transparent_18%),radial-gradient(circle_at_75%_22%,rgba(19,162,164,0.22),transparent_24%),linear-gradient(180deg,rgba(15,27,39,0.82),rgba(7,12,20,1))]" />
              <div class="absolute inset-0 bg-[linear-gradient(rgba(255,255,255,0.045)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.045)_1px,transparent_1px)] bg-[size:42px_42px] opacity-40" />
              <div class="relative flex h-[520px] flex-col justify-between p-6">
                <div class="flex items-start justify-between gap-3">
                  <div>
                    <p class="text-xs font-semibold uppercase tracking-[0.26em] text-muted-foreground">
                      Loaded space
                    </p>
                    <h3 class="mt-2 text-2xl font-semibold tracking-[-0.06em] text-foreground">
                      {selectedSpace()?.world ?? 'Loading...'}
                    </h3>
                    <p class="mt-2 text-sm text-muted-foreground">
                      Client map: {selectedSpace()?.client_map ?? 'n/a'}
                    </p>
                  </div>
                  <Badge variant={selectedSpace()?.has_script ? 'success' : 'outline'}>
                    {selectedSpace()?.has_script ? 'Script enabled' : 'No script'}
                  </Badge>
                </div>

                <div class="grid gap-3 sm:grid-cols-3">
                  <div class="rounded-[24px] border border-white/8 bg-black/25 p-4">
                    <p class="text-xs uppercase tracking-[0.22em] text-muted-foreground">World ID</p>
                    <p class="mt-2 text-2xl font-semibold tracking-[-0.06em] text-foreground">
                      {selectedSpace()?.world_id ?? '—'}
                    </p>
                  </div>
                  <div class="rounded-[24px] border border-white/8 bg-black/25 p-4">
                    <p class="text-xs uppercase tracking-[0.22em] text-muted-foreground">Mission Links</p>
                    <p class="mt-2 text-2xl font-semibold tracking-[-0.06em] text-foreground">
                      {selectedSpace()?.mission_count ?? 0}
                    </p>
                  </div>
                  <div class="rounded-[24px] border border-white/8 bg-black/25 p-4">
                    <p class="text-xs uppercase tracking-[0.22em] text-muted-foreground">Flags</p>
                    <p class="mt-2 text-2xl font-semibold tracking-[-0.06em] text-foreground">
                      {selectedSpace()?.flags ?? 0}
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        <div class="space-y-4">
          <Card>
            <CardHeader class="space-y-3">
              <Badge variant="outline">Loaded Spaces</Badge>
              <CardTitle>Catalog</CardTitle>
              <CardDescription>
                Select a world row to inspect its current metadata.
              </CardDescription>
            </CardHeader>
            <CardContent class="space-y-4">
              <Show when={spacesResource.error}>
                <div class="text-sm text-destructive">{spacesResource.error?.message}</div>
              </Show>
              <Show when={spacesResource.loading && !spacesResource()}>
                <div class="text-sm text-muted-foreground">Loading space catalog...</div>
              </Show>
              <Show when={spacesResource() && !spacesResource()!.available}>
                <div class="text-sm text-muted-foreground">{spacesResource()!.reason}</div>
              </Show>
              <For each={spacesResource()?.spaces ?? []}>
                {(space) => (
                  <button
                    class={`w-full rounded-[24px] border p-4 text-left transition ${
                      selectedSpaceId() === space.world_id
                        ? 'border-primary/30 bg-primary/10'
                        : 'border-white/8 bg-white/[0.03]'
                    }`}
                    onClick={() => setSelectedSpaceId(space.world_id)}
                    type="button"
                  >
                    <div class="flex items-center justify-between gap-3">
                      <div>
                        <p class="text-sm font-medium text-foreground">{space.world}</p>
                        <p class="mt-1 text-sm text-muted-foreground">
                          {space.mission_count} linked missions
                        </p>
                      </div>
                      <Badge variant={space.has_script ? 'success' : 'outline'}>
                        {space.has_script ? 'Script' : 'Data only'}
                      </Badge>
                    </div>
                    <div class="mt-4 space-y-2">
                      <div class="flex items-center justify-between text-xs uppercase tracking-[0.22em] text-muted-foreground">
                        <span>Mission density</span>
                        <span>{Math.min(space.mission_count, 100)}%</span>
                      </div>
                      <Progress value={Math.min(space.mission_count, 100)} />
                    </div>
                  </button>
                )}
              </For>
            </CardContent>
          </Card>

          <Card>
            <CardHeader class="space-y-3">
              <Badge variant="secondary">Inspection</Badge>
              <CardTitle>Camera modes</CardTitle>
              <CardDescription>
                UI reserved for the eventual scene integration.
              </CardDescription>
            </CardHeader>
            <CardContent class="space-y-3">
              <Button class="w-full justify-start" disabled variant="outline">
                <Compass class="size-4" />
                Recenter on selected zone
              </Button>
              <Button class="w-full justify-start" disabled variant="outline">
                <Layers3 class="size-4" />
                Toggle nav wireframe
              </Button>
              <Button class="w-full justify-start" disabled variant="outline">
                <Orbit class="size-4" />
                Lock to player path
              </Button>
            </CardContent>
          </Card>
        </div>
      </section>
    </div>
  );
}
