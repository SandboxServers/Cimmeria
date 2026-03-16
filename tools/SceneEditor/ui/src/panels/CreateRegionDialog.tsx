import { useState } from 'react';
import { X } from 'lucide-react';
import { invoke } from '../lib/tauri';
import type { DbEntity, WorldInfo } from '../lib/types';

interface CreateRegionDialogProps {
  position: { x: number; y: number; z: number };
  worlds: WorldInfo[];
  currentWorldId: number | null;
  onCreated: (entity: DbEntity) => void;
  onClose: () => void;
}

export function CreateRegionDialog({
  position,
  worlds,
  currentWorldId,
  onCreated,
  onClose,
}: CreateRegionDialogProps) {
  const [radius, setRadius] = useState(500);
  const [height, setHeight] = useState(300);
  const [handler, setHandler] = useState('OnEnterRegion');
  const [tag, setTag] = useState('');
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const worldId = currentWorldId ?? worlds[0]?.world_id ?? 0;

  const handleCreate = async () => {
    if (!handler.trim()) {
      setError('Handler is required');
      return;
    }
    setCreating(true);
    setError(null);
    try {
      const entity = await invoke<DbEntity>('create_region', {
        worldId,
        x: position.x,
        y: position.y,
        z: position.z,
        radius,
        height,
        handler: handler.trim(),
        tag: tag.trim() || null,
      });
      onCreated(entity);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-96 rounded-lg border bg-card p-4 shadow-xl">
        <div className="mb-3 flex items-center justify-between">
          <h3 className="text-sm font-medium text-foreground">Create Region</h3>
          <button onClick={onClose} className="text-muted-foreground hover:text-foreground">
            <X size={16} />
          </button>
        </div>

        <div className="space-y-3">
          {/* Position (read-only) */}
          <div>
            <label className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
              Position (UE3 cm)
            </label>
            <div className="flex gap-2 font-mono text-xs text-foreground">
              <span>X: {position.x.toFixed(0)}</span>
              <span>Y: {position.y.toFixed(0)}</span>
              <span>Z: {position.z.toFixed(0)}</span>
            </div>
          </div>

          {/* Radius */}
          <div>
            <label className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
              Radius (cm)
            </label>
            <input
              type="number"
              value={radius}
              onChange={e => setRadius(Number(e.target.value))}
              className="w-full rounded bg-input px-2 py-1 text-xs text-foreground outline-none"
            />
          </div>

          {/* Height */}
          <div>
            <label className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
              Height (cm)
            </label>
            <input
              type="number"
              value={height}
              onChange={e => setHeight(Number(e.target.value))}
              className="w-full rounded bg-input px-2 py-1 text-xs text-foreground outline-none"
            />
          </div>

          {/* Handler */}
          <div>
            <label className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
              Handler
            </label>
            <input
              type="text"
              value={handler}
              onChange={e => setHandler(e.target.value)}
              placeholder="OnEnterRegion"
              className="w-full rounded bg-input px-2 py-1 text-xs text-foreground placeholder:text-muted-foreground/50 outline-none"
            />
          </div>

          {/* Tag */}
          <div>
            <label className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
              Tag (optional)
            </label>
            <input
              type="text"
              value={tag}
              onChange={e => setTag(e.target.value)}
              placeholder="region_01"
              className="w-full rounded bg-input px-2 py-1 text-xs text-foreground placeholder:text-muted-foreground/50 outline-none"
            />
          </div>

          {error && (
            <p className="text-xs text-destructive">{error}</p>
          )}

          <div className="flex justify-end gap-2 pt-1">
            <button
              onClick={onClose}
              className="rounded px-3 py-1 text-xs text-muted-foreground hover:bg-muted"
            >
              Cancel
            </button>
            <button
              onClick={handleCreate}
              disabled={creating}
              className="rounded bg-primary px-3 py-1 text-xs text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
            >
              {creating ? 'Creating...' : 'Create Region'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
