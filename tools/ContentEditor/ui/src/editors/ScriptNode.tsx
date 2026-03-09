import { memo, useEffect } from 'react';
import {
  Handle,
  type Node,
  type NodeProps,
  Position,
  useUpdateNodeInternals,
} from '@xyflow/react';
import { nodeTypeColors, portTypeColors, type ScriptNodeData } from './scriptTypes';

/**
 * Calculate the CSS `top` position for a handle at a given index within
 * a set of `count` handles, starting below the header area.
 */
function getPortTop(index: number, count: number): string {
  // Header is roughly 44px tall; each port row is ~28px
  const headerOffset = 52;
  const rowHeight = 28;
  const top = headerOffset + index * rowHeight + rowHeight / 2;
  // Return as px so the handle layout is predictable
  return `${top}px`;
}

/**
 * Compute a stable signature for handle layout so xyflow can re-measure
 * when port visibility changes.
 */
function getHandleSignature(data: ScriptNodeData): string {
  const visIn = data.inputPorts.filter((p) => p.visible).map((p) => p.name);
  const visOut = data.outputPorts.filter((p) => p.visible).map((p) => p.name);
  return JSON.stringify({ visIn, visOut });
}

/**
 * Get the color for a port type, falling back to a neutral gray.
 */
function getPortColor(portType: string): string {
  return portTypeColors[portType] ?? '#64748b';
}

/**
 * Custom xyflow node for rendering a script block on the visual script canvas.
 *
 * Renders:
 * - Header with display name + node type badge (colored by Variable/Action/Condition/Event)
 * - Input ports on the left with colored dots and labels
 * - Output ports on the right with colored dots and labels
 * - Trigger ports get a red diamond-shaped indicator
 */
export const ScriptNode = memo(function ScriptNode({
  data,
  id,
  selected,
}: NodeProps<Node<ScriptNodeData>>) {
  const updateNodeInternals = useUpdateNodeInternals();
  const nodeData = data as ScriptNodeData;
  const colors = nodeTypeColors[nodeData.nodeType] ?? nodeTypeColors.Action;
  const handleSig = getHandleSignature(nodeData);

  const visibleInputs = nodeData.inputPorts.filter((p) => p.visible);
  const visibleOutputs = nodeData.outputPorts.filter((p) => p.visible);
  const maxPorts = Math.max(visibleInputs.length, visibleOutputs.length, 1);

  // Minimum node height: header (52) + ports rows + padding
  const nodeHeight = 52 + maxPorts * 28 + 12;

  useEffect(() => {
    updateNodeInternals(id);
  }, [handleSig, id, updateNodeInternals]);

  return (
    <div
      className="relative rounded-[20px] border px-0 py-0 shadow-[0_16px_40px_rgba(0,0,0,0.32)] backdrop-blur-lg transition-all"
      style={{
        minWidth: 220,
        minHeight: nodeHeight,
        borderColor: selected ? '#f5aa31' : colors.border,
        backgroundColor: selected ? 'rgba(245,170,49,0.12)' : 'rgba(9,18,28,0.92)',
        boxShadow: selected
          ? `0 0 0 1px rgba(245,170,49,0.3), 0 16px 40px rgba(0,0,0,0.32)`
          : undefined,
        opacity: nodeData.enabled ? 1 : 0.5,
      }}
    >
      {/* Header */}
      <div
        className="flex items-center justify-between gap-2 rounded-t-[20px] px-4 py-2.5"
        style={{ backgroundColor: colors.bg }}
      >
        <span
          className="truncate text-sm font-semibold tracking-[-0.02em]"
          style={{ color: colors.text }}
        >
          {nodeData.displayName}
        </span>
        <span
          className="shrink-0 rounded-full px-2.5 py-0.5 text-[10px] font-semibold uppercase tracking-[0.18em]"
          style={{
            backgroundColor: `${colors.border}`,
            color: colors.text,
          }}
        >
          {nodeData.nodeType}
        </span>
      </div>

      {/* Port rows */}
      <div className="flex px-1 pb-2 pt-1" style={{ minHeight: maxPorts * 28 + 8 }}>
        {/* Input ports column */}
        <div className="flex min-w-[80px] flex-1 flex-col gap-0">
          {visibleInputs.map((port) => {
            const color = getPortColor(port.portType);
            const isTrigger = port.portType === 'Trigger';
            return (
              <div
                className="flex items-center gap-1.5 py-0.5 pl-4 pr-2"
                key={`in-${port.name}`}
              >
                {isTrigger ? (
                  <span
                    className="inline-block h-2.5 w-2.5 rotate-45"
                    style={{ backgroundColor: color }}
                  />
                ) : (
                  <span
                    className="inline-block h-2 w-2 rounded-full"
                    style={{ backgroundColor: color }}
                  />
                )}
                <span className="text-[11px] text-[rgba(224,231,239,0.76)]">{port.name}</span>
              </div>
            );
          })}
        </div>

        {/* Output ports column */}
        <div className="flex min-w-[80px] flex-1 flex-col items-end gap-0">
          {visibleOutputs.map((port) => {
            const color = getPortColor(port.portType);
            const isTrigger = port.portType === 'Trigger';
            return (
              <div
                className="flex items-center gap-1.5 py-0.5 pl-2 pr-4"
                key={`out-${port.name}`}
              >
                <span className="text-[11px] text-[rgba(224,231,239,0.76)]">{port.name}</span>
                {isTrigger ? (
                  <span
                    className="inline-block h-2.5 w-2.5 rotate-45"
                    style={{ backgroundColor: color }}
                  />
                ) : (
                  <span
                    className="inline-block h-2 w-2 rounded-full"
                    style={{ backgroundColor: color }}
                  />
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* Debug badge */}
      {nodeData.debug && (
        <div className="absolute -top-2 right-2 rounded-full bg-[rgba(239,68,68,0.3)] px-2 py-0.5 text-[9px] font-semibold uppercase tracking-[0.14em] text-[#fca5a5]">
          Debug
        </div>
      )}

      {/* Comment indicator */}
      {nodeData.comment && (
        <div className="border-t border-[rgba(255,255,255,0.06)] px-4 py-1.5">
          <p className="truncate text-[10px] text-[rgba(160,174,192,0.72)]">{nodeData.comment}</p>
        </div>
      )}

      {/* Input handles */}
      {visibleInputs.map((port, index) => {
        const color = getPortColor(port.portType);
        return (
          <Handle
            id={`in-${port.name}`}
            key={`in-${port.name}`}
            position={Position.Left}
            style={{
              background: color,
              border: `2px solid ${color}cc`,
              boxShadow: `0 0 6px ${color}44`,
              height: 12,
              left: -7,
              top: getPortTop(index, visibleInputs.length),
              width: 12,
            }}
            type="target"
          />
        );
      })}

      {/* Output handles */}
      {visibleOutputs.map((port, index) => {
        const color = getPortColor(port.portType);
        return (
          <Handle
            id={`out-${port.name}`}
            key={`out-${port.name}`}
            position={Position.Right}
            style={{
              background: color,
              border: `2px solid ${color}cc`,
              boxShadow: `0 0 6px ${color}44`,
              height: 12,
              right: -7,
              top: getPortTop(index, visibleOutputs.length),
              width: 12,
            }}
            type="source"
          />
        );
      })}
    </div>
  );
});
