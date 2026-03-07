import { For } from 'solid-js';
import { Download, LockKeyhole, Save } from 'lucide-solid';
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
import { configSections } from '../data/admin';

export default function Config() {
  return (
    <div class="space-y-6">
      <PageHeader
        actions={
          <>
            <Button variant="secondary">
              <Download class="size-4" />
              Export snapshot
            </Button>
            <Button>
              <Save class="size-4" />
              Save draft config
            </Button>
          </>
        }
        badge="Config Surfaces"
        description="The config page now matches the rest of the admin panel: grouped settings, operator context, and clear room for editable versus protected values."
        eyebrow="Configuration"
        title="Runtime and environment controls"
      />

      <section class="grid gap-4 xl:grid-cols-[1.15fr_0.85fr]">
        <div class="grid gap-4">
          <For each={configSections}>
            {(section) => (
              <Card>
                <CardHeader class="space-y-3">
                  <Badge variant="secondary">{section.title}</Badge>
                  <CardTitle>{section.title} settings</CardTitle>
                  <CardDescription>
                    Structured rows sized for XML config loading and eventual inline edits.
                  </CardDescription>
                </CardHeader>
                <CardContent class="grid gap-4 md:grid-cols-2">
                  <For each={section.items}>
                    {(item) => (
                      <div class="space-y-2">
                        <label class="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                          {item.label}
                        </label>
                        <Input readonly value={item.value} />
                      </div>
                    )}
                  </For>
                </CardContent>
              </Card>
            )}
          </For>
        </div>

        <div class="space-y-4">
          <Card>
            <CardHeader class="space-y-3">
              <Badge variant="outline">Protection Model</Badge>
              <CardTitle>Change governance</CardTitle>
              <CardDescription>
                Config mutations should be deliberate, reviewable, and easy to roll back.
              </CardDescription>
            </CardHeader>
            <CardContent class="space-y-3">
              <div class="rounded-[24px] border border-white/8 bg-white/[0.03] p-4">
                <div class="flex items-center gap-3">
                  <LockKeyhole class="size-4 text-primary" />
                  <p class="text-sm font-medium text-foreground">Readonly defaults</p>
                </div>
                <p class="mt-2 text-sm leading-6 text-muted-foreground">
                  Sensitive ports, filesystem paths, and runtime flags remain locked until backend save hooks exist.
                </p>
              </div>
              <div class="rounded-[24px] border border-primary/20 bg-primary/8 p-4">
                <p class="text-sm font-medium text-foreground">Restart requirement</p>
                <p class="mt-2 text-sm leading-6 text-muted-foreground">
                  Network and persistence changes should surface restart warnings before writeback.
                </p>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader class="space-y-3">
              <Badge variant="secondary">Pending Actions</Badge>
              <CardTitle>Apply path</CardTitle>
              <CardDescription>
                UI affordances reserved for future config IPC or admin API endpoints.
              </CardDescription>
            </CardHeader>
            <CardContent class="space-y-3">
              <Button class="w-full justify-start" variant="outline">
                <Save class="size-4" />
                Stage config patch
              </Button>
              <Button class="w-full justify-start" variant="outline">
                <Download class="size-4" />
                Diff against disk
              </Button>
              <Button class="w-full justify-start" variant="outline">
                <LockKeyhole class="size-4" />
                Review restart impact
              </Button>
            </CardContent>
          </Card>
        </div>
      </section>
    </div>
  );
}
