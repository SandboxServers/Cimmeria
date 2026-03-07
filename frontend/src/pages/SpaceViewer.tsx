import { For } from 'solid-js';
import { Compass, Layers3, LocateFixed, Orbit } from 'lucide-solid';
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
import { spaceLayers } from '../data/admin';

export default function SpaceViewer() {
  return (
    <div class="space-y-6">
      <PageHeader
        actions={
          <>
            <Button variant="secondary">
              <Orbit class="size-4" />
              Orbit camera
            </Button>
            <Button>
              <LocateFixed class="size-4" />
              Follow entity
            </Button>
          </>
        }
        badge="Phase 5 Preview"
        description="The viewer route remains part of the shell, but its design now matches the phase 4 dashboard instead of feeling like a disconnected placeholder."
        eyebrow="World View"
        title="Spatial monitoring surface"
      />

      <section class="grid gap-4 xl:grid-cols-[1.25fr_0.75fr]">
        <Card class="overflow-hidden">
          <CardHeader class="space-y-3">
            <Badge variant="secondary">Viewport</Badge>
            <CardTitle>Harset command center</CardTitle>
            <CardDescription>
              Space viewer frame sized for the future Three.js scene, overlays, and entity markers.
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
                      Harset.nav
                    </h3>
                  </div>
                  <Badge variant="success">Geometry stub ready</Badge>
                </div>

                <div class="grid gap-3 sm:grid-cols-3">
                  <div class="rounded-[24px] border border-white/8 bg-black/25 p-4">
                    <p class="text-xs uppercase tracking-[0.22em] text-muted-foreground">Cells</p>
                    <p class="mt-2 text-2xl font-semibold tracking-[-0.06em] text-foreground">
                      12
                    </p>
                  </div>
                  <div class="rounded-[24px] border border-white/8 bg-black/25 p-4">
                    <p class="text-xs uppercase tracking-[0.22em] text-muted-foreground">Tracked entities</p>
                    <p class="mt-2 text-2xl font-semibold tracking-[-0.06em] text-foreground">
                      84
                    </p>
                  </div>
                  <div class="rounded-[24px] border border-white/8 bg-black/25 p-4">
                    <p class="text-xs uppercase tracking-[0.22em] text-muted-foreground">Nav chunks</p>
                    <p class="mt-2 text-2xl font-semibold tracking-[-0.06em] text-foreground">
                      124
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
              <Badge variant="outline">Overlay Controls</Badge>
              <CardTitle>Renderable layers</CardTitle>
              <CardDescription>
                UI slots for nav meshes, spawn points, players, and debug overlays.
              </CardDescription>
            </CardHeader>
            <CardContent class="space-y-4">
              <For each={spaceLayers}>
                {(layer) => (
                  <div class="rounded-[24px] border border-white/8 bg-white/[0.03] p-4">
                    <div class="flex items-center justify-between gap-3">
                      <div>
                        <p class="text-sm font-medium text-foreground">{layer.label}</p>
                        <p class="mt-1 text-sm text-muted-foreground">{layer.count} tracked</p>
                      </div>
                      <Badge variant={layer.enabled ? 'success' : 'outline'}>
                        {layer.enabled ? 'Visible' : 'Hidden'}
                      </Badge>
                    </div>
                    <div class="mt-4 space-y-2">
                      <div class="flex items-center justify-between text-xs uppercase tracking-[0.22em] text-muted-foreground">
                        <span>Density</span>
                        <span>{Math.min(layer.count, 100)}%</span>
                      </div>
                      <Progress value={Math.min(layer.count, 100)} />
                    </div>
                  </div>
                )}
              </For>
            </CardContent>
          </Card>

          <Card>
            <CardHeader class="space-y-3">
              <Badge variant="secondary">Inspection</Badge>
              <CardTitle>Camera modes</CardTitle>
              <CardDescription>
                Controls positioned for the eventual Three.js integration.
              </CardDescription>
            </CardHeader>
            <CardContent class="space-y-3">
              <Button class="w-full justify-start" variant="outline">
                <Compass class="size-4" />
                Recenter on selected zone
              </Button>
              <Button class="w-full justify-start" variant="outline">
                <Layers3 class="size-4" />
                Toggle nav wireframe
              </Button>
              <Button class="w-full justify-start" variant="outline">
                <LocateFixed class="size-4" />
                Lock to player path
              </Button>
            </CardContent>
          </Card>
        </div>
      </section>
    </div>
  );
}
