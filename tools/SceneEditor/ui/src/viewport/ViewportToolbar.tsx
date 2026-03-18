import { Maximize2, Minimize2 } from 'lucide-react';
import type { ViewportType, RenderMode } from '../lib/types';

interface ViewportToolbarProps {
  type: ViewportType;
  realtime: boolean;
  renderMode: RenderMode;
  isMaximized?: boolean;
  onTypeChange: (type: ViewportType) => void;
  onRealtimeChange: (realtime: boolean) => void;
  onRenderModeChange: (mode: RenderMode) => void;
  onMaximize: () => void;
}

const VIEWPORT_LABELS: Record<ViewportType, string> = {
  top: 'Top',
  front: 'Front',
  side: 'Side',
  perspective: 'Perspective',
};

export function ViewportToolbar({
  type,
  realtime,
  renderMode,
  isMaximized,
  onTypeChange,
  onRealtimeChange,
  onRenderModeChange,
  onMaximize,
}: ViewportToolbarProps) {
  const isPerspective = type === 'perspective';

  return (
    <div className="flex h-6 shrink-0 items-center justify-between border-b bg-card/80 px-2 text-[10px] select-none">
      <div className="flex items-center gap-2">
        <select
          className="rounded bg-muted px-1 py-0 text-[10px] text-foreground outline-none"
          value={type}
          onChange={e => onTypeChange(e.target.value as ViewportType)}
        >
          <option value="top">Top</option>
          <option value="front">Front</option>
          <option value="side">Side</option>
          <option value="perspective">Perspective</option>
        </select>

        {isPerspective && (
          <select
            className="rounded bg-muted px-1 py-0 text-[10px] text-foreground outline-none"
            value={renderMode}
            onChange={e => onRenderModeChange(e.target.value as RenderMode)}
          >
            <option value="wireframe">Wireframe</option>
            <option value="unlit">Unlit</option>
            <option value="lit">Lit</option>
          </select>
        )}

        <label className="flex items-center gap-1 text-muted-foreground">
          <input
            type="checkbox"
            className="h-2.5 w-2.5 accent-primary"
            checked={realtime}
            onChange={e => onRealtimeChange(e.target.checked)}
          />
          RT
        </label>
      </div>

      <div className="flex items-center gap-2">
        <span className="font-medium text-muted-foreground">{VIEWPORT_LABELS[type]}</span>
        <button
          className="rounded p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground"
          onClick={onMaximize}
          title={isMaximized ? 'Restore' : 'Maximize'}
        >
          {isMaximized ? <Minimize2 size={10} /> : <Maximize2 size={10} />}
        </button>
      </div>
    </div>
  );
}
