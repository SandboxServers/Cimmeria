import {
  FolderOpen,
  Move,
  RotateCw,
  Maximize2,
  Globe,
  Box,
  Grid2x2,
  Grid3x3,
} from 'lucide-react';

interface MainToolbarProps {
  transformMode: 'move' | 'rotate' | 'scale';
  coordSystem: 'world' | 'local';
  showGrid: boolean;
  gridSnap: number;
  selectionCount: number;
  onTransformChange: (mode: 'move' | 'rotate' | 'scale') => void;
  onCoordSystemChange: (system: 'world' | 'local') => void;
  onToggleGrid: () => void;
  onGridSnapChange: (snap: number) => void;
  onOpenZone: () => void;
}

export function MainToolbar({
  transformMode,
  coordSystem,
  showGrid,
  gridSnap,
  selectionCount,
  onTransformChange,
  onCoordSystemChange,
  onToggleGrid,
  onGridSnapChange,
  onOpenZone,
}: MainToolbarProps) {
  return (
    <div className="flex h-9 shrink-0 items-center gap-0.5 border-b bg-card px-2 select-none">
      {/* File ops */}
      <ToolButton icon={FolderOpen} tooltip="Open Zone" onClick={onOpenZone} />

      <ToolDivider />

      {/* Transform mode */}
      <ToolButton
        icon={Move}
        tooltip="Move (W)"
        active={transformMode === 'move'}
        onClick={() => onTransformChange('move')}
      />
      <ToolButton
        icon={RotateCw}
        tooltip="Rotate (E)"
        active={transformMode === 'rotate'}
        onClick={() => onTransformChange('rotate')}
      />
      <ToolButton
        icon={Maximize2}
        tooltip="Scale (R)"
        active={transformMode === 'scale'}
        onClick={() => onTransformChange('scale')}
      />

      <ToolDivider />

      {/* Coord system */}
      <ToolButton
        icon={coordSystem === 'world' ? Globe : Box}
        tooltip={`Coord: ${coordSystem === 'world' ? 'World' : 'Local'}`}
        onClick={() => onCoordSystemChange(coordSystem === 'world' ? 'local' : 'world')}
        label={coordSystem === 'world' ? 'World' : 'Local'}
      />

      <ToolDivider />

      {/* View toggles */}
      <ToolButton
        icon={Grid3x3}
        tooltip="Toggle Grid"
        active={showGrid}
        onClick={onToggleGrid}
      />
      {/* Grid snap — context-sensitive to transform mode */}
      <select
        className="h-6 rounded bg-muted px-1 text-[11px] text-foreground outline-none"
        value={gridSnap}
        onChange={e => onGridSnapChange(Number(e.target.value))}
        title={transformMode === 'rotate' ? 'Rotation Snap (degrees)' : 'Grid Snap'}
      >
        {transformMode === 'rotate' ? (
          <>
            <option value={1}>Snap: 1&deg;</option>
            <option value={5}>Snap: 5&deg;</option>
            <option value={10}>Snap: 10&deg;</option>
            <option value={15}>Snap: 15&deg;</option>
            <option value={30}>Snap: 30&deg;</option>
            <option value={45}>Snap: 45&deg;</option>
            <option value={90}>Snap: 90&deg;</option>
          </>
        ) : (
          <>
            <option value={1}>Snap: 1</option>
            <option value={5}>Snap: 5</option>
            <option value={10}>Snap: 10</option>
            <option value={25}>Snap: 25</option>
            <option value={50}>Snap: 50</option>
            <option value={100}>Snap: 100</option>
          </>
        )}
      </select>

      {/* Spacer */}
      <div className="flex-1" />

      {/* Selection info */}
      {selectionCount > 0 && (
        <span className="text-[11px] text-muted-foreground">
          {selectionCount} selected
        </span>
      )}
    </div>
  );
}

// ---- Sub-components ----

function ToolButton({
  icon: Icon,
  tooltip,
  active,
  onClick,
  label,
}: {
  icon: React.ComponentType<{ size?: number; className?: string }>;
  tooltip: string;
  active?: boolean;
  onClick: () => void;
  label?: string;
}) {
  return (
    <button
      className={`flex items-center gap-1 rounded px-1.5 py-1 text-xs transition-colors ${
        active
          ? 'bg-primary/20 text-primary'
          : 'text-muted-foreground hover:bg-muted hover:text-foreground'
      }`}
      onClick={onClick}
      title={tooltip}
    >
      <Icon size={16} />
      {label && <span className="text-[11px]">{label}</span>}
    </button>
  );
}

function ToolDivider() {
  return <div className="mx-1 h-5 w-px bg-border" />;
}
