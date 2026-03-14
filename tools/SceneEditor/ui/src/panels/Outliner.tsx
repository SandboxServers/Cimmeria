import { useState, useMemo, useRef, useEffect, useCallback } from 'react';
import { ChevronRight, ChevronDown, Search } from 'lucide-react';
import type { ActorListEntry } from '../lib/types';
import { ACTOR_COLORS } from '../lib/types';

interface OutlinerProps {
  actors: ActorListEntry[];
  classFilter: Set<string>;
  selectedKeys: Set<string>;
  onSelect: (key: string | null) => void;
  onToggleSelect: (key: string) => void;
  onAddSelect: (key: string) => void;
  onBoxSelect: (keys: string[]) => void;
  onFocusSelected: () => void;
}

/** Maximum actors to render in an expanded class group to avoid DOM overload. */
const MAX_VISIBLE_PER_CLASS = 200;

export function Outliner({ actors, classFilter, selectedKeys, onSelect, onToggleSelect, onAddSelect, onBoxSelect, onFocusSelected }: OutlinerProps) {
  const [search, setSearch] = useState('');
  const [expandedClasses, setExpandedClasses] = useState<Set<string>>(new Set());
  const selectedRef = useRef<HTMLDivElement>(null);
  const lastClickedKeyRef = useRef<string | null>(null);

  // O(1) actor lookup map
  const actorByKey = useMemo(() => {
    const m = new Map<string, ActorListEntry>();
    for (const a of actors) m.set(a.key, a);
    return m;
  }, [actors]);

  // Group actors by class
  const grouped = useMemo(() => {
    const map = new Map<string, ActorListEntry[]>();
    for (const actor of actors) {
      if (!classFilter.has(actor.class_name)) continue;
      const lower = search.toLowerCase();
      if (lower && !actor.object_name.toLowerCase().includes(lower) && !actor.class_name.toLowerCase().includes(lower)) {
        continue;
      }
      let list = map.get(actor.class_name);
      if (!list) {
        list = [];
        map.set(actor.class_name, list);
      }
      list.push(actor);
    }
    // Sort classes by count descending
    return [...map.entries()].sort((a, b) => b[1].length - a[1].length);
  }, [actors, classFilter, search]);

  // Flat list of visible actor keys (for range select)
  const flatKeys = useMemo(() => {
    const keys: string[] = [];
    for (const [className, classActors] of grouped) {
      if (expandedClasses.has(className)) {
        for (const a of classActors.slice(0, MAX_VISIBLE_PER_CLASS)) {
          keys.push(a.key);
        }
      }
    }
    return keys;
  }, [grouped, expandedClasses]);

  // Scroll to selected actor when selection changes
  useEffect(() => {
    if (selectedRef.current) {
      selectedRef.current.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    }
  }, [selectedKeys]);

  // Auto-expand the class group containing the last selected actor
  useEffect(() => {
    if (selectedKeys.size === 0) return;
    const lastKey = [...selectedKeys][selectedKeys.size - 1];
    const actor = actorByKey.get(lastKey);
    if (actor && !expandedClasses.has(actor.class_name)) {
      setExpandedClasses(prev => {
        const next = new Set(prev);
        next.add(actor.class_name);
        return next;
      });
    }
  }, [selectedKeys, actors, expandedClasses]);

  const toggleClass = (cls: string) => {
    setExpandedClasses(prev => {
      const next = new Set(prev);
      if (next.has(cls)) {
        next.delete(cls);
      } else {
        next.add(cls);
      }
      return next;
    });
  };

  const handleActorClick = useCallback((key: string, e: React.MouseEvent) => {
    if (e.ctrlKey || e.metaKey) {
      onToggleSelect(key);
      lastClickedKeyRef.current = key;
    } else if (e.shiftKey && lastClickedKeyRef.current) {
      // Range select: select all between lastClicked and current in flat order
      const startIdx = flatKeys.indexOf(lastClickedKeyRef.current);
      const endIdx = flatKeys.indexOf(key);
      if (startIdx >= 0 && endIdx >= 0) {
        const lo = Math.min(startIdx, endIdx);
        const hi = Math.max(startIdx, endIdx);
        const rangeKeys = flatKeys.slice(lo, hi + 1);
        onBoxSelect(rangeKeys);
      } else {
        onAddSelect(key);
      }
    } else {
      onSelect(key);
      lastClickedKeyRef.current = key;
    }
  }, [flatKeys, onSelect, onToggleSelect, onAddSelect, onBoxSelect]);

  const handleActorDoubleClick = useCallback((key: string) => {
    onSelect(key);
    onFocusSelected();
  }, [onSelect, onFocusSelected]);

  // Select all actors in a class group (Ctrl+click header)
  const handleClassHeaderClick = useCallback((className: string, classActors: ActorListEntry[], e: React.MouseEvent) => {
    if (e.ctrlKey || e.metaKey) {
      // Select all actors in this class
      onBoxSelect(classActors.map(a => a.key));
    } else {
      toggleClass(className);
    }
  }, [onBoxSelect]);

  const totalVisible = grouped.reduce((sum, [, list]) => sum + list.length, 0);

  return (
    <div className="flex h-full flex-col bg-card">
      {/* Header */}
      <div className="flex items-center justify-between border-b px-3 py-1.5">
        <span className="text-xs font-medium text-foreground">Outliner</span>
        <span className="font-mono text-[10px] text-muted-foreground">
          {totalVisible.toLocaleString()}
        </span>
      </div>

      {/* Search */}
      <div className="relative border-b px-2 py-1">
        <Search size={12} className="absolute left-3.5 top-2.5 text-muted-foreground" />
        <input
          type="text"
          className="w-full rounded bg-input py-1 pl-6 pr-2 text-xs text-foreground placeholder:text-muted-foreground/60 outline-none"
          placeholder="Filter actors..."
          value={search}
          onChange={e => setSearch(e.target.value)}
        />
      </div>

      {/* Tree */}
      <div className="flex-1 overflow-y-auto subtle-scrollbar">
        {grouped.map(([className, classActors]) => {
          const isExpanded = expandedClasses.has(className);
          const color = ACTOR_COLORS[className] ?? ACTOR_COLORS._default;
          const displayActors = classActors.slice(0, MAX_VISIBLE_PER_CLASS);
          const hasMore = classActors.length > MAX_VISIBLE_PER_CLASS;
          const selectedInClass = classActors.filter(a => selectedKeys.has(a.key)).length;

          return (
            <div key={className}>
              {/* Class header */}
              <button
                className="flex w-full items-center gap-1 px-2 py-0.5 text-left text-xs hover:bg-muted"
                onClick={(e) => handleClassHeaderClick(className, classActors, e)}
                title="Click to expand/collapse, Ctrl+click to select all in class"
              >
                {isExpanded ? (
                  <ChevronDown size={12} className="shrink-0 text-muted-foreground" />
                ) : (
                  <ChevronRight size={12} className="shrink-0 text-muted-foreground" />
                )}
                <span
                  className="mr-1 inline-block h-2 w-2 shrink-0 rounded-full"
                  style={{ backgroundColor: color }}
                />
                <span className="truncate text-foreground">{className}</span>
                {selectedInClass > 0 && (
                  <span className="ml-1 shrink-0 rounded-sm bg-primary/20 px-1 text-[9px] text-primary">
                    {selectedInClass}
                  </span>
                )}
                <span className="ml-auto shrink-0 font-mono text-[10px] text-muted-foreground">
                  {classActors.length.toLocaleString()}
                </span>
              </button>

              {/* Actor list */}
              {isExpanded && (
                <div className="pl-5">
                  {displayActors.map(actor => {
                    const isSelected = selectedKeys.has(actor.key);
                    const isLast = isSelected && [...selectedKeys][selectedKeys.size - 1] === actor.key;
                    return (
                      <div
                        key={actor.key}
                        ref={isLast ? selectedRef : undefined}
                        className={`flex cursor-pointer items-center gap-1.5 rounded-sm px-2 py-px text-[11px] ${
                          isSelected
                            ? 'bg-primary/20 text-primary'
                            : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                        }`}
                        onClick={e => handleActorClick(actor.key, e)}
                        onDoubleClick={() => handleActorDoubleClick(actor.key)}
                      >
                        <span className="truncate">{actor.object_name}</span>
                        <span className="ml-auto shrink-0 text-[9px] text-muted-foreground/60">
                          [{actor.tile}]
                        </span>
                      </div>
                    );
                  })}
                  {hasMore && (
                    <div className="px-2 py-0.5 text-[10px] italic text-muted-foreground/50">
                      ... and {(classActors.length - MAX_VISIBLE_PER_CLASS).toLocaleString()} more
                    </div>
                  )}
                </div>
              )}
            </div>
          );
        })}

        {grouped.length === 0 && (
          <div className="px-3 py-6 text-center text-xs text-muted-foreground/60">
            {search ? 'No actors match filter' : 'No actors loaded'}
          </div>
        )}
      </div>
    </div>
  );
}
