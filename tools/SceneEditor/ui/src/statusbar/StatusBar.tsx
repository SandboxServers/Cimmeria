import type { ZoneSummary, ActorEntry, DbStatus } from '../lib/types';

interface StatusBarProps {
  zone: ZoneSummary | null;
  mousePos: { x: number; y: number; z: number };
  selectedActor: ActorEntry | null;
  gridSnap: number;
  actorCount: number;
  totalActorCount: number;
  selectionCount: number;
  meshProgress: { loaded: number; total: number } | null;
  transformMode: 'move' | 'rotate' | 'scale';
  dbStatus: DbStatus | null;
  dbEntityCount: number;
  placementMode: boolean;
  placementTemplateName: string | null;
}

export function StatusBar({ zone, mousePos, selectedActor, gridSnap, actorCount, totalActorCount, selectionCount, meshProgress, transformMode, dbStatus, dbEntityCount, placementMode, placementTemplateName }: StatusBarProps) {
  return (
    <div className="flex h-7 shrink-0 items-center gap-0 border-t bg-card text-[11px] select-none">
      {/* Zone info */}
      <StatusField>
        {zone
          ? `Zone: ${zone.zone_name} (${actorCount.toLocaleString()}/${totalActorCount.toLocaleString()} visible)`
          : 'No zone loaded'}
      </StatusField>

      <StatusDivider />

      {/* Selected actor */}
      <StatusField>
        {selectionCount > 1
          ? `${selectionCount} actors selected`
          : selectedActor
            ? `${selectedActor.class_name} :: ${selectedActor.object_name}`
            : 'No selection'}
      </StatusField>

      <StatusDivider />

      {/* Mouse position */}
      <StatusField mono>
        X: {mousePos.x.toFixed(0)}  Y: {mousePos.y.toFixed(0)}  Z: {mousePos.z.toFixed(0)}
      </StatusField>

      <StatusDivider />

      {/* Transform + Grid */}
      <StatusField mono>
        {transformMode.charAt(0).toUpperCase() + transformMode.slice(1)} | Grid: {gridSnap}
      </StatusField>

      {/* Mesh loading progress */}
      {meshProgress && (
        <>
          <StatusDivider />
          <StatusField>
            <span className="flex items-center gap-1.5">
              <span className="relative h-1.5 w-20 overflow-hidden rounded-full bg-muted">
                <span
                  className="absolute inset-y-0 left-0 rounded-full bg-primary transition-all duration-200"
                  style={{ width: `${(meshProgress.loaded / meshProgress.total) * 100}%` }}
                />
              </span>
              <span className="font-mono text-[10px]">
                Meshes: {meshProgress.loaded}/{meshProgress.total}
              </span>
            </span>
          </StatusField>
        </>
      )}

      {/* DB status */}
      <StatusDivider />
      <StatusField>
        {dbStatus?.connected
          ? <span className="text-emerald-500">DB: {dbEntityCount} entities</span>
          : <span className="text-muted-foreground/40">DB: offline</span>}
      </StatusField>

      {/* Placement mode */}
      {placementMode && placementTemplateName && (
        <>
          <StatusDivider />
          <StatusField>
            <span className="text-amber-400">Placing: {placementTemplateName}</span>
          </StatusField>
        </>
      )}

      {/* Keyboard hints (right-aligned) */}
      <div className="ml-auto flex items-center gap-1 px-2">
        <KbdHint keys="Ctrl+1-9" label="Bookmark" />
        <KbdHint keys="W/E/R" label="Move/Rot/Scale" />
      </div>
    </div>
  );
}

function StatusField({ children, mono }: { children: React.ReactNode; mono?: boolean }) {
  return (
    <div
      className={`truncate px-3 text-muted-foreground ${mono ? 'font-mono text-[10px]' : ''}`}
    >
      {children}
    </div>
  );
}

function StatusDivider() {
  return <div className="h-4 w-px shrink-0 bg-border" />;
}

function KbdHint({ keys, label }: { keys: string; label: string }) {
  return (
    <span className="text-[9px] text-muted-foreground/50">
      <span className="font-mono">{keys}</span> {label}
    </span>
  );
}
