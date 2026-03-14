import { useState, useMemo } from 'react';
import { ChevronRight, ChevronDown, Trash2 } from 'lucide-react';
import type { DbEntity } from '../lib/types';
import { DB_ENTITY_COLORS } from '../lib/types';

interface DbEntityListProps {
  entities: DbEntity[];
  selectedIds: Set<number>;
  onSelect: (id: number) => void;
  onDelete: (id: number) => void;
}

/** Friendly labels for source_table values. */
const TABLE_LABELS: Record<string, string> = {
  spawnlist: 'Spawns',
  spawn_points: 'Spawn Points',
  generic_regions: 'Regions',
  stargates: 'Stargates',
};

export function DbEntityList({ entities, selectedIds, onSelect, onDelete }: DbEntityListProps) {
  const [expandedTables, setExpandedTables] = useState<Set<string>>(new Set(['spawnlist', 'spawn_points', 'generic_regions', 'stargates']));

  // Group entities by source_table
  const grouped = useMemo(() => {
    const map = new Map<string, DbEntity[]>();
    for (const entity of entities) {
      let list = map.get(entity.source_table);
      if (!list) {
        list = [];
        map.set(entity.source_table, list);
      }
      list.push(entity);
    }
    // Sort by table name for stable ordering
    return [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  }, [entities]);

  const toggleTable = (table: string) => {
    setExpandedTables(prev => {
      const next = new Set(prev);
      if (next.has(table)) next.delete(table);
      else next.add(table);
      return next;
    });
  };

  return (
    <div className="flex h-full flex-col bg-card">
      {/* Header */}
      <div className="flex items-center justify-between border-b px-3 py-1.5">
        <span className="text-xs font-medium text-foreground">DB Entities</span>
        <span className="font-mono text-[10px] text-muted-foreground">
          {entities.length}
        </span>
      </div>

      {/* Entity list */}
      <div className="flex-1 overflow-y-auto subtle-scrollbar">
        {grouped.map(([table, tableEntities]) => {
          const isExpanded = expandedTables.has(table);
          const label = TABLE_LABELS[table] ?? table;
          const selectedInTable = tableEntities.filter(e => selectedIds.has(e.id)).length;

          return (
            <div key={table}>
              {/* Table header */}
              <button
                className="flex w-full items-center gap-1 px-2 py-1 text-left text-xs hover:bg-muted"
                onClick={() => toggleTable(table)}
              >
                {isExpanded ? (
                  <ChevronDown size={12} className="shrink-0 text-muted-foreground" />
                ) : (
                  <ChevronRight size={12} className="shrink-0 text-muted-foreground" />
                )}
                <span className="font-medium text-foreground">{label}</span>
                {selectedInTable > 0 && (
                  <span className="ml-1 shrink-0 rounded-sm bg-primary/20 px-1 text-[9px] text-primary">
                    {selectedInTable}
                  </span>
                )}
                <span className="ml-auto shrink-0 rounded-full bg-muted px-1.5 py-0.5 font-mono text-[9px] text-muted-foreground">
                  {tableEntities.length}
                </span>
              </button>

              {/* Entity rows */}
              {isExpanded && (
                <div className="pl-4">
                  {tableEntities.map(entity => {
                    const isSelected = selectedIds.has(entity.id);
                    const color = DB_ENTITY_COLORS[entity.entity_type] ?? DB_ENTITY_COLORS._default;

                    return (
                      <div
                        key={entity.id}
                        className={`group flex cursor-pointer items-center gap-1.5 rounded-sm px-2 py-0.5 text-[11px] ${
                          isSelected
                            ? 'bg-primary/20 text-primary'
                            : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                        }`}
                        onClick={() => onSelect(entity.id)}
                      >
                        <span
                          className="inline-block h-2 w-2 shrink-0 rotate-45"
                          style={{ backgroundColor: color }}
                        />
                        <span className="truncate">
                          {entity.name || entity.entity_type}
                        </span>
                        <span className="ml-auto shrink-0 text-[9px] text-muted-foreground/60">
                          {entity.entity_type}
                        </span>
                        <button
                          className="ml-1 hidden shrink-0 rounded p-0.5 text-muted-foreground/50 hover:bg-destructive/10 hover:text-destructive group-hover:block"
                          onClick={e => {
                            e.stopPropagation();
                            onDelete(entity.id);
                          }}
                          title="Delete entity"
                        >
                          <Trash2 size={10} />
                        </button>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          );
        })}

        {entities.length === 0 && (
          <div className="px-3 py-6 text-center text-xs text-muted-foreground/60">
            No DB entities loaded
          </div>
        )}
      </div>
    </div>
  );
}
