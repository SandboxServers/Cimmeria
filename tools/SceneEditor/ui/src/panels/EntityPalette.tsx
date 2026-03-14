import { useState, useMemo } from 'react';
import { Search, Database, MapPin } from 'lucide-react';
import type { EntityTemplate } from '../lib/types';
import { DB_ENTITY_COLORS } from '../lib/types';

interface EntityPaletteProps {
  templates: EntityTemplate[];
  selectedTemplate: EntityTemplate | null;
  onSelectTemplate: (template: EntityTemplate | null) => void;
  connected: boolean;
  placementMode: boolean;
  onTogglePlacement: () => void;
}

export function EntityPalette({ templates, selectedTemplate, onSelectTemplate, connected, placementMode, onTogglePlacement }: EntityPaletteProps) {
  const [search, setSearch] = useState('');

  // Group templates by class, filtered by search
  const grouped = useMemo(() => {
    const lower = search.toLowerCase();
    const filtered = templates.filter(t => {
      if (!lower) return true;
      return (
        t.template_name.toLowerCase().includes(lower) ||
        t.class.toLowerCase().includes(lower) ||
        (t.name && t.name.toLowerCase().includes(lower))
      );
    });

    const map = new Map<string, EntityTemplate[]>();
    for (const t of filtered) {
      let list = map.get(t.class);
      if (!list) {
        list = [];
        map.set(t.class, list);
      }
      list.push(t);
    }
    // Sort classes alphabetically, then templates by name within each class
    return [...map.entries()]
      .sort((a, b) => a[0].localeCompare(b[0]))
      .map(([cls, items]) => [cls, items.sort((a, b) => a.template_name.localeCompare(b.template_name))] as const);
  }, [templates, search]);

  if (!connected) {
    return (
      <div className="flex h-full flex-col bg-card">
        <div className="border-b px-3 py-1.5">
          <span className="text-xs font-medium text-foreground">Entity Palette</span>
        </div>
        <div className="flex flex-1 flex-col items-center justify-center gap-2 px-4">
          <Database size={24} className="text-muted-foreground/40" />
          <span className="text-center text-xs text-muted-foreground/60">
            Connect to database to browse entity templates
          </span>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col bg-card">
      {/* Header */}
      <div className="flex items-center justify-between border-b px-3 py-1.5">
        <span className="text-xs font-medium text-foreground">Entity Palette</span>
        <span className="font-mono text-[10px] text-muted-foreground">
          {templates.length}
        </span>
      </div>

      {/* Place button */}
      {selectedTemplate && (
        <div className="border-b px-2 py-1.5">
          <button
            onClick={onTogglePlacement}
            className={`flex w-full items-center justify-center gap-1.5 rounded px-2 py-1 text-xs font-medium transition-colors ${
              placementMode
                ? 'bg-primary text-primary-foreground'
                : 'bg-muted text-muted-foreground hover:bg-muted/80 hover:text-foreground'
            }`}
          >
            <MapPin size={12} />
            {placementMode ? 'Placing: ' : 'Place: '}{selectedTemplate.template_name}
          </button>
          {placementMode && (
            <p className="mt-1 text-center text-[10px] text-muted-foreground">
              Click in viewport to place. Esc to cancel.
            </p>
          )}
        </div>
      )}

      {/* Search */}
      <div className="relative border-b px-2 py-1">
        <Search size={12} className="absolute left-3.5 top-2.5 text-muted-foreground" />
        <input
          type="text"
          className="w-full rounded bg-input py-1 pl-6 pr-2 text-xs text-foreground placeholder:text-muted-foreground/60 outline-none"
          placeholder="Filter templates..."
          value={search}
          onChange={e => setSearch(e.target.value)}
        />
      </div>

      {/* Template list */}
      <div className="flex-1 overflow-y-auto subtle-scrollbar">
        {grouped.map(([className, classTemplates]) => {
          const color = DB_ENTITY_COLORS[className] ?? DB_ENTITY_COLORS._default;
          return (
            <div key={className}>
              {/* Class header */}
              <div className="flex items-center gap-1.5 px-2 py-1 text-xs">
                <span
                  className="inline-block h-2 w-2 shrink-0 rotate-45"
                  style={{ backgroundColor: color }}
                />
                <span className="font-medium text-foreground">{className}</span>
                <span className="ml-auto font-mono text-[10px] text-muted-foreground">
                  {classTemplates.length}
                </span>
              </div>

              {/* Templates */}
              <div className="pl-4">
                {classTemplates.map(template => {
                  const isSelected = selectedTemplate?.template_id === template.template_id;
                  return (
                    <button
                      key={template.template_id}
                      className={`flex w-full items-center gap-1.5 rounded-sm px-2 py-0.5 text-left text-[11px] ${
                        isSelected
                          ? 'bg-primary/20 text-primary'
                          : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                      }`}
                      onClick={() => onSelectTemplate(isSelected ? null : template)}
                    >
                      <span className="truncate">{template.template_name}</span>
                      {template.level != null && (
                        <span className="ml-auto shrink-0 text-[9px] text-muted-foreground/60">
                          Lv{template.level}
                        </span>
                      )}
                    </button>
                  );
                })}
              </div>
            </div>
          );
        })}

        {grouped.length === 0 && (
          <div className="px-3 py-6 text-center text-xs text-muted-foreground/60">
            {search ? 'No templates match filter' : 'No templates loaded'}
          </div>
        )}
      </div>
    </div>
  );
}
