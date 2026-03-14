import { useState } from 'react';
import type { ActorListEntry, ZoneBounds, DbEntity } from '../lib/types';
import { ViewportPanel } from './ViewportPanel';

type Slot = 'front' | 'side' | 'top' | 'perspective';
const SLOTS: Slot[] = ['front', 'side', 'top', 'perspective'];

interface ViewportContainerProps {
  actors: ActorListEntry[];
  bounds: ZoneBounds | null;
  selectedKeys: Set<string>;
  showGrid: boolean;
  focusRequest: number;
  transformMode: 'move' | 'rotate' | 'scale';
  gridSnap: number;
  onSelect: (key: string | null) => void;
  onToggleSelect: (key: string) => void;
  onAddSelect: (key: string) => void;
  onBoxSelect: (keys: string[]) => void;
  onMouseMove: (pos: { x: number; y: number; z: number }) => void;
  onSelectAll: () => void;
  onFocusSelected: () => void;
  onMoveActors: (keys: string[], newX: number[], newY: number[], newZ: number[]) => void;
  onRotateActors: (keys: string[], newPitch: number[], newYaw: number[], newRoll: number[]) => void;
  onScaleActors: (keys: string[], newDS: number[], newDSX: number[], newDSY: number[], newDSZ: number[]) => void;
  onDeleteActors: () => void;
  onDuplicateActors: () => void;
  onSelectSameClass: () => void;
  onInvertSelection: () => void;
  dbEntities?: DbEntity[];
  selectedDbEntityIds?: Set<number>;
}

export function ViewportContainer({
  actors,
  bounds,
  selectedKeys,
  showGrid,
  focusRequest,
  transformMode,
  gridSnap,
  onSelect,
  onToggleSelect,
  onAddSelect,
  onBoxSelect,
  onMouseMove,
  onSelectAll,
  onFocusSelected,
  onMoveActors,
  onRotateActors,
  onScaleActors,
  onDeleteActors,
  onDuplicateActors,
  onSelectSameClass,
  onInvertSelection,
  dbEntities,
  selectedDbEntityIds,
}: ViewportContainerProps) {
  // null = show all 4, string = maximized viewport
  const [maximized, setMaximized] = useState<Slot | null>(null);

  const toggleMaximize = (slot: Slot) => {
    setMaximized(prev => (prev === slot ? null : slot));
  };

  if (maximized) {
    return (
      <div className="h-full w-full">
        <ViewportPanel
          type={maximized}
          actors={actors}
          bounds={bounds}
          selectedKeys={selectedKeys}
          showGrid={showGrid}
          focusRequest={focusRequest}
          transformMode={transformMode}
          gridSnap={gridSnap}
          isMaximized
          onSelect={onSelect}
          onToggleSelect={onToggleSelect}
          onAddSelect={onAddSelect}
          onBoxSelect={onBoxSelect}
          onMouseMove={onMouseMove}
          onMaximize={() => toggleMaximize(maximized)}
          onSelectAll={onSelectAll}
          onFocusSelected={onFocusSelected}
          onMoveActors={onMoveActors}
          onRotateActors={onRotateActors}
          onScaleActors={onScaleActors}
          onDeleteActors={onDeleteActors}
          onDuplicateActors={onDuplicateActors}
          onSelectSameClass={onSelectSameClass}
          onInvertSelection={onInvertSelection}
          dbEntities={dbEntities}
          selectedDbEntityIds={selectedDbEntityIds}
        />
      </div>
    );
  }

  return (
    <div className="grid h-full w-full grid-cols-2 grid-rows-2 gap-px bg-border">
      {SLOTS.map(slot => (
        <ViewportPanel
          key={slot}
          type={slot}
          actors={actors}
          bounds={bounds}
          selectedKeys={selectedKeys}
          showGrid={showGrid}
          focusRequest={focusRequest}
          transformMode={transformMode}
          gridSnap={gridSnap}
          onSelect={onSelect}
          onToggleSelect={onToggleSelect}
          onAddSelect={onAddSelect}
          onBoxSelect={onBoxSelect}
          onMouseMove={onMouseMove}
          onMaximize={() => toggleMaximize(slot)}
          onSelectAll={onSelectAll}
          onFocusSelected={onFocusSelected}
          onMoveActors={onMoveActors}
          onRotateActors={onRotateActors}
          onScaleActors={onScaleActors}
          onDeleteActors={onDeleteActors}
          onDuplicateActors={onDuplicateActors}
          onSelectSameClass={onSelectSameClass}
          onInvertSelection={onInvertSelection}
          dbEntities={dbEntities}
          selectedDbEntityIds={selectedDbEntityIds}
        />
      ))}
    </div>
  );
}
