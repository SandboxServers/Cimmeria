import { memo, useEffect } from 'react';
import {
  Handle,
  type Node,
  type NodeProps,
  Position,
  useUpdateNodeInternals,
} from '@xyflow/react';
import { familyLabel, familyTint } from './constants';
import type { EditorNodeData, MissionNodeData } from './types';

/**
 * Resolve the port labels for a mission node, falling back to sensible defaults.
 * Anchor-End nodes have no outputs; all other nodes default to a single "In"/"Out".
 */
function getPortLabels(data: MissionNodeData) {
  const fallbackInputs = ['In'];
  const fallbackOutputs =
    data.family === 'anchor' && data.stage === 'End' ? [] : ['Out'];

  return {
    inputs: data.inputs ?? fallbackInputs,
    outputs: data.outputs ?? fallbackOutputs,
  };
}

/**
 * Produce a JSON signature of the current handle layout so xyflow can
 * re-measure when ports are added or removed.
 */
function getHandleSignature(data: MissionNodeData) {
  const { inputs, outputs } = getPortLabels(data);
  return JSON.stringify({ inputs, outputs });
}

/**
 * Calculate the CSS `top` position for a handle at a given index within
 * a set of `count` handles, distributing them evenly along the node height.
 */
function getPortTop(index: number, count: number): string {
  if (count <= 1) {
    return '50%';
  }
  return `${((index + 1) / (count + 1)) * 100}%`;
}

/**
 * Custom xyflow node for rendering a mission card on the chain flow canvas.
 *
 * Renders:
 * - Family badge (Sequence / Trigger / Condition / Action / Counter)
 * - Title, scenario, status, thread badge, detail text
 * - Up to 3 property rows
 * - Input handles on the left, output handles on the right
 * - Port labels next to each handle (visible at xl breakpoint)
 * - Color theming driven by family tint and per-node accent
 */
export const MissionNode = memo(function MissionNode({
  data,
  id,
  selected,
}: NodeProps<Node<EditorNodeData>>) {
  const updateNodeInternals = useUpdateNodeInternals();
  const missionData = data as MissionNodeData;
  const tint = familyTint[missionData.family];
  const { inputs, outputs } = getPortLabels(missionData);
  const handleSignature = getHandleSignature(missionData);

  useEffect(() => {
    updateNodeInternals(id);
  }, [handleSignature, id, updateNodeInternals]);

  return (
    <div
      className={`relative min-w-[250px] rounded-[28px] border px-5 py-4 shadow-[0_22px_50px_rgba(0,0,0,0.28)] backdrop-blur-lg transition-all ${
        selected
          ? 'border-[#f5aa31] bg-[rgba(245,170,49,0.14)]'
          : 'border-[rgba(255,255,255,0.08)] bg-[rgba(9,18,28,0.9)]'
      }`}
      style={{
        boxShadow: selected
          ? `0 0 0 1px ${missionData.accent}33, 0 22px 50px rgba(0,0,0,0.28)`
          : undefined,
      }}
    >
      {/* Thread color dot */}
      <div
        className="absolute left-4 top-4 h-2.5 w-2.5 rounded-full shadow-[0_0_18px_rgba(255,255,255,0.25)]"
        style={{ backgroundColor: missionData.threadColor ?? missionData.accent }}
      />

      {/* Thread color top bar for Start/End anchors */}
      {missionData.threadColor && (missionData.stage === 'Start' || missionData.stage === 'End') ? (
        <div
          className="absolute inset-x-0 top-0 h-1 rounded-t-[28px]"
          style={{ backgroundColor: missionData.threadColor }}
        />
      ) : null}

      {/* Family badge + status */}
      <div className="ml-5 flex items-center justify-between gap-3">
        <span
          className="rounded-full px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.22em]"
          style={{
            backgroundColor: tint.chip,
            color: tint.text,
          }}
        >
          {familyLabel[missionData.family]}
        </span>
        <span
          className="rounded-full px-3 py-1 text-[11px] font-medium"
          style={{
            backgroundColor: missionData.threadColor ? `${missionData.threadColor}22` : tint.chip,
            color: missionData.threadColor ?? tint.text,
          }}
        >
          {missionData.status}
        </span>
      </div>

      {/* Title + scenario */}
      <h3 className="mt-4 text-lg font-semibold tracking-[-0.04em] text-white">
        {missionData.title}
      </h3>
      <p className="mt-1 text-xs uppercase tracking-[0.22em] text-[rgba(160,174,192,0.72)]">
        {missionData.scenario}
      </p>

      {/* Thread name badge */}
      {missionData.threadColor && missionData.threadName ? (
        <div className="mt-2">
          <span
            className="rounded-full px-3 py-1 text-[10px] font-semibold uppercase tracking-[0.2em]"
            style={{
              backgroundColor: `${missionData.threadColor}22`,
              border: `1px solid ${missionData.threadColor}55`,
              color: missionData.threadColor,
            }}
          >
            {missionData.threadName}
          </span>
        </div>
      ) : null}

      {/* Detail text */}
      <p className="mt-3 text-sm leading-6 text-[rgba(224,231,239,0.76)]">{missionData.detail}</p>

      {/* Properties panel (up to 3 rows) */}
      <div className="mt-4 space-y-2 rounded-[20px] border border-[rgba(255,255,255,0.06)] bg-[rgba(0,0,0,0.18)] p-3">
        {missionData.properties.slice(0, 3).map((item) => (
          <div className="flex items-center justify-between gap-3 text-xs" key={item.label}>
            <span className="uppercase tracking-[0.2em] text-[rgba(160,174,192,0.74)]">
              {item.label}
            </span>
            <span className="font-mono text-[rgba(244,247,250,0.92)]">{item.value}</span>
          </div>
        ))}
      </div>

      {/* Input port labels */}
      {inputs.map((label, index) => {
        const top = getPortTop(index, inputs.length);
        return (
          <div
            className="pointer-events-none absolute -left-[108px] hidden -translate-y-1/2 xl:block"
            key={`input-label-${label}-${index}`}
            style={{ top }}
          >
            <span className="block max-w-[92px] overflow-hidden text-ellipsis whitespace-nowrap rounded-full border border-[rgba(255,94,91,0.28)] bg-[rgba(255,94,91,0.14)] px-2 py-1 text-[9px] font-semibold uppercase tracking-[0.12em] text-[#ffd0cf]">
              {label}
            </span>
          </div>
        );
      })}

      {/* Output port labels */}
      {outputs.map((label, index) => {
        const top = getPortTop(index, outputs.length);
        return (
          <div
            className="pointer-events-none absolute -right-[108px] hidden -translate-y-1/2 xl:block"
            key={`output-label-${label}-${index}`}
            style={{ top }}
          >
            <span className="block max-w-[92px] overflow-hidden text-ellipsis whitespace-nowrap rounded-full border border-[rgba(255,94,91,0.28)] bg-[rgba(255,94,91,0.14)] px-2 py-1 text-[9px] font-semibold uppercase tracking-[0.12em] text-[#ffd0cf]">
              {label}
            </span>
          </div>
        );
      })}

      {/* Input handles */}
      {inputs.map((_, index) => (
        <Handle
          id={`input-${index}`}
          key={`input-${index}`}
          position={Position.Left}
          style={{
            background: '#ff5e5b',
            border: '2px solid rgba(255, 208, 207, 0.9)',
            boxShadow: `0 0 0 4px ${familyTint.anchor.glow}`,
            height: 16,
            left: -10,
            top: getPortTop(index, inputs.length),
            width: 16,
          }}
          type="target"
        />
      ))}

      {/* Output handles */}
      {outputs.map((_, index) => (
        <Handle
          id={`output-${index}`}
          key={`output-${index}`}
          position={Position.Right}
          style={{
            background: '#ff5e5b',
            border: '2px solid rgba(255, 208, 207, 0.9)',
            boxShadow: `0 0 0 4px ${familyTint.anchor.glow}`,
            height: 16,
            right: -10,
            top: getPortTop(index, outputs.length),
            width: 16,
          }}
          type="source"
        />
      ))}
    </div>
  );
});
