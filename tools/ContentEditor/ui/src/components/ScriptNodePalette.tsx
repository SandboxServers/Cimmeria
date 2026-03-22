import { useMemo, useState } from 'react';
import { nodeTypeColors, portTypeColors, type ScriptNodeTemplate } from '../editors/scriptTypes';

interface ScriptNodePaletteProps {
  templates: ScriptNodeTemplate[];
  scriptType: string;
  onAdd: (templateRef: string) => void;
}

const nodeTypeOrder = ['Variable', 'Action', 'Condition', 'Event'];

/**
 * Searchable palette panel for browsing script node templates.
 *
 * Features:
 * - Search by name, description, category
 * - Filter by node type (Variable / Action / Condition / Event)
 * - Only shows nodes compatible with the active script type
 * - Click to add to canvas
 * - Shows port summary for each template
 */
export default function ScriptNodePalette({
  templates,
  scriptType,
  onAdd,
}: ScriptNodePaletteProps) {
  const [search, setSearch] = useState('');
  const [typeFilter, setTypeFilter] = useState<string | 'all'>('all');

  const filteredTemplates = useMemo(() => {
    let result = templates.filter(
      (t) =>
        t.script_types.length === 0 || t.script_types.includes(scriptType),
    );

    if (typeFilter !== 'all') {
      result = result.filter((t) => t.node_type === typeFilter);
    }

    if (search.trim()) {
      const q = search.toLowerCase();
      result = result.filter(
        (t) =>
          t.display_name.toLowerCase().includes(q) ||
          t.ref_name.toLowerCase().includes(q) ||
          t.description.toLowerCase().includes(q) ||
          t.category.toLowerCase().includes(q),
      );
    }

    return result;
  }, [templates, scriptType, typeFilter, search]);

  // Group by category
  const grouped = useMemo(() => {
    const groups = new Map<string, ScriptNodeTemplate[]>();
    for (const t of filteredTemplates) {
      const key = t.category || 'Uncategorized';
      const arr = groups.get(key) ?? [];
      arr.push(t);
      groups.set(key, arr);
    }
    // Sort groups by name
    return Array.from(groups.entries()).sort((a, b) => a[0].localeCompare(b[0]));
  }, [filteredTemplates]);

  return (
    <section className="obsidian-panel border border-[rgba(77,70,53,0.15)] bg-surface-low p-6 shadow-[0_24px_80px_rgba(0,0,0,0.22)]">
      <div className="mb-4">
        <p className="font-headline text-xs uppercase tracking-widest text-primary">
          Script Node Palette
        </p>
        <p className="mt-2 max-w-3xl text-sm leading-6 text-on-surface-variant">
          Browse available script nodes. Click a node to add it to the canvas.
        </p>
      </div>

      {/* Filters */}
      <div className="mb-4 grid gap-3 sm:grid-cols-[minmax(0,2fr)_minmax(0,1fr)]">
        <label className="border border-[rgba(77,70,53,0.15)] bg-surface-lowest px-4 py-3">
          <span className="text-[10px] font-dense uppercase tracking-widest text-primary">
            Search
          </span>
          <input
            className="mt-2 w-full bg-transparent text-sm text-on-surface outline-none placeholder:text-on-surface-variant/50"
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Find by name, category, or description"
            type="search"
            value={search}
          />
        </label>

        <div className="flex gap-2">
          {['all', ...nodeTypeOrder].map((nt) => {
            const active = typeFilter === nt;
            const colors = nt !== 'all' ? nodeTypeColors[nt] : null;
            return (
              <button
                className={`flex-1 border px-2 py-2 text-[10px] font-semibold uppercase tracking-[0.16em] transition-colors ${
                  active
                    ? 'border-primary/40 bg-primary/18 text-primary'
                    : 'border-[rgba(77,70,53,0.15)] bg-surface-lowest text-on-surface-variant hover:bg-surface-highest'
                }`}
                key={nt}
                onClick={() => setTypeFilter(nt)}
                type="button"
                style={
                  active && colors
                    ? {
                        borderColor: colors.border,
                        backgroundColor: colors.bg,
                        color: colors.text,
                      }
                    : undefined
                }
              >
                {nt === 'all' ? 'All' : nt}
              </button>
            );
          })}
        </div>
      </div>

      {/* Grouped results */}
      <div className="grid gap-4 2xl:grid-cols-2">
        {grouped.length > 0 ? (
          grouped.map(([category, items]) => (
            <section
              className="border border-[rgba(77,70,53,0.15)] bg-surface-low p-5"
              key={category}
            >
              <p className="font-headline mb-3 text-xs uppercase tracking-widest text-primary">
                {category}
              </p>
              <div className="grid gap-2 md:grid-cols-2">
                {items.map((t) => (
                  <PaletteCard key={t.ref_name} template={t} onAdd={onAdd} />
                ))}
              </div>
            </section>
          ))
        ) : (
          <section className="border border-dashed border-[rgba(77,70,53,0.15)] bg-surface-lowest p-10 text-center text-sm text-on-surface-variant 2xl:col-span-2">
            No nodes match the current search/filter combination.
          </section>
        )}
      </div>
    </section>
  );
}

/* ----- Individual palette card ----- */

function PaletteCard({
  template,
  onAdd,
}: {
  template: ScriptNodeTemplate;
  onAdd: (ref: string) => void;
}) {
  const colors = nodeTypeColors[template.node_type] ?? nodeTypeColors.Action;
  const inputs = template.ports.filter((p) => p.direction === 'in');
  const outputs = template.ports.filter((p) => p.direction === 'out');

  return (
    <button
      className="border border-[rgba(77,70,53,0.15)] bg-surface-lowest p-3 text-left transition-colors hover:border-primary/30 hover:bg-primary/6"
      onClick={() => onAdd(template.ref_name)}
      type="button"
    >
      <div className="flex items-center justify-between gap-2">
        <span className="truncate text-sm font-medium text-on-surface">
          {template.display_name}
        </span>
        <span
          className="shrink-0 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.16em]"
          style={{ backgroundColor: colors.bg, color: colors.text, border: `1px solid ${colors.border}` }}
        >
          {template.node_type}
        </span>
      </div>

      {template.description && (
        <p className="mt-1.5 line-clamp-2 text-xs leading-5 text-on-surface-variant">
          {template.description}
        </p>
      )}

      {/* Port summary */}
      <div className="mt-2 flex flex-wrap gap-1">
        {inputs.map((p) => (
          <span
            className="px-1.5 py-0.5 text-[9px] font-medium"
            key={`in-${p.name}`}
            style={{
              backgroundColor: `${portTypeColors[p.port_type] ?? '#64748b'}22`,
              color: portTypeColors[p.port_type] ?? '#94a3b8',
            }}
          >
            {p.name}
          </span>
        ))}
        {outputs.map((p) => (
          <span
            className="border border-dashed px-1.5 py-0.5 text-[9px] font-medium"
            key={`out-${p.name}`}
            style={{
              borderColor: `${portTypeColors[p.port_type] ?? '#64748b'}44`,
              color: portTypeColors[p.port_type] ?? '#94a3b8',
            }}
          >
            {p.name}
          </span>
        ))}
      </div>
    </button>
  );
}
