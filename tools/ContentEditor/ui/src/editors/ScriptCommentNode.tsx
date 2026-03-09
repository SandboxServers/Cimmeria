import { memo } from 'react';
import type { Node, NodeProps } from '@xyflow/react';
import type { ScriptCommentNodeData } from './scriptTypes';

/**
 * Predefined comment colors (matches the ServerEd comment palette).
 * The color index maps to a background / border pair.
 */
const commentColors: Record<number, { bg: string; border: string; text: string }> = {
  0: { bg: 'rgba(148,163,184,0.08)', border: 'rgba(148,163,184,0.2)', text: 'rgba(224,231,239,0.76)' },
  1: { bg: 'rgba(59,130,246,0.08)', border: 'rgba(59,130,246,0.2)', text: '#93c5fd' },
  2: { bg: 'rgba(34,197,94,0.08)', border: 'rgba(34,197,94,0.2)', text: '#86efac' },
  3: { bg: 'rgba(245,170,49,0.08)', border: 'rgba(245,170,49,0.2)', text: '#fcd34d' },
  4: { bg: 'rgba(168,85,247,0.08)', border: 'rgba(168,85,247,0.2)', text: '#c4b5fd' },
  5: { bg: 'rgba(239,68,68,0.08)', border: 'rgba(239,68,68,0.2)', text: '#fca5a5' },
};

const defaultCommentColor = commentColors[0];

/**
 * A simple transparent comment annotation node for the script canvas.
 * Shows a text label inside a colored, rounded rectangle. Not connectable.
 */
export const ScriptCommentNode = memo(function ScriptCommentNode({
  data,
  selected,
}: NodeProps<Node<ScriptCommentNodeData>>) {
  const commentData = data as ScriptCommentNodeData;
  const colors = commentColors[commentData.color] ?? defaultCommentColor;

  return (
    <div
      className="rounded-[16px] border p-4 transition-all"
      style={{
        width: commentData.width || 200,
        minHeight: commentData.height || 60,
        backgroundColor: colors.bg,
        borderColor: selected ? '#f5aa31' : colors.border,
        borderStyle: 'dashed',
      }}
    >
      <p
        className="whitespace-pre-wrap text-xs leading-5"
        style={{ color: colors.text }}
      >
        {commentData.text}
      </p>
    </div>
  );
});
