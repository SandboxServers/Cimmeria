import { useState, useEffect } from 'react';
import type { DbEntity } from '../lib/types';
import { DB_ENTITY_COLORS } from '../lib/types';

interface DbEntityDetailProps {
  entity: DbEntity | null;
  onUpdatePosition?: (spawnId: number, x: number, y: number, z: number, heading: number) => void;
}

export function DbEntityDetail({ entity, onUpdatePosition }: DbEntityDetailProps) {
  const [editX, setEditX] = useState('');
  const [editY, setEditY] = useState('');
  const [editZ, setEditZ] = useState('');
  const [editHeading, setEditHeading] = useState('');
  const [dirty, setDirty] = useState(false);

  // Sync editable fields when entity changes
  useEffect(() => {
    if (entity) {
      setEditX(entity.x.toFixed(1));
      setEditY(entity.y.toFixed(1));
      setEditZ(entity.z.toFixed(1));
      setEditHeading(entity.heading.toFixed(1));
      setDirty(false);
    }
  }, [entity?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!entity) {
    return (
      <div className="flex h-full flex-col bg-card">
        <div className="border-b px-3 py-1.5">
          <span className="text-xs font-medium text-foreground">DB Entity</span>
        </div>
        <div className="flex flex-1 items-center justify-center">
          <span className="text-xs text-muted-foreground/60">Select a DB entity</span>
        </div>
      </div>
    );
  }

  const color = DB_ENTITY_COLORS[entity.entity_type] ?? DB_ENTITY_COLORS._default;
  const canEdit = entity.source_table === 'spawnlist' && onUpdatePosition;

  const handleSave = () => {
    if (!canEdit) return;
    const x = parseFloat(editX);
    const y = parseFloat(editY);
    const z = parseFloat(editZ);
    const heading = parseFloat(editHeading);
    if (!isNaN(x) && !isNaN(y) && !isNaN(z) && !isNaN(heading)) {
      onUpdatePosition!(entity.id, x, y, z, heading);
      setDirty(false);
    }
  };

  const handleFieldChange = (setter: (v: string) => void, value: string) => {
    setter(value);
    setDirty(true);
  };

  return (
    <div className="flex h-full flex-col bg-card">
      {/* Header */}
      <div className="flex items-center gap-2 border-b px-3 py-1.5">
        <span
          className="inline-block h-2.5 w-2.5 shrink-0 rotate-45"
          style={{ backgroundColor: color }}
        />
        <span className="truncate text-xs font-medium text-foreground">
          {entity.name || entity.entity_type}
        </span>
        <span className="ml-auto text-[10px] text-muted-foreground">
          #{entity.id}
        </span>
      </div>

      {/* Properties */}
      <div className="flex-1 overflow-y-auto subtle-scrollbar">
        <PropertySection title="Identity">
          <PropertyRow label="Source" value={entity.source_table} />
          <PropertyRow label="Type" value={entity.entity_type} />
          <PropertyRow label="Name" value={entity.name} />
          {entity.tag && <PropertyRow label="Tag" value={entity.tag} />}
          {entity.template_id != null && (
            <PropertyRow label="Template ID" value={entity.template_id.toString()} />
          )}
        </PropertySection>

        <PropertySection title={canEdit ? 'Position (editable)' : 'Position (UE3 cm)'}>
          {canEdit ? (
            <>
              <EditableRow label="X" value={editX} onChange={v => handleFieldChange(setEditX, v)} />
              <EditableRow label="Y" value={editY} onChange={v => handleFieldChange(setEditY, v)} />
              <EditableRow label="Z" value={editZ} onChange={v => handleFieldChange(setEditZ, v)} />
              <EditableRow label="Heading" value={editHeading} onChange={v => handleFieldChange(setEditHeading, v)} />
              {dirty && (
                <button
                  onClick={handleSave}
                  className="mt-1 w-full rounded bg-primary px-2 py-1 text-[10px] font-medium text-primary-foreground hover:bg-primary/90"
                >
                  Save Position to DB
                </button>
              )}
            </>
          ) : (
            <>
              <PropertyRow label="X" value={entity.x.toFixed(1)} mono />
              <PropertyRow label="Y" value={entity.y.toFixed(1)} mono />
              <PropertyRow label="Z" value={entity.z.toFixed(1)} mono />
              <PropertyRow label="Heading" value={`${entity.heading.toFixed(1)}°`} mono />
            </>
          )}
        </PropertySection>

        {(entity.radius != null || entity.height != null) && (
          <PropertySection title="Region">
            {entity.radius != null && (
              <PropertyRow label="Radius" value={`${entity.radius.toFixed(0)} cm`} mono />
            )}
            {entity.height != null && (
              <PropertyRow label="Height" value={`${entity.height.toFixed(0)} cm`} mono />
            )}
            {entity.handler && <PropertyRow label="Handler" value={entity.handler} />}
          </PropertySection>
        )}

        {entity.gate_address && (
          <PropertySection title="Stargate">
            <PropertyRow label="Address" value={entity.gate_address} mono />
          </PropertySection>
        )}
      </div>
    </div>
  );
}

function PropertySection({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="border-b border-border/50">
      <div className="px-3 py-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground/60">
        {title}
      </div>
      <div className="px-3 pb-1.5">{children}</div>
    </div>
  );
}

function PropertyRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="flex items-center gap-2 py-0.5 text-[11px]">
      <span className="w-20 shrink-0 text-muted-foreground">{label}</span>
      <span className={`truncate text-foreground ${mono ? 'font-mono text-[10px]' : ''}`}>
        {value}
      </span>
    </div>
  );
}

function EditableRow({ label, value, onChange }: { label: string; value: string; onChange: (v: string) => void }) {
  return (
    <div className="flex items-center gap-2 py-0.5 text-[11px]">
      <span className="w-20 shrink-0 text-muted-foreground">{label}</span>
      <input
        type="number"
        value={value}
        onChange={e => onChange(e.target.value)}
        className="w-full rounded bg-input px-1.5 py-0.5 font-mono text-[10px] text-foreground outline-none"
      />
    </div>
  );
}
