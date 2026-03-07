import { For, createSignal } from 'solid-js';
import { Boxes, FileCheck2, Save, Send } from 'lucide-solid';
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
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../components/ui/tabs';
import { Textarea } from '../components/ui/textarea';
import { contentDomains, publishQueue } from '../data/admin';

export default function ContentEditor() {
  const [activeDomain, setActiveDomain] = createSignal(contentDomains[0].name);
  const selectedDomain = () =>
    contentDomains.find((domain) => domain.name === activeDomain()) ?? contentDomains[0];

  return (
    <div class="space-y-6">
      <PageHeader
        actions={
          <>
            <Button variant="secondary">
              <FileCheck2 class="size-4" />
              Validate payload
            </Button>
            <Button>
              <Send class="size-4" />
              Publish revision
            </Button>
          </>
        }
        badge="Hot Reload Ready"
        description="The content editor now reads like a real production tool: domain tabs, revision metadata, draft content, and a visible publish queue for live shards."
        eyebrow="Content"
        title="Structured game data editing"
      />

      <Tabs defaultValue={contentDomains[0].name} onValueChange={setActiveDomain}>
        <TabsList class="w-full justify-start">
          <For each={contentDomains}>
            {(domain) => <TabsTrigger value={domain.name}>{domain.name}</TabsTrigger>}
          </For>
        </TabsList>

        <For each={contentDomains}>
          {(domain) => (
            <TabsContent value={domain.name}>
              <section class="grid gap-4 xl:grid-cols-[1.2fr_0.8fr]">
                <Card>
                  <CardHeader class="gap-4 lg:flex-row lg:items-center lg:justify-between">
                    <div class="space-y-2">
                      <Badge variant="secondary">{selectedDomain().owner}</Badge>
                      <CardTitle>{selectedDomain().name} workspace</CardTitle>
                      <CardDescription>{selectedDomain().summary}</CardDescription>
                    </div>
                    <div class="rounded-[24px] border border-white/8 bg-white/[0.03] px-4 py-3 text-right">
                      <p class="text-xs uppercase tracking-[0.22em] text-muted-foreground">
                        Records
                      </p>
                      <p class="mt-1 text-2xl font-semibold tracking-[-0.06em] text-foreground">
                        {selectedDomain().records}
                      </p>
                    </div>
                  </CardHeader>
                  <CardContent class="space-y-4">
                    <div class="grid gap-4 md:grid-cols-2">
                      <div class="space-y-2">
                        <label class="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                          Resource path
                        </label>
                        <Input value={`${domain.name.toLowerCase()}/sample-record`} />
                      </div>
                      <div class="space-y-2">
                        <label class="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                          Revision tag
                        </label>
                        <Input value="phase4-ui-pass" />
                      </div>
                    </div>
                    <div class="space-y-2">
                      <label class="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                        Editor notes
                      </label>
                      <Textarea value={`Editing ${domain.name} definitions with a phase 4 operator workflow. Preserve schema safety, show ownership, and surface publish intent before commit.`} />
                    </div>
                    <div class="space-y-2">
                      <label class="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                        Draft payload
                      </label>
                      <Textarea
                        class="min-h-60 font-mono text-xs leading-6"
                        value={`{\n  "domain": "${domain.name}",\n  "owner": "${domain.owner}",\n  "status": "draft",\n  "records": ${domain.records},\n  "validation": {\n    "schema": "pass",\n    "references": "pass",\n    "publish": "queued"\n  }\n}`}
                      />
                    </div>
                    <div class="flex flex-wrap gap-3">
                      <Button>
                        <Save class="size-4" />
                        Save draft
                      </Button>
                      <Button variant="outline">
                        <Boxes class="size-4" />
                        Compare with live
                      </Button>
                    </div>
                  </CardContent>
                </Card>

                <div class="space-y-4">
                  <Card>
                    <CardHeader class="space-y-3">
                      <Badge variant="outline">Publish Queue</Badge>
                      <CardTitle>Queued revisions</CardTitle>
                      <CardDescription>
                        Changes staged for review or hot reload rollout.
                      </CardDescription>
                    </CardHeader>
                    <CardContent class="space-y-3">
                      <For each={publishQueue}>
                        {(item) => (
                          <div class="rounded-[24px] border border-white/8 bg-white/[0.03] p-4">
                            <div class="flex items-start justify-between gap-3">
                              <div>
                                <p class="text-sm font-medium text-foreground">{item.target}</p>
                                <p class="mt-1 text-sm text-muted-foreground">{item.author}</p>
                              </div>
                              <Badge
                                variant={
                                  item.state === 'Ready'
                                    ? 'success'
                                    : item.state === 'Review'
                                      ? 'secondary'
                                      : 'outline'
                                }
                              >
                                {item.state}
                              </Badge>
                            </div>
                          </div>
                        )}
                      </For>
                    </CardContent>
                  </Card>

                  <Card>
                    <CardHeader class="space-y-3">
                      <Badge variant="secondary">Safety Rails</Badge>
                      <CardTitle>Validation envelope</CardTitle>
                      <CardDescription>
                        The UI reserves space for schema errors, referential drift, and hot reload warnings.
                      </CardDescription>
                    </CardHeader>
                    <CardContent class="space-y-3">
                      <div class="rounded-[24px] border border-emerald-400/20 bg-emerald-400/10 p-4">
                        <p class="text-sm font-medium text-foreground">Schema check</p>
                        <p class="mt-2 text-sm leading-6 text-muted-foreground">
                          No missing columns or enum mismatches detected in the draft view.
                        </p>
                      </div>
                      <div class="rounded-[24px] border border-primary/20 bg-primary/8 p-4">
                        <p class="text-sm font-medium text-foreground">Reload impact</p>
                        <p class="mt-2 text-sm leading-6 text-muted-foreground">
                          Safe for live apply if downstream mission chains stay unchanged.
                        </p>
                      </div>
                    </CardContent>
                  </Card>
                </div>
              </section>
            </TabsContent>
          )}
        </For>
      </Tabs>
    </div>
  );
}
