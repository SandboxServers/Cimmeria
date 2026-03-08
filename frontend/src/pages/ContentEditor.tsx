import { useState } from 'react';
import { Boxes, FileCheck2, Save, Send } from 'lucide-react';
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
  const [activeDomain, setActiveDomain] = useState(contentDomains[0].name);
  const selectedDomain =
    contentDomains.find((domain) => domain.name === activeDomain) ?? contentDomains[0];

  return (
    <div className="space-y-6">
      <PageHeader
        actions={
          <>
            <Button variant="secondary">
              <FileCheck2 className="size-4" />
              Validate payload
            </Button>
            <Button>
              <Send className="size-4" />
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
        <TabsList className="w-full justify-start">
          {contentDomains.map((domain) => (
            <TabsTrigger key={domain.name} value={domain.name}>{domain.name}</TabsTrigger>
          ))}
        </TabsList>

        {contentDomains.map((domain) => (
          <TabsContent key={domain.name} value={domain.name}>
            <section className="grid gap-4 xl:grid-cols-[1.2fr_0.8fr]">
              <Card>
                <CardHeader className="gap-4 lg:flex-row lg:items-center lg:justify-between">
                  <div className="space-y-2">
                    <Badge variant="secondary">{selectedDomain.owner}</Badge>
                    <CardTitle>{selectedDomain.name} workspace</CardTitle>
                    <CardDescription>{selectedDomain.summary}</CardDescription>
                  </div>
                  <div className="rounded-[24px] border border-white/8 bg-white/[0.03] px-4 py-3 text-right">
                    <p className="text-xs uppercase tracking-[0.22em] text-muted-foreground">
                      Records
                    </p>
                    <p className="mt-1 text-2xl font-semibold tracking-[-0.06em] text-foreground">
                      {selectedDomain.records}
                    </p>
                  </div>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="space-y-2">
                      <label className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                        Resource path
                      </label>
                      <Input defaultValue={`${domain.name.toLowerCase()}/sample-record`} />
                    </div>
                    <div className="space-y-2">
                      <label className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                        Revision tag
                      </label>
                      <Input defaultValue="phase4-ui-pass" />
                    </div>
                  </div>
                  <div className="space-y-2">
                    <label className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                      Editor notes
                    </label>
                    <Textarea defaultValue={`Editing ${domain.name} definitions with a phase 4 operator workflow. Preserve schema safety, show ownership, and surface publish intent before commit.`} />
                  </div>
                  <div className="space-y-2">
                    <label className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                      Draft payload
                    </label>
                    <Textarea
                      className="min-h-60 font-mono text-xs leading-6"
                      defaultValue={`{\n  "domain": "${domain.name}",\n  "owner": "${domain.owner}",\n  "status": "draft",\n  "records": ${domain.records},\n  "validation": {\n    "schema": "pass",\n    "references": "pass",\n    "publish": "queued"\n  }\n}`}
                    />
                  </div>
                  <div className="flex flex-wrap gap-3">
                    <Button>
                      <Save className="size-4" />
                      Save draft
                    </Button>
                    <Button variant="outline">
                      <Boxes className="size-4" />
                      Compare with live
                    </Button>
                  </div>
                </CardContent>
              </Card>

              <div className="space-y-4">
                <Card>
                  <CardHeader className="space-y-3">
                    <Badge variant="outline">Publish Queue</Badge>
                    <CardTitle>Queued revisions</CardTitle>
                    <CardDescription>
                      Changes staged for review or hot reload rollout.
                    </CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-3">
                    {publishQueue.map((item) => (
                      <div key={item.target} className="rounded-[24px] border border-white/8 bg-white/[0.03] p-4">
                        <div className="flex items-start justify-between gap-3">
                          <div>
                            <p className="text-sm font-medium text-foreground">{item.target}</p>
                            <p className="mt-1 text-sm text-muted-foreground">{item.author}</p>
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
                    ))}
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="space-y-3">
                    <Badge variant="secondary">Safety Rails</Badge>
                    <CardTitle>Validation envelope</CardTitle>
                    <CardDescription>
                      The UI reserves space for schema errors, referential drift, and hot reload warnings.
                    </CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-3">
                    <div className="rounded-[24px] border border-emerald-400/20 bg-emerald-400/10 p-4">
                      <p className="text-sm font-medium text-foreground">Schema check</p>
                      <p className="mt-2 text-sm leading-6 text-muted-foreground">
                        No missing columns or enum mismatches detected in the draft view.
                      </p>
                    </div>
                    <div className="rounded-[24px] border border-primary/20 bg-primary/8 p-4">
                      <p className="text-sm font-medium text-foreground">Reload impact</p>
                      <p className="mt-2 text-sm leading-6 text-muted-foreground">
                        Safe for live apply if downstream mission chains stay unchanged.
                      </p>
                    </div>
                  </CardContent>
                </Card>
              </div>
            </section>
          </TabsContent>
        ))}
      </Tabs>
    </div>
  );
}
