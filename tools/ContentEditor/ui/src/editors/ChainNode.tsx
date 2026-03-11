import { memo } from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Zap, Filter, Play, Hash } from 'lucide-react';

interface ChainNodeData {
  chainId: number;
  name: string;
  description: string;
  scopeType: string;
  enabled: boolean;
  priority: number;
  triggerCount: number;
  conditionCount: number;
  actionCount: number;
  counterCount: number;
}

export const ChainNode = memo(function ChainNode({
  data,
  selected,
}: NodeProps) {
  const d = data as unknown as ChainNodeData;

  return (
    <div
      className={`min-w-[280px] rounded-lg border shadow-lg transition-all ${
        selected
          ? 'border-primary shadow-primary/20'
          : d.enabled
            ? 'border-border'
            : 'border-border/50 opacity-60'
      } bg-card`}
    >
      {/* Header */}
      <div className="flex items-center gap-2 rounded-t-lg border-b border-border bg-secondary/30 px-3 py-2">
        <div
          className={`h-2 w-2 rounded-full ${
            d.enabled ? 'bg-green-400' : 'bg-muted-foreground'
          }`}
        />
        <span className="flex-1 truncate text-sm font-medium text-foreground">
          {d.name}
        </span>
        <span className="text-xs text-muted-foreground">#{d.chainId}</span>
      </div>

      {/* Description */}
      {d.description && (
        <div className="border-b border-border/50 px-3 py-1.5 text-xs text-muted-foreground">
          {d.description}
        </div>
      )}

      {/* Counts */}
      <div className="flex items-center gap-3 px-3 py-2">
        <CountBadge icon={<Zap className="h-3 w-3" />} count={d.triggerCount} color="text-accent" />
        <CountBadge icon={<Filter className="h-3 w-3" />} count={d.conditionCount} color="text-yellow-400" />
        <CountBadge icon={<Play className="h-3 w-3" />} count={d.actionCount} color="text-green-400" />
        {d.counterCount > 0 && (
          <CountBadge icon={<Hash className="h-3 w-3" />} count={d.counterCount} color="text-purple-400" />
        )}
        <span className="ml-auto text-xs text-muted-foreground">
          {d.scopeType} P{d.priority}
        </span>
      </div>

      <Handle type="target" position={Position.Left} className="!bg-accent" />
      <Handle type="source" position={Position.Right} className="!bg-green-400" />
    </div>
  );
});

function CountBadge({
  icon,
  count,
  color,
}: {
  icon: React.ReactNode;
  count: number;
  color: string;
}) {
  return (
    <span className={`flex items-center gap-1 text-xs ${color}`}>
      {icon}
      {count}
    </span>
  );
}
