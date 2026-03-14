import { useState, useCallback } from 'react';
import type { ActorListEntry, ViewportType, ZoneBounds, RenderMode, DbEntity } from '../lib/types';
import { ViewportToolbar } from './ViewportToolbar';
import { ViewportCanvas } from './ViewportCanvas';
import { Viewport3D } from './Viewport3D';
import { ContextMenu, buildActorContextMenu } from './ContextMenu';

interface ViewportPanelProps {
  type: ViewportType;
  actors: ActorListEntry[];
  bounds: ZoneBounds | null;
  selectedKeys: Set<string>;
  showGrid: boolean;
  focusRequest: number;
  transformMode: 'move' | 'rotate' | 'scale';
  gridSnap: number;
  isMaximized?: boolean;
  onSelect: (key: string | null) => void;
  onToggleSelect: (key: string) => void;
  onAddSelect: (key: string) => void;
  onBoxSelect: (keys: string[]) => void;
  onMouseMove: (pos: { x: number; y: number; z: number }) => void;
  onMaximize: () => void;
  onSelectAll: () => void;
  onFocusSelected: () => void;
  onMoveActors: (keys: string[], newX: number[], newY: number[], newZ: number[]) => void;
  onRotateActors: (keys: string[], newPitch: number[], newYaw: number[], newRoll: number[]) => void;
  onScaleActors: (keys: string[], newDS: number[], newDSX: number[], newDSY: number[], newDSZ: number[]) => void;
  onDeleteActors?: () => void;
  onDuplicateActors?: () => void;
  onSelectSameClass?: () => void;
  onInvertSelection?: () => void;
  dbEntities?: DbEntity[];
  selectedDbEntityIds?: Set<number>;
  placementMode?: boolean;
  onPlaceSpawn?: (x: number, y: number, z: number) => void;
  selectedTemplateName?: string | null;
  onSelectDbEntity?: (id: number) => void;
  onUpdateSpawnPosition?: (spawnId: number, x: number, y: number, z: number, heading: number) => void;
}

export function ViewportPanel({
  type: initialType,
  actors,
  bounds,
  selectedKeys,
  showGrid,
  focusRequest,
  transformMode,
  gridSnap,
  isMaximized,
  onSelect,
  onToggleSelect,
  onAddSelect,
  onBoxSelect,
  onMouseMove,
  onMaximize,
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
  placementMode,
  onPlaceSpawn,
  selectedTemplateName,
  onSelectDbEntity,
  onUpdateSpawnPosition,
}: ViewportPanelProps) {
  const [viewType, setViewType] = useState<ViewportType>(initialType);
  const [realtime, setRealtime] = useState(false);
  const [renderMode, setRenderMode] = useState<RenderMode>('unlit');
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);

  const is3D = viewType === 'perspective';

  const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    if (!is3D) {
      setContextMenu({ x: e.clientX, y: e.clientY });
    }
  }, [is3D]);

  // 3D viewport fires this when RMB is clicked without orbit drag
  const handle3DContextMenu = useCallback((x: number, y: number) => {
    setContextMenu({ x, y });
  }, []);

  return (
    <div className="flex h-full flex-col bg-background">
      <ViewportToolbar
        type={viewType}
        realtime={realtime}
        renderMode={renderMode}
        isMaximized={isMaximized}
        onTypeChange={setViewType}
        onRealtimeChange={setRealtime}
        onRenderModeChange={setRenderMode}
        onMaximize={onMaximize}
      />
      <div className="relative flex-1" onContextMenu={handleContextMenu}>
        {is3D ? (
          <Viewport3D
            actors={actors}
            bounds={bounds}
            selectedKeys={selectedKeys}
            showGrid={showGrid}
            renderMode={renderMode}
            focusRequest={focusRequest}
            transformMode={transformMode}
            gridSnap={gridSnap}
            onSelect={onSelect}
            onToggleSelect={onToggleSelect}
            onAddSelect={onAddSelect}
            onMouseMove={onMouseMove}
            onMoveActors={onMoveActors}
            onRotateActors={onRotateActors}
            onScaleActors={onScaleActors}
            onBoxSelect={onBoxSelect}
            onContextMenu={handle3DContextMenu}
            placementMode={placementMode}
            onPlaceSpawn={onPlaceSpawn}
            selectedTemplateName={selectedTemplateName}
            dbEntities={dbEntities}
            selectedDbEntityIds={selectedDbEntityIds}
            onSelectDbEntity={onSelectDbEntity}
            onUpdateSpawnPosition={onUpdateSpawnPosition}
          />
        ) : (
          <ViewportCanvas
            type={viewType}
            actors={actors}
            bounds={bounds}
            selectedKeys={selectedKeys}
            showGrid={showGrid}
            focusRequest={focusRequest}
            onSelect={onSelect}
            onToggleSelect={onToggleSelect}
            onAddSelect={onAddSelect}
            onBoxSelect={onBoxSelect}
            onMouseMove={onMouseMove}
            dbEntities={dbEntities}
            selectedDbEntityIds={selectedDbEntityIds}
          />
        )}
      </div>

      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          actions={buildActorContextMenu({
            hasSelection: selectedKeys.size > 0,
            selectionCount: selectedKeys.size,
            onFocus: onFocusSelected,
            onSelectAll,
            onDeselectAll: () => onSelect(null),
            onSelectSameClass,
            onInvertSelection,
            onCopy: onDuplicateActors,
            onDelete: onDeleteActors,
          })}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  );
}
