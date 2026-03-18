import { useState } from 'react';
import { X, Grid3X3, Circle, Waves } from 'lucide-react';
import { invoke } from '../lib/tauri';
import type { ActorListEntry } from '../lib/types';

type LayoutMode = 'grid' | 'circle' | 'spiral';

interface ProceduralPlacementDialogProps {
  /** The actor key to duplicate for placement */
  sourceKey: string | null;
  onPlaced: (newActors: ActorListEntry[]) => void;
  onClose: () => void;
}

export function ProceduralPlacementDialog({ sourceKey, onPlaced, onClose }: ProceduralPlacementDialogProps) {
  const [mode, setMode] = useState<LayoutMode>('grid');
  const [placing, setPlacing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Grid params
  const [spacingX, setSpacingX] = useState(200);
  const [spacingY, setSpacingY] = useState(200);
  const [rows, setRows] = useState(3);
  const [cols, setCols] = useState(3);

  // Circle params
  const [centerX, setCenterX] = useState(0);
  const [centerY, setCenterY] = useState(0);
  const [centerZ, setCenterZ] = useState(0);
  const [radius, setRadius] = useState(500);
  const [circleCount, setCircleCount] = useState(8);
  const [startAngle, setStartAngle] = useState(0);

  // Spiral params
  const [startRadius, setStartRadius] = useState(100);
  const [endRadius, setEndRadius] = useState(800);
  const [turns, setTurns] = useState(2);
  const [spiralCount, setSpiralCount] = useState(20);

  if (!sourceKey) {
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
        <div className="w-96 rounded-lg border bg-card p-4 shadow-xl">
          <div className="mb-3 flex items-center justify-between">
            <h3 className="text-sm font-medium text-foreground">Procedural Placement</h3>
            <button onClick={onClose} className="text-muted-foreground hover:text-foreground"><X size={16} /></button>
          </div>
          <p className="text-xs text-muted-foreground">Select an actor first, then open this dialog to place copies in a pattern.</p>
        </div>
      </div>
    );
  }

  const handlePlace = async () => {
    setPlacing(true);
    setError(null);
    try {
      let result: ActorListEntry[];
      if (mode === 'grid') {
        result = await invoke<ActorListEntry[]>('place_grid', {
          sourceKey, spacingX, spacingY, rows, cols,
        });
      } else if (mode === 'circle') {
        result = await invoke<ActorListEntry[]>('place_circle', {
          sourceKey, centerX, centerY, centerZ, radius, count: circleCount, startAngle,
        });
      } else {
        result = await invoke<ActorListEntry[]>('place_spiral', {
          sourceKey, centerX, centerY, centerZ, startRadius, endRadius, turns, count: spiralCount,
        });
      }
      onPlaced(result);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setPlacing(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-[420px] rounded-lg border bg-card p-4 shadow-xl">
        {/* Header */}
        <div className="mb-3 flex items-center justify-between">
          <h3 className="text-sm font-medium text-foreground">Procedural Placement</h3>
          <button onClick={onClose} className="text-muted-foreground hover:text-foreground"><X size={16} /></button>
        </div>

        {/* Mode selector */}
        <div className="mb-3 flex gap-1">
          {([
            { id: 'grid' as LayoutMode, label: 'Grid', icon: <Grid3X3 size={14} /> },
            { id: 'circle' as LayoutMode, label: 'Circle', icon: <Circle size={14} /> },
            { id: 'spiral' as LayoutMode, label: 'Spiral', icon: <Waves size={14} /> },
          ]).map(({ id, label, icon }) => (
            <button
              key={id}
              onClick={() => setMode(id)}
              className={`flex flex-1 items-center justify-center gap-1.5 rounded px-2 py-1.5 text-xs font-medium ${
                mode === id
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-muted text-muted-foreground hover:text-foreground'
              }`}
            >
              {icon} {label}
            </button>
          ))}
        </div>

        {/* Source */}
        <div className="mb-3 rounded bg-muted/50 px-3 py-1.5 text-[10px] text-muted-foreground">
          Source: <span className="font-mono text-foreground">{sourceKey}</span>
        </div>

        {/* Params */}
        <div className="space-y-2">
          {mode === 'grid' && (
            <>
              <div className="grid grid-cols-2 gap-2">
                <ParamField label="Spacing X (cm)" value={spacingX} onChange={setSpacingX} />
                <ParamField label="Spacing Y (cm)" value={spacingY} onChange={setSpacingY} />
              </div>
              <div className="grid grid-cols-2 gap-2">
                <ParamField label="Rows" value={rows} onChange={setRows} />
                <ParamField label="Columns" value={cols} onChange={setCols} />
              </div>
              <p className="text-[10px] text-muted-foreground">
                Will place {rows * cols} actors in a {rows}x{cols} grid
              </p>
            </>
          )}

          {mode === 'circle' && (
            <>
              <div className="grid grid-cols-3 gap-2">
                <ParamField label="Center X" value={centerX} onChange={setCenterX} />
                <ParamField label="Center Y" value={centerY} onChange={setCenterY} />
                <ParamField label="Center Z" value={centerZ} onChange={setCenterZ} />
              </div>
              <div className="grid grid-cols-3 gap-2">
                <ParamField label="Radius (cm)" value={radius} onChange={setRadius} />
                <ParamField label="Count" value={circleCount} onChange={setCircleCount} />
                <ParamField label="Start Angle" value={startAngle} onChange={setStartAngle} />
              </div>
              <p className="text-[10px] text-muted-foreground">
                Will place {circleCount} actors in a circle of radius {radius}cm
              </p>
            </>
          )}

          {mode === 'spiral' && (
            <>
              <div className="grid grid-cols-3 gap-2">
                <ParamField label="Center X" value={centerX} onChange={setCenterX} />
                <ParamField label="Center Y" value={centerY} onChange={setCenterY} />
                <ParamField label="Center Z" value={centerZ} onChange={setCenterZ} />
              </div>
              <div className="grid grid-cols-2 gap-2">
                <ParamField label="Start Radius" value={startRadius} onChange={setStartRadius} />
                <ParamField label="End Radius" value={endRadius} onChange={setEndRadius} />
              </div>
              <div className="grid grid-cols-2 gap-2">
                <ParamField label="Turns" value={turns} onChange={setTurns} />
                <ParamField label="Count" value={spiralCount} onChange={setSpiralCount} />
              </div>
              <p className="text-[10px] text-muted-foreground">
                Will place {spiralCount} actors spiraling from {startRadius}cm to {endRadius}cm
              </p>
            </>
          )}
        </div>

        {error && <p className="mt-2 text-xs text-destructive">{error}</p>}

        <div className="mt-3 flex justify-end gap-2">
          <button onClick={onClose} className="rounded px-3 py-1 text-xs text-muted-foreground hover:bg-muted">Cancel</button>
          <button
            onClick={handlePlace}
            disabled={placing}
            className="rounded bg-primary px-3 py-1 text-xs text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            {placing ? 'Placing...' : `Place ${mode === 'grid' ? rows * cols : mode === 'circle' ? circleCount : spiralCount} Actors`}
          </button>
        </div>
      </div>
    </div>
  );
}

function ParamField({ label, value, onChange }: { label: string; value: number; onChange: (v: number) => void }) {
  return (
    <div>
      <label className="mb-0.5 block text-[10px] text-muted-foreground">{label}</label>
      <input
        type="number"
        value={value}
        onChange={e => onChange(Number(e.target.value))}
        className="w-full rounded bg-input px-2 py-1 text-xs text-foreground outline-none"
      />
    </div>
  );
}
