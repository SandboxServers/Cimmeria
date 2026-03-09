import type { Node } from '@xyflow/react';
import {
  Zap,
  Filter,
  Play,
  Hash,
  Info,
  Layers,
  Compass,
} from 'lucide-react';
import type {
  ChainFrameData,
  EditorNodeData,
  MissionNodeData,
  PrimitiveFamily,
} from '../editors/types';

const familyIcon: Record<PrimitiveFamily, React.ReactNode> = {
  anchor: <Compass className="h-3.5 w-3.5" />,
  trigger: <Zap className="h-3.5 w-3.5" />,
  condition: <Filter className="h-3.5 w-3.5" />,
  action: <Play className="h-3.5 w-3.5" />,
  counter: <Hash className="h-3.5 w-3.5" />,
};

const familyColor: Record<PrimitiveFamily, string> = {
  anchor: 'text-[#ffd0cf]',
  trigger: 'text-[#8ae5e2]',
  condition: 'text-[#ffd38a]',
  action: 'text-[#c7ffd5]',
  counter: 'text-[#d4dce5]',
};

interface InspectorProps {
  selectedNode: Node<EditorNodeData> | null;
}

export function Inspector({ selectedNode }: InspectorProps) {
  if (!selectedNode) {
    return (
      <div className="flex h-full items-center justify-center border-l border-border bg-card text-sm text-muted-foreground">
        Select a node to inspect
      </div>
    );
  }

  const data = selectedNode.data;

  if (data.kind === 'chain') {
    return <ChainInspector data={data as ChainFrameData} nodeId={selectedNode.id} />;
  }

  return <MissionInspector data={data as MissionNodeData} nodeId={selectedNode.id} />;
}

/* ----- Chain frame inspector ----- */

function ChainInspector({ data, nodeId }: { data: ChainFrameData; nodeId: string }) {
  return (
    <div className="subtle-scrollbar flex h-full flex-col overflow-y-auto border-l border-border bg-card">
      <div className="border-b border-border px-4 py-2">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Chain Inspector
        </h2>
      </div>

      <div className="space-y-4 p-4">
        <section>
          <h3 className="mb-2 flex items-center gap-1.5 text-sm font-medium text-foreground">
            <Layers className="h-3.5 w-3.5" />
            {data.name || '(unnamed)'}
          </h3>
          <div className="space-y-2 text-sm">
            <Field label="Node ID" value={nodeId} />
            <Field label="Summary" value={data.summary || '(none)'} />
            <Field label="Scope" value={`${data.scopeType} / ${data.scopeId}`} />
            <Field label="Priority" value={String(data.priority)} />
            <Field label="Enabled" value={data.enabled ? 'Yes' : 'No'} />
            <Field label="Source" value={data.source} />
            <Field label="Semantic" value={data.semantic} />
            <Field label="Space ID" value={data.spaceId} />
            {data.missionId && <Field label="Mission ID" value={data.missionId} />}
          </div>
        </section>

        {/* Per-family counts */}
        {data.counts && (
          <section>
            <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Primitive Counts
            </h3>
            <div className="grid grid-cols-2 gap-2">
              {(Object.entries(data.counts) as [PrimitiveFamily, number][]).map(
                ([family, count]) => (
                  <div
                    className="flex items-center gap-2 rounded border border-border bg-muted/50 px-3 py-1.5 text-xs"
                    key={family}
                  >
                    <span className={familyColor[family]}>{familyIcon[family]}</span>
                    <span className="capitalize text-foreground">{family}</span>
                    <span className="ml-auto font-mono text-muted-foreground">{count}</span>
                  </div>
                ),
              )}
            </div>
          </section>
        )}

        {/* Color swatch */}
        <section>
          <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Chain Color
          </h3>
          <div className="flex items-center gap-2">
            <div
              className="h-6 w-6 rounded-full border border-white/10"
              style={{ backgroundColor: data.color }}
            />
            <span className="font-mono text-xs text-muted-foreground">{data.color}</span>
          </div>
        </section>
      </div>
    </div>
  );
}

/* ----- Mission node inspector ----- */

function MissionInspector({ data, nodeId }: { data: MissionNodeData; nodeId: string }) {
  return (
    <div className="subtle-scrollbar flex h-full flex-col overflow-y-auto border-l border-border bg-card">
      <div className="border-b border-border px-4 py-2">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Node Inspector
        </h2>
      </div>

      <div className="space-y-4 p-4">
        {/* Header */}
        <section>
          <h3 className={`mb-2 flex items-center gap-1.5 text-sm font-medium ${familyColor[data.family]}`}>
            {familyIcon[data.family]}
            {data.title}
          </h3>
          <div className="space-y-2 text-sm">
            <Field label="Node ID" value={nodeId} />
            <Field label="Family" value={data.family} />
            <Field label="Stage" value={data.stage} />
            <Field label="Status" value={data.status} />
            <Field label="Scenario" value={data.scenario} />
          </div>
        </section>

        {/* Detail */}
        {data.detail && (
          <section>
            <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Detail
            </h3>
            <p className="text-sm leading-6 text-muted-foreground">{data.detail}</p>
          </section>
        )}

        {/* Properties */}
        <section>
          <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Properties ({data.properties.length})
          </h3>
          <div className="space-y-1.5">
            {data.properties.map((prop) => (
              <div
                className="rounded border border-border bg-muted/50 px-3 py-1.5 text-xs"
                key={prop.label}
              >
                <span className="uppercase tracking-[0.2em] text-muted-foreground">
                  {prop.label}
                </span>
                <span className="ml-2 font-mono text-foreground">{prop.value}</span>
              </div>
            ))}
            {data.properties.length === 0 && (
              <p className="text-xs text-muted-foreground">No properties defined</p>
            )}
          </div>
        </section>

        {/* Thread info */}
        {data.threadColor && (
          <section>
            <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Thread
            </h3>
            <div className="flex items-center gap-2">
              <div
                className="h-4 w-4 rounded-full border border-white/10"
                style={{ backgroundColor: data.threadColor }}
              />
              <span className="text-xs text-foreground">{data.threadName ?? 'Unnamed thread'}</span>
            </div>
          </section>
        )}

        {/* Accent color */}
        <section>
          <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Accent
          </h3>
          <div className="flex items-center gap-2">
            <div
              className="h-4 w-4 rounded-full border border-white/10"
              style={{ backgroundColor: data.accent }}
            />
            <span className="font-mono text-xs text-muted-foreground">{data.accent}</span>
          </div>
        </section>
      </div>
    </div>
  );
}

/* ----- Shared field component ----- */

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-baseline justify-between gap-2">
      <span className="text-muted-foreground">{label}</span>
      <span className="text-right font-mono text-xs text-foreground">{value}</span>
    </div>
  );
}
