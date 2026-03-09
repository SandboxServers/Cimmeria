import { memo } from 'react';
import { BaseEdge, type EdgeProps, getBezierPath } from '@xyflow/react';

/**
 * Custom xyflow edge that renders a sequence thread between mission nodes.
 *
 * Renders:
 * - A wide translucent glow stroke underneath the main path
 * - A narrower opaque stroke on top (both using the sequence color)
 * - A filled circle at the source endpoint
 * - A stroked circle at the target endpoint
 * - A floating label pill at the midpoint showing the sequence name
 *
 * The edge color and label are read from `data.color` and `data.label`
 * on the SequenceEdgeData attached to each edge.
 */
export const SequenceThreadEdge = memo(function SequenceThreadEdge({
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
  const [path, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
    curvature: 0.28,
  });
  const color = typeof data?.color === 'string' ? data.color : '#ff5e5b';

  return (
    <>
      {/* Glow stroke */}
      <BaseEdge
        id={`${id}-glow`}
        path={path}
        style={{
          stroke: `${color}33`,
          strokeWidth: selected ? 14 : 10,
          strokeLinecap: 'round',
        }}
      />

      {/* Main stroke */}
      <BaseEdge
        id={id}
        path={path}
        style={{
          stroke: color,
          strokeWidth: selected ? 5 : 4,
          strokeLinecap: 'round',
        }}
      />

      {/* Source dot */}
      <circle cx={sourceX} cy={sourceY} fill={color} r={6} />

      {/* Target dot */}
      <circle cx={targetX} cy={targetY} fill="#e5eef3" r={5} stroke={color} strokeWidth={2} />

      {/* Midpoint label */}
      <foreignObject height={48} width={220} x={labelX - 110} y={labelY - 24}>
        <div className="flex h-full items-center justify-center">
          <span
            className="inline-flex items-center gap-2 rounded-full px-4 py-2 text-[11px] font-semibold uppercase tracking-[0.2em] shadow-[0_10px_26px_rgba(0,0,0,0.45)]"
            style={{
              backgroundColor: 'rgba(4, 10, 18, 0.97)',
              border: `1px solid ${color}66`,
              color: '#f8fafc',
            }}
          >
            <span className="h-2.5 w-2.5 rounded-full" style={{ backgroundColor: color }} />
            {typeof data?.label === 'string' ? data.label : 'Sequence yarn'}
          </span>
        </div>
      </foreignObject>
    </>
  );
});
