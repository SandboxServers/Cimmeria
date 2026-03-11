import { useCallback, useMemo } from 'react';
import { Download, LockKeyhole, RefreshCw, Save } from 'lucide-react';
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
import { useResource } from '../lib/hooks';
import ConnectionStatus, { getConnectionError } from '../components/ConnectionStatus';
import { buildConfigSections, fetchAdminConfig, fetchAdminStatus, formatUptime } from '../lib/admin-api';

export default function Config() {
  const config = useResource(useCallback(fetchAdminConfig, []));
  const status = useResource(useCallback(fetchAdminStatus, []));

  const sections = useMemo(() => (config.data ? buildConfigSections(config.data) : []), [config.data]);

  const refreshAll = () => {
    config.refetch();
    status.refetch();
  };

  return (
    <div className="space-y-6">
      <PageHeader
        actions={
          <>
            <Button onClick={refreshAll} variant="secondary">
              <RefreshCw className="size-4" />
              Refresh config
            </Button>
            <Button disabled variant="outline">
              <Save className="size-4" />
              Save draft config
            </Button>
          </>
        }
        badge="Config Surfaces"
        description="This page now loads its current values from the admin API instead of static mock configuration data."
        eyebrow="Configuration"
        title="Runtime and environment controls"
      />

      {getConnectionError(config.error, status.error) && (
        <ConnectionStatus onRetry={refreshAll} />
      )}

      <section className="grid gap-4 xl:grid-cols-[1.15fr_0.85fr]">
        <div className="grid gap-4">
          {config.error && !getConnectionError(config.error) && (
            <Card className="border-destructive/22 bg-destructive/8">
              <CardHeader>
                <Badge variant="destructive">Load Failed</Badge>
                <CardTitle>Config route error</CardTitle>
                <CardDescription>{config.error.message}</CardDescription>
              </CardHeader>
            </Card>
          )}
          {sections.length > 0 ? (
            sections.map((section) => (
              <Card key={section.title}>
                <CardHeader className="space-y-3">
                  <Badge variant="secondary">{section.title}</Badge>
                  <CardTitle>{section.title} settings</CardTitle>
                  <CardDescription>
                    Directly loaded from `/api/config`.
                  </CardDescription>
                </CardHeader>
                <CardContent className="grid gap-4 md:grid-cols-2">
                  {section.items.map((item) => (
                    <div key={item.label} className="space-y-2">
                      <label className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                        {item.label}
                      </label>
                      <Input readOnly value={item.value} />
                    </div>
                  ))}
                </CardContent>
              </Card>
            ))
          ) : (
            <Card><CardContent className="pt-6 text-sm text-muted-foreground">{config.loading ? 'Loading configuration...' : 'Configuration unavailable.'}</CardContent></Card>
          )}
        </div>

        <div className="space-y-4">
          <Card>
            <CardHeader className="space-y-3">
              <Badge variant="outline">Protection Model</Badge>
              <CardTitle>Runtime state</CardTitle>
              <CardDescription>
                Backed by `/api/config/status`.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="rounded-[24px] border border-white/8 bg-white/[0.03] p-4">
                <div className="flex items-center gap-3">
                  <LockKeyhole className="size-4 text-primary" />
                  <p className="text-sm font-medium text-foreground">Readonly defaults</p>
                </div>
                <p className="mt-2 text-sm leading-6 text-muted-foreground">
                  Config editing is still intentionally locked until save hooks exist on the backend.
                </p>
              </div>
              <div className="rounded-[24px] border border-primary/20 bg-primary/8 p-4">
                <p className="text-sm font-medium text-foreground">Uptime</p>
                <p className="mt-2 text-sm leading-6 text-muted-foreground">
                  {status.data
                    ? `Server has been up for ${formatUptime(status.data.uptime_seconds)}.`
                    : 'Loading status...'}
                </p>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="space-y-3">
              <Badge variant="secondary">Pending Actions</Badge>
              <CardTitle>Apply path</CardTitle>
              <CardDescription>
                Ready for the next phase once backend mutation endpoints exist.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <Button className="w-full justify-start" disabled variant="outline">
                <Save className="size-4" />
                Stage config patch
              </Button>
              <Button className="w-full justify-start" disabled variant="outline">
                <Download className="size-4" />
                Diff against disk
              </Button>
              <Button className="w-full justify-start" disabled variant="outline">
                <LockKeyhole className="size-4" />
                Review restart impact
              </Button>
            </CardContent>
          </Card>
        </div>
      </section>
    </div>
  );
}
