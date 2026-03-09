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
    <section className="rounded-[32px] border border-white/8 bg-[rgba(9,18,28,0.8)] p-6 shadow-[0_24px_80px_rgba(0,0,0,0.22)]">
      <div className="mb-4">
        <p className="text-xs font-semibold uppercase tracking-[0.24em] text-[rgba(160,174,192,0.72)]">
          Script Node Palette
        </p>
        <p className="mt-2 max-w-3xl text-sm leading-6 text-[rgba(224,231,239,0.76)]">
          Browse available script nodes. Click a node to add it to the canvas.
        </p>
      </div>

      {/* Filters */}
      <div className="mb-4 grid gap-3 sm:grid-cols-[minmax(0,2fr)_minmax(0,1fr)]">
        <label className="rounded-[24px] border border-white/8 bg-[rgba(255,255,255,0.03)] px-4 py-3">
          <span className="text-[10px] font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
            Search
          </span>
          <input
            className="mt-2 w-full bg-transparent text-sm text-white outline-none placeholder:text-[rgba(160,174,192,0.56)]"
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
                className={`flex-1 rounded-[20px] border px-2 py-2 text-[10px] font-semibold uppercase tracking-[0.16em] transition-colors ${
                  active
                    ? 'border-[rgba(245,170,49,0.4)] bg-[rgba(245,170,49,0.18)] text-[#ffd38a]'
                    : 'border-white/8 bg-[rgba(255,255,255,0.03)] text-[rgba(224,231,239,0.72)] hover:bg-[rgba(255,255,255,0.06)]'
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
              className="rounded-[28px] border border-white/8 bg-[rgba(255,255,255,0.03)] p-5"
              key={category}
            >
              <p className="mb-3 text-xs font-semibold uppercase tracking-[0.22em] text-[rgba(160,174,192,0.72)]">
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
          <section className="rounded-[28px] border border-dashed border-white/10 bg-[rgba(255,255,255,0.02)] p-10 text-center text-sm text-[rgba(224,231,239,0.72)] 2xl:col-span-2">
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
      className="rounded-[20px] border border-white/8 bg-[rgba(255,255,255,0.02)] p-3 text-left transition-colors hover:border-[rgba(245,170,49,0.3)] hover:bg-[rgba(245,170,49,0.06)]"
      onClick={() => onAdd(template.ref_name)}
      type="button"
    >
      <div className="flex items-center justify-between gap-2">
        <span className="truncate text-sm font-medium text-white">
          {template.display_name}
        </span>
        <span
          className="shrink-0 rounded-full px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.16em]"
          style={{ backgroundColor: colors.bg, color: colors.text, border: `1px solid ${colors.border}` }}
        >
          {template.node_type}
        </span>
      </div>

      {template.description && (
        <p className="mt-1.5 line-clamp-2 text-xs leading-5 text-[rgba(224,231,239,0.64)]">
          {template.description}
        </p>
      )}

      {/* Port summary */}
      <div className="mt-2 flex flex-wrap gap-1">
        {inputs.map((p) => (
          <span
            className="rounded-full px-1.5 py-0.5 text-[9px] font-medium"
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
            className="rounded-full border border-dashed px-1.5 py-0.5 text-[9px] font-medium"
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
