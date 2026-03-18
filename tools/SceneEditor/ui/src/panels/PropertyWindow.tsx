import { useState, useRef, useEffect } from 'react';
import { ChevronRight, ChevronDown } from 'lucide-react';
import type { ActorEntry } from '../lib/types';
import { ACTOR_COLORS } from '../lib/types';

interface PropertyWindowProps {
  actor: ActorEntry | null;
  selectionCount: number;
  onMoveActor?: (key: string, x: number, y: number, z: number) => void;
  onRotateActor?: (key: string, pitch: number, yaw: number, roll: number) => void;
  onScaleActor?: (key: string, drawScale: number, dsX: number, dsY: number, dsZ: number) => void;
  onPropertyChange?: (key: string, propName: string, newValue: string) => void;
  isLocked?: boolean;
}

interface PropertyCategory {
  name: string;
  entries: PropertyEntry[];
}

interface PropertyEntry {
  label: string;
  value: string;
  editable?: boolean;
  editKey?: string; // identifier for which field group this belongs to
}

function buildCategories(actor: ActorEntry): PropertyCategory[] {
  const categories: PropertyCategory[] = [];

  // Movement
  categories.push({
    name: 'Movement',
    entries: [
      { label: 'X', value: actor.x.toFixed(1), editable: true, editKey: 'loc_x' },
      { label: 'Y', value: actor.y.toFixed(1), editable: true, editKey: 'loc_y' },
      { label: 'Z', value: actor.z.toFixed(1), editable: true, editKey: 'loc_z' },
      { label: 'Pitch', value: actor.pitch.toFixed(1), editable: true, editKey: 'rot_pitch' },
      { label: 'Yaw', value: actor.yaw.toFixed(1), editable: true, editKey: 'rot_yaw' },
      { label: 'Roll', value: actor.roll.toFixed(1), editable: true, editKey: 'rot_roll' },
    ],
  });

  // Display
  categories.push({
    name: 'Display',
    entries: [
      { label: 'DrawScale', value: actor.draw_scale.toFixed(4), editable: true, editKey: 'ds' },
      { label: 'DrawScale X', value: actor.draw_scale_x.toFixed(2), editable: true, editKey: 'ds_x' },
      { label: 'DrawScale Y', value: actor.draw_scale_y.toFixed(2), editable: true, editKey: 'ds_y' },
      { label: 'DrawScale Z', value: actor.draw_scale_z.toFixed(2), editable: true, editKey: 'ds_z' },
    ],
  });

  // Object
  categories.push({
    name: 'Object',
    entries: [
      { label: 'Class', value: actor.class_name },
      { label: 'Name', value: actor.object_name },
      { label: 'Tile', value: actor.tile },
      { label: 'Key', value: actor.key },
    ],
  });

  // Mesh
  if (actor.static_mesh) {
    categories.push({
      name: 'Mesh',
      entries: [
        { label: 'StaticMesh', value: actor.static_mesh },
      ],
    });
  }

  // CME-specific: SGW gameplay properties
  const sgwProps = actor.properties.filter(p =>
    p.name.startsWith('SGW') || p.name.startsWith('Spawn') ||
    p.name === 'bEnabled' || p.name === 'SpawnClass' ||
    p.name === 'MaxCount' || p.name === 'RespawnTime' ||
    p.name === 'LinkedStargate' || p.name === 'DestinationZone' ||
    p.name === 'MissionID' || p.name === 'InteractionType' ||
    p.name === 'bNoDelete' || p.name === 'bCollideActors'
  );
  if (sgwProps.length > 0) {
    categories.push({
      name: 'CME Data',
      entries: sgwProps.map(p => ({
        label: p.name,
        value: p.value,
        editable: true,
        editKey: `prop:${p.name}`,
      })),
    });
  }

  // Additional properties (from tagged property data)
  const knownNames = new Set([
    'Location', 'Rotation', 'DrawScale', 'DrawScale3D', 'StaticMesh',
    ...sgwProps.map(p => p.name),
  ]);
  const extraProps = actor.properties.filter(p => !knownNames.has(p.name));
  if (extraProps.length > 0) {
    categories.push({
      name: 'Properties',
      entries: extraProps.map(p => ({
        label: p.name,
        value: p.value,
        editable: true,
        editKey: `prop:${p.name}`,
      })),
    });
  }

  return categories;
}

export function PropertyWindow({ actor, selectionCount, onMoveActor, onRotateActor, onScaleActor, onPropertyChange, isLocked }: PropertyWindowProps) {
  const handleEdit = (editKey: string, newValue: number) => {
    if (isLocked) return;
    if (!actor) return;
    const k = actor.key;

    // Location changes
    if (editKey === 'loc_x') onMoveActor?.(k, newValue, actor.y, actor.z);
    else if (editKey === 'loc_y') onMoveActor?.(k, actor.x, newValue, actor.z);
    else if (editKey === 'loc_z') onMoveActor?.(k, actor.x, actor.y, newValue);
    // Rotation changes
    else if (editKey === 'rot_pitch') onRotateActor?.(k, newValue, actor.yaw, actor.roll);
    else if (editKey === 'rot_yaw') onRotateActor?.(k, actor.pitch, newValue, actor.roll);
    else if (editKey === 'rot_roll') onRotateActor?.(k, actor.pitch, actor.yaw, newValue);
    // Scale changes
    else if (editKey === 'ds') onScaleActor?.(k, newValue, actor.draw_scale_x, actor.draw_scale_y, actor.draw_scale_z);
    else if (editKey === 'ds_x') onScaleActor?.(k, actor.draw_scale, newValue, actor.draw_scale_y, actor.draw_scale_z);
    else if (editKey === 'ds_y') onScaleActor?.(k, actor.draw_scale, actor.draw_scale_x, newValue, actor.draw_scale_z);
    else if (editKey === 'ds_z') onScaleActor?.(k, actor.draw_scale, actor.draw_scale_x, actor.draw_scale_y, newValue);
    // Generic property changes
    else if (editKey.startsWith('prop:')) {
      const propName = editKey.slice(5);
      onPropertyChange?.(k, propName, String(newValue));
    }
  };

  // Handle string property edits (for non-numeric properties)
  const handleStringEdit = (editKey: string, newValue: string) => {
    if (isLocked || !actor) return;
    if (editKey.startsWith('prop:')) {
      const propName = editKey.slice(5);
      onPropertyChange?.(actor.key, propName, newValue);
    }
  };

  return (
    <div className="flex h-full flex-col bg-card">
      {/* Header */}
      <div className="border-b px-3 py-1.5">
        <div className="flex items-center justify-between">
          <span className="text-xs font-medium text-foreground">Properties</span>
          {selectionCount > 1 && (
            <span className="text-[10px] text-muted-foreground">{selectionCount} selected</span>
          )}
        </div>
        {actor && (
          <div className="mt-1 flex items-center gap-2">
            <span
              className="h-2.5 w-2.5 rounded-full"
              style={{ backgroundColor: ACTOR_COLORS[actor.class_name] ?? ACTOR_COLORS._default }}
            />
            <span className="text-[11px] font-medium text-foreground">{actor.class_name}</span>
            <span className="truncate text-[10px] text-muted-foreground">{actor.object_name}</span>
          </div>
        )}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto subtle-scrollbar">
        {actor ? (
          <PropertyCategories actor={actor} onEdit={handleEdit} />
        ) : selectionCount > 1 ? (
          <div className="flex h-full items-center justify-center">
            <span className="text-xs text-muted-foreground/60">{selectionCount} actors selected</span>
          </div>
        ) : (
          <div className="flex h-full items-center justify-center">
            <span className="text-xs text-muted-foreground/60">No actor selected</span>
          </div>
        )}
      </div>
    </div>
  );
}

function PropertyCategories({ actor, onEdit }: { actor: ActorEntry; onEdit: (editKey: string, value: number) => void }) {
  const categories = buildCategories(actor);

  return (
    <div className="py-1">
      {categories.map(cat => (
        <CategorySection key={cat.name} category={cat} onEdit={onEdit} />
      ))}
    </div>
  );
}

function CategorySection({ category, onEdit }: { category: PropertyCategory; onEdit: (editKey: string, value: number) => void }) {
  const [expanded, setExpanded] = useState(true);

  return (
    <div>
      <button
        className="flex w-full items-center gap-1 px-2 py-0.5 text-left text-xs font-medium text-foreground hover:bg-muted"
        onClick={() => setExpanded(e => !e)}
      >
        {expanded ? (
          <ChevronDown size={12} className="shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight size={12} className="shrink-0 text-muted-foreground" />
        )}
        {category.name}
      </button>

      {expanded && (
        <div className="mb-1">
          {category.entries.map((entry, i) => (
            <PropertyRow key={`${entry.label}-${i}`} entry={entry} onEdit={onEdit} />
          ))}
        </div>
      )}
    </div>
  );
}

function PropertyRow({ entry, onEdit }: { entry: PropertyEntry; onEdit: (editKey: string, value: number) => void }) {
  if (entry.editable && entry.editKey) {
    return (
      <div className="flex items-center gap-2 px-5 py-px text-[11px]">
        <span className="w-24 shrink-0 truncate text-muted-foreground">{entry.label}</span>
        <EditableNumber
          value={entry.value}
          onCommit={(val) => onEdit(entry.editKey!, val)}
        />
      </div>
    );
  }

  return (
    <div className="flex items-start gap-2 px-5 py-px text-[11px]">
      <span className="w-24 shrink-0 truncate text-muted-foreground">{entry.label}</span>
      <span className="min-w-0 break-all font-mono text-[10px] text-foreground">{entry.value}</span>
    </div>
  );
}

function EditableNumber({ value, onCommit }: { value: string; onCommit: (val: number) => void }) {
  const [editing, setEditing] = useState(false);
  const [text, setText] = useState(value);
  const inputRef = useRef<HTMLInputElement>(null);

  // Sync when external value changes
  useEffect(() => {
    if (!editing) setText(value);
  }, [value, editing]);

  const commit = () => {
    setEditing(false);
    const num = parseFloat(text);
    if (!isNaN(num) && text !== value) {
      onCommit(num);
    } else {
      setText(value); // revert
    }
  };

  if (!editing) {
    return (
      <span
        className="min-w-0 cursor-text rounded px-1 font-mono text-[10px] text-foreground hover:bg-muted"
        onDoubleClick={() => {
          setEditing(true);
          setTimeout(() => inputRef.current?.select(), 0);
        }}
      >
        {text}
      </span>
    );
  }

  return (
    <input
      ref={inputRef}
      type="text"
      className="w-full min-w-0 rounded border border-primary/50 bg-background px-1 font-mono text-[10px] text-foreground outline-none focus:border-primary"
      value={text}
      onChange={e => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={e => {
        if (e.key === 'Enter') commit();
        if (e.key === 'Escape') { setText(value); setEditing(false); }
        e.stopPropagation(); // prevent viewport shortcuts
      }}
      autoFocus
    />
  );
}
