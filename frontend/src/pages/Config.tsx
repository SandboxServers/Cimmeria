import { For, Show, createMemo, createResource } from 'solid-js';
import { Download, LockKeyhole, RefreshCw, Save } from 'lucide-solid';
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
import { buildConfigSections, fetchAdminConfig, fetchAdminStatus, formatUptime } from '../lib/admin-api';

export default function Config() {
  const [config, { refetch: refetchConfig }] = createResource(fetchAdminConfig);
  const [status, { refetch: refetchStatus }] = createResource(fetchAdminStatus);

  const sections = createMemo(() => (config() ? buildConfigSections(config()!) : []));

  const refreshAll = () => {
    void refetchConfig();
    void refetchStatus();
  };

  return (
    <div class="space-y-6">
      <PageHeader
        actions={
          <>
            <Button onClick={refreshAll} variant="secondary">
              <RefreshCw class="size-4" />
              Refresh config
            </Button>
            <Button disabled variant="outline">
              <Save class="size-4" />
              Save draft config
            </Button>
          </>
        }
        badge="Config Surfaces"
        description="This page now loads its current values from the admin API instead of static mock configuration data."
        eyebrow="Configuration"
        title="Runtime and environment controls"
      />

      <section class="grid gap-4 xl:grid-cols-[1.15fr_0.85fr]">
        <div class="grid gap-4">
          <Show when={config.error}>
            <Card class="border-destructive/22 bg-destructive/8">
              <CardHeader>
                <Badge variant="destructive">Load Failed</Badge>
                <CardTitle>Config route error</CardTitle>
                <CardDescription>{config.error?.message}</CardDescription>
              </CardHeader>
            </Card>
          </Show>
          <Show when={sections().length > 0} fallback={<Card><CardContent class="pt-6 text-sm text-muted-foreground">{config.loading ? 'Loading configuration...' : 'Configuration unavailable.'}</CardContent></Card>}>
            <For each={sections()}>
              {(section) => (
                <Card>
                  <CardHeader class="space-y-3">
                    <Badge variant="secondary">{section.title}</Badge>
                    <CardTitle>{section.title} settings</CardTitle>
                    <CardDescription>
                      Directly loaded from `/api/config`.
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
          </Show>
        </div>

        <div class="space-y-4">
          <Card>
            <CardHeader class="space-y-3">
              <Badge variant="outline">Protection Model</Badge>
              <CardTitle>Runtime state</CardTitle>
              <CardDescription>
                Backed by `/api/config/status`.
              </CardDescription>
            </CardHeader>
            <CardContent class="space-y-3">
              <div class="rounded-[24px] border border-white/8 bg-white/[0.03] p-4">
                <div class="flex items-center gap-3">
                  <LockKeyhole class="size-4 text-primary" />
                  <p class="text-sm font-medium text-foreground">Readonly defaults</p>
                </div>
                <p class="mt-2 text-sm leading-6 text-muted-foreground">
                  Config editing is still intentionally locked until save hooks exist on the backend.
                </p>
              </div>
              <div class="rounded-[24px] border border-primary/20 bg-primary/8 p-4">
                <p class="text-sm font-medium text-foreground">Uptime</p>
                <p class="mt-2 text-sm leading-6 text-muted-foreground">
                  {status()
                    ? `Server has been up for ${formatUptime(status()!.uptime_seconds)}.`
                    : 'Loading status...'}
                </p>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader class="space-y-3">
              <Badge variant="secondary">Pending Actions</Badge>
              <CardTitle>Apply path</CardTitle>
              <CardDescription>
                Ready for the next phase once backend mutation endpoints exist.
              </CardDescription>
            </CardHeader>
            <CardContent class="space-y-3">
              <Button class="w-full justify-start" disabled variant="outline">
                <Save class="size-4" />
                Stage config patch
              </Button>
              <Button class="w-full justify-start" disabled variant="outline">
                <Download class="size-4" />
                Diff against disk
              </Button>
              <Button class="w-full justify-start" disabled variant="outline">
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
