import { useCallback } from 'react';
import {
  Info,
  ToggleLeft,
  ToggleRight,
  Bug,
  MessageSquare,
} from 'lucide-react';
import {
  nodeTypeColors,
  portTypeColors,
  type EnumDefinition,
  type ScriptNodeData,
  type ScriptNodeTemplate,
} from '../editors/scriptTypes';

interface ScriptPropertyPanelProps {
  nodeData: ScriptNodeData | null;
  template: ScriptNodeTemplate | null;
  enums: EnumDefinition[];
  onPropertyChange: (nodeId: number, key: string, value: string) => void;
}

/**
 * Property editor panel for a selected script node.
 *
 * Shows:
 * - Node header info (name, type, category, description)
 * - Editable properties (text, number, checkbox, enum dropdown)
 * - Port info (read-only)
 * - Comment field, debug toggle, enabled toggle
 */
export default function ScriptPropertyPanel({
  nodeData,
  template,
  enums,
  onPropertyChange,
}: ScriptPropertyPanelProps) {
  if (!nodeData) {
    return (
      <div className="flex h-full items-center justify-center border-l border-border bg-card text-sm text-muted-foreground">
        Select a script node to inspect
      </div>
    );
  }

  const colors = nodeTypeColors[nodeData.nodeType] ?? nodeTypeColors.Action;

  return (
    <div className="subtle-scrollbar flex h-full flex-col overflow-y-auto border-l border-border bg-card">
      {/* Header */}
      <div className="border-b border-border px-4 py-2">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Script Node Inspector
        </h2>
      </div>

      <div className="space-y-4 p-4">
        {/* Node identity */}
        <section>
          <div className="flex items-center gap-2">
            <Info className="h-3.5 w-3.5" style={{ color: colors.text }} />
            <h3 className="text-sm font-medium" style={{ color: colors.text }}>
              {nodeData.displayName}
            </h3>
          </div>
          <div className="mt-2 space-y-2 text-sm">
            <Field label="Node ID" value={String(nodeData.nodeId)} />
            <Field label="Template" value={nodeData.templateRef} />
            <Field label="Type" value={nodeData.nodeType} />
            {nodeData.category && <Field label="Category" value={nodeData.category} />}
          </div>
          {nodeData.description && (
            <p className="mt-2 text-xs leading-5 text-muted-foreground">{nodeData.description}</p>
          )}
        </section>

        {/* Toggles */}
        <section className="space-y-2">
          <ToggleField
            icon={<ToggleLeft className="h-3.5 w-3.5" />}
            label="Enabled"
            value={nodeData.enabled}
            onChange={(v) => onPropertyChange(nodeData.nodeId, '_enabled', v ? 'true' : 'false')}
          />
          <ToggleField
            icon={<Bug className="h-3.5 w-3.5" />}
            label="Debug"
            value={nodeData.debug}
            onChange={(v) => onPropertyChange(nodeData.nodeId, '_debug', v ? 'true' : 'false')}
          />
        </section>

        {/* Comment */}
        <section>
          <h3 className="mb-2 flex items-center gap-1.5 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            <MessageSquare className="h-3.5 w-3.5" />
            Comment
          </h3>
          <textarea
            className="w-full rounded-[12px] border border-white/8 bg-[rgba(255,255,255,0.03)] px-3 py-2 text-xs text-white outline-none placeholder:text-[rgba(160,174,192,0.56)] focus:border-[rgba(245,170,49,0.3)]"
            rows={2}
            value={nodeData.comment}
            onChange={(e) => onPropertyChange(nodeData.nodeId, '_comment', e.target.value)}
            placeholder="Add a note..."
          />
        </section>

        {/* Properties */}
        {template && template.properties.length > 0 && (
          <section>
            <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Properties ({template.properties.length})
            </h3>
            <div className="space-y-2">
              {template.properties.map((propDef) => (
                <PropertyField
                  key={propDef.name}
                  name={propDef.name}
                  propType={propDef.prop_type}
                  value={nodeData.properties[propDef.name] ?? propDef.default_value}
                  databaseRef={propDef.database_ref}
                  enums={enums}
                  onChange={(v) => onPropertyChange(nodeData.nodeId, propDef.name, v)}
                />
              ))}
            </div>
          </section>
        )}

        {/* Ports info (read-only) */}
        <section>
          <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Input Ports ({nodeData.inputPorts.length})
          </h3>
          <div className="space-y-1">
            {nodeData.inputPorts.map((p) => (
              <PortRow key={`in-${p.name}`} name={p.name} portType={p.portType} visible={p.visible} />
            ))}
            {nodeData.inputPorts.length === 0 && (
              <p className="text-xs text-muted-foreground">No input ports</p>
            )}
          </div>
        </section>

        <section>
          <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Output Ports ({nodeData.outputPorts.length})
          </h3>
          <div className="space-y-1">
            {nodeData.outputPorts.map((p) => (
              <PortRow key={`out-${p.name}`} name={p.name} portType={p.portType} visible={p.visible} />
            ))}
            {nodeData.outputPorts.length === 0 && (
              <p className="text-xs text-muted-foreground">No output ports</p>
            )}
          </div>
        </section>
      </div>
    </div>
  );
}

/* ----- Sub-components ----- */

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-baseline justify-between gap-2">
      <span className="text-muted-foreground">{label}</span>
      <span className="text-right font-mono text-xs text-foreground">{value}</span>
    </div>
  );
}

function ToggleField({
  icon,
  label,
  value,
  onChange,
}: {
  icon: React.ReactNode;
  label: string;
  value: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <button
      className="flex w-full items-center gap-2 rounded-[12px] border border-white/8 bg-[rgba(255,255,255,0.03)] px-3 py-2 text-xs transition-colors hover:bg-[rgba(255,255,255,0.06)]"
      onClick={() => onChange(!value)}
      type="button"
    >
      <span className={value ? 'text-[#86efac]' : 'text-muted-foreground'}>{icon}</span>
      <span className="uppercase tracking-[0.18em] text-[rgba(160,174,192,0.72)]">{label}</span>
      <span className="ml-auto">
        {value ? (
          <ToggleRight className="h-4 w-4 text-[#86efac]" />
        ) : (
          <ToggleLeft className="h-4 w-4 text-muted-foreground" />
        )}
      </span>
    </button>
  );
}

function PropertyField({
  name,
  propType,
  value,
  databaseRef,
  enums,
  onChange,
}: {
  name: string;
  propType: string;
  value: string;
  databaseRef: string | null;
  enums: EnumDefinition[];
  onChange: (v: string) => void;
}) {
  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
      onChange(e.target.value);
    },
    [onChange],
  );

  // Check if this is an enum type
  const enumDef = databaseRef ? enums.find((e) => e.name === databaseRef) : null;

  return (
    <div className="rounded-[12px] border border-white/8 bg-[rgba(255,255,255,0.03)] px-3 py-2">
      <span className="block text-[10px] font-semibold uppercase tracking-[0.18em] text-[rgba(160,174,192,0.72)]">
        {name}
        <span className="ml-2 font-normal normal-case tracking-normal text-[rgba(160,174,192,0.48)]">
          ({propType})
        </span>
      </span>

      {propType === 'Boolean' ? (
        <button
          className="mt-1 flex items-center gap-2 text-sm"
          onClick={() => onChange(value === 'true' ? 'false' : 'true')}
          type="button"
        >
          {value === 'true' ? (
            <ToggleRight className="h-4 w-4 text-[#86efac]" />
          ) : (
            <ToggleLeft className="h-4 w-4 text-muted-foreground" />
          )}
          <span className="text-xs text-foreground">{value}</span>
        </button>
      ) : enumDef ? (
        <select
          className="mt-1 w-full bg-transparent text-sm text-white outline-none"
          value={value}
          onChange={handleChange}
        >
          {enumDef.tokens.map((tok) => (
            <option key={tok.value} value={tok.value}>
              {tok.name}
            </option>
          ))}
        </select>
      ) : propType === 'Integer' ? (
        <input
          className="mt-1 w-full bg-transparent font-mono text-sm text-white outline-none placeholder:text-[rgba(160,174,192,0.48)]"
          type="number"
          step="1"
          value={value}
          onChange={handleChange}
        />
      ) : propType === 'Float' ? (
        <input
          className="mt-1 w-full bg-transparent font-mono text-sm text-white outline-none placeholder:text-[rgba(160,174,192,0.48)]"
          type="number"
          step="0.01"
          value={value}
          onChange={handleChange}
        />
      ) : (
        <input
          className="mt-1 w-full bg-transparent text-sm text-white outline-none placeholder:text-[rgba(160,174,192,0.48)]"
          type="text"
          value={value}
          onChange={handleChange}
        />
      )}
    </div>
  );
}

function PortRow({
  name,
  portType,
  visible,
}: {
  name: string;
  portType: string;
  visible: boolean;
}) {
  const color = portTypeColors[portType] ?? '#64748b';
  return (
    <div
      className="flex items-center gap-2 rounded border border-border bg-muted/50 px-3 py-1.5 text-xs"
      style={{ opacity: visible ? 1 : 0.45 }}
    >
      <span
        className="h-2 w-2 rounded-full"
        style={{ backgroundColor: color }}
      />
      <span className="text-foreground">{name}</span>
      <span className="ml-auto font-mono text-muted-foreground">{portType}</span>
      {!visible && (
        <span className="text-[9px] uppercase tracking-wider text-muted-foreground">hidden</span>
      )}
    </div>
  );
}
