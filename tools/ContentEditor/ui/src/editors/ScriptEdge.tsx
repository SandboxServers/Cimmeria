import { memo } from 'react';
import { BaseEdge, type EdgeProps, getBezierPath } from '@xyflow/react';
import { portTypeColors } from './scriptTypes';

/**
 * Custom xyflow edge for rendering a connection between script node ports.
 *
 * Renders:
 * - A translucent glow stroke underneath the main path
 * - A narrower opaque stroke on top, colored by the source port type
 * - Slightly thicker strokes when the edge is selected
 */
export const ScriptEdge = memo(function ScriptEdge({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  data,
  selected,
}: EdgeProps) {
  const [path] = getBezierPath({
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
    curvature: 0.3,
  });

  const portType = typeof data?.portType === 'string' ? data.portType : '';
  const color = portTypeColors[portType] ?? '#64748b';

  return (
    <>
      {/* Glow stroke */}
      <BaseEdge
        id={`${id}-glow`}
        path={path}
        style={{
          stroke: `${color}30`,
          strokeWidth: selected ? 12 : 8,
          strokeLinecap: 'round',
        }}
      />

      {/* Main stroke */}
      <BaseEdge
        id={id}
        path={path}
        style={{
          stroke: color,
          strokeWidth: selected ? 3.5 : 2.5,
          strokeLinecap: 'round',
        }}
      />
    </>
  );
});
