import { useState, useEffect, useRef, useMemo } from 'react';
import { Search, X } from 'lucide-react';
import type { ActorListEntry } from '../lib/types';
import { ACTOR_COLORS } from '../lib/types';

interface ActorSearchProps {
  actors: ActorListEntry[];
  onSelect: (key: string) => void;
  onClose: () => void;
}

const MAX_RESULTS = 50;

export function ActorSearch({ actors, onSelect, onClose }: ActorSearchProps) {
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const results = useMemo(() => {
    if (!query.trim()) return [];
    const q = query.toLowerCase();
    const matches: ActorListEntry[] = [];
    for (const a of actors) {
      if (matches.length >= MAX_RESULTS) break;
      if (
        a.object_name.toLowerCase().includes(q) ||
        a.class_name.toLowerCase().includes(q) ||
        a.key.toLowerCase().includes(q)
      ) {
        matches.push(a);
      }
    }
    return matches;
  }, [actors, query]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [results]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setSelectedIndex(i => Math.min(i + 1, results.length - 1));
        break;
      case 'ArrowUp':
        e.preventDefault();
        setSelectedIndex(i => Math.max(i - 1, 0));
        break;
      case 'Enter':
        e.preventDefault();
        if (results[selectedIndex]) {
          onSelect(results[selectedIndex].key);
          onClose();
        }
        break;
      case 'Escape':
        onClose();
        break;
    }
  };

  // Scroll selected item into view
  useEffect(() => {
    const list = listRef.current;
    if (!list) return;
    const selected = list.children[selectedIndex] as HTMLElement | undefined;
    if (selected) {
      selected.scrollIntoView({ block: 'nearest' });
    }
  }, [selectedIndex]);

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[15%]">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/50" onClick={onClose} />

      {/* Dialog */}
      <div
        className="relative z-10 w-[500px] overflow-hidden rounded-lg border bg-card shadow-2xl"
        onKeyDown={handleKeyDown}
      >
        {/* Search input */}
        <div className="flex items-center gap-2 border-b px-3 py-2">
          <Search size={14} className="shrink-0 text-muted-foreground" />
          <input
            ref={inputRef}
            type="text"
            className="flex-1 bg-transparent text-sm text-foreground placeholder:text-muted-foreground/50 outline-none"
            placeholder="Search actors by name, class, or key..."
            value={query}
            onChange={e => setQuery(e.target.value)}
          />
          <button
            className="rounded p-0.5 text-muted-foreground hover:text-foreground"
            onClick={onClose}
          >
            <X size={14} />
          </button>
        </div>

        {/* Results */}
        <div ref={listRef} className="max-h-[300px] overflow-y-auto subtle-scrollbar">
          {results.length === 0 && query.trim() && (
            <div className="px-3 py-4 text-center text-xs text-muted-foreground/60">
              No actors match "{query}"
            </div>
          )}
          {results.map((actor, i) => {
            const color = ACTOR_COLORS[actor.class_name] ?? ACTOR_COLORS._default;
            return (
              <button
                key={actor.key}
                className={`flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs ${
                  i === selectedIndex
                    ? 'bg-primary/20 text-primary'
                    : 'text-foreground hover:bg-muted'
                }`}
                onClick={() => {
                  onSelect(actor.key);
                  onClose();
                }}
                onMouseEnter={() => setSelectedIndex(i)}
              >
                <span
                  className="inline-block h-2 w-2 shrink-0 rounded-full"
                  style={{ backgroundColor: color }}
                />
                <span className="min-w-0 truncate font-medium">{actor.object_name}</span>
                <span className="ml-auto shrink-0 text-[10px] text-muted-foreground">
                  {actor.class_name}
                </span>
                <span className="shrink-0 text-[9px] text-muted-foreground/50">
                  [{actor.tile}]
                </span>
              </button>
            );
          })}
          {results.length >= MAX_RESULTS && (
            <div className="px-3 py-1 text-center text-[10px] text-muted-foreground/50">
              Showing first {MAX_RESULTS} results...
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t px-3 py-1.5 text-[10px] text-muted-foreground">
          <span>{results.length > 0 ? `${results.length} matches` : 'Type to search'}</span>
          <span>Enter to select, Esc to close</span>
        </div>
      </div>
    </div>
  );
}
