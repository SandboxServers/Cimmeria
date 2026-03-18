import { useState } from 'react';
import { FolderOpen, Loader2, Map } from 'lucide-react';
import type { ZoneInfo } from '../lib/types';

interface ZoneSelectorProps {
  zones: ZoneInfo[];
  loading: boolean;
  loadingMessage: string;
  onScan: (cookedPcPath: string) => void;
  onLoadZone: (zonePath: string) => void;
}

const DEFAULT_PATH = 'F:/Stargate Worlds-QA/Working/SGWGame/CookedPC';

export function ZoneSelector({
  zones,
  loading,
  loadingMessage,
  onScan,
  onLoadZone,
}: ZoneSelectorProps) {
  const [path, setPath] = useState(DEFAULT_PATH);

  return (
    <div className="flex h-screen items-center justify-center bg-background">
      <div className="w-[600px] rounded-lg border bg-card shadow-2xl">
        {/* Header */}
        <div className="border-b px-6 py-4">
          <h1 className="text-lg font-semibold text-foreground">Cimmeria Scene Editor</h1>
          <p className="mt-0.5 text-xs text-muted-foreground">
            Select a CookedPC directory and choose a zone to load.
          </p>
        </div>

        {/* Path input */}
        <div className="border-b px-6 py-4">
          <label className="mb-1 block text-xs font-medium text-muted-foreground">
            CookedPC Path
          </label>
          <div className="flex gap-2">
            <div className="relative flex-1">
              <FolderOpen size={14} className="absolute left-2.5 top-2 text-muted-foreground" />
              <input
                type="text"
                className="w-full rounded border bg-input py-1.5 pl-8 pr-3 font-mono text-xs text-foreground outline-none focus:border-primary"
                value={path}
                onChange={e => setPath(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && onScan(path)}
              />
            </div>
            <button
              className="rounded bg-primary px-4 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
              onClick={() => onScan(path)}
              disabled={loading || !path.trim()}
            >
              Scan
            </button>
          </div>
        </div>

        {/* Zone list */}
        <div className="max-h-[400px] overflow-y-auto subtle-scrollbar px-2 py-2">
          {loading && (
            <div className="flex items-center justify-center gap-2 py-8 text-xs text-muted-foreground">
              <Loader2 size={16} className="animate-spin" />
              <span>{loadingMessage || 'Loading...'}</span>
            </div>
          )}

          {!loading && zones.length === 0 && (
            <div className="py-8 text-center text-xs text-muted-foreground/60">
              Click &quot;Scan&quot; to find zones in the CookedPC directory.
            </div>
          )}

          {!loading && zones.length > 0 && (
            <>
              <div className="px-2 pb-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                {zones.length} zones found
              </div>
              {zones.map(zone => (
                <button
                  key={zone.path}
                  className="flex w-full items-center gap-3 rounded px-3 py-2 text-left hover:bg-muted"
                  onClick={() => onLoadZone(zone.path)}
                >
                  <Map size={16} className="shrink-0 text-accent" />
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-sm font-medium text-foreground">{zone.name}</div>
                    <div className="font-mono text-[10px] text-muted-foreground">
                      {zone.tile_count} tile{zone.tile_count !== 1 ? 's' : ''}
                    </div>
                  </div>
                </button>
              ))}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
