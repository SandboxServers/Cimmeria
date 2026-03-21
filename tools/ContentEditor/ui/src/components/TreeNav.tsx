import { useState } from 'react';
import {
  FolderOpen,
  Folder,
  MapPin,
  ChevronRight,
  ChevronDown,
} from 'lucide-react';
import type { SpaceEntry } from './AppLayout';

interface TreeNavProps {
  spaces: SpaceEntry[];
  onSelectSpace: (spaceId: string) => void;
  activeSpaceId: string | null;
}

export function TreeNav({ spaces, onSelectSpace, activeSpaceId }: TreeNavProps) {
  const [expandedSections, setExpandedSections] = useState<Set<string>>(
    new Set(['spaces'])
  );

  const toggleSection = (key: string) => {
    setExpandedSections((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  return (
    <div className="subtle-scrollbar flex h-full flex-col overflow-y-auto border-r border-[rgba(77,70,53,0.15)] bg-surface-lowest">
      <div className="border-b border-[rgba(77,70,53,0.15)] px-3 py-2">
        <h2 className="font-headline text-primary tracking-[0.2em] uppercase text-xs">
          GATE_ADDRESSES
        </h2>
      </div>

      {/* Spaces section */}
      <div>
        <button
          onClick={() => toggleSection('spaces')}
          className="flex w-full items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-on-surface hover:bg-surface-highest hover:translate-x-1 transition-all duration-200"
        >
          {expandedSections.has('spaces') ? (
            <ChevronDown className="h-3.5 w-3.5 text-on-surface-variant" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-on-surface-variant" />
          )}
          {expandedSections.has('spaces') ? (
            <FolderOpen className="h-4 w-4 text-primary" />
          ) : (
            <Folder className="h-4 w-4 text-primary" />
          )}
          Spaces
        </button>

        {expandedSections.has('spaces') && (
          <div className="pb-1">
            {spaces.length === 0 && (
              <div className="px-8 py-1 text-xs text-on-surface-variant">
                No spaces found
              </div>
            )}
            {spaces.map((space) => (
              <button
                key={space.space_id}
                onClick={() => onSelectSpace(space.space_id)}
                className={`flex w-full items-center gap-2 px-6 py-1 text-sm transition-all duration-200 hover:bg-surface-highest hover:translate-x-1 ${
                  activeSpaceId === space.space_id
                    ? 'bg-secondary-container/30 text-primary shadow-[inset_4px_0_0_0_#F2CA50]'
                    : 'text-on-surface-variant'
                }`}
              >
                <MapPin className="h-3.5 w-3.5 shrink-0 text-tertiary" />
                <span className="truncate">{space.space_id}</span>
                <span className="ml-auto text-xs opacity-60">
                  {space.chain_count}
                </span>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
