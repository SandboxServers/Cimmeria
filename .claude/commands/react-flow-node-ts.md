# React Flow Node TypeScript Patterns

> Patterns for creating custom ReactFlow node components with proper TypeScript types, handles, and state integration. Apply when adding new node types to the chain editor.

---

## Node Component Template

```tsx
import { memo } from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';

interface MyNodeData extends Record<string, unknown> {
  title: string;
  description?: string;
  category: string;
}

type MyNodeType = Node<MyNodeData, 'my-node'>;

export const MyNode = memo(function MyNode({
  id,
  data,
  selected,
}: NodeProps<MyNodeType>) {
  return (
    <div className={`rounded-lg border p-3 ${selected ? 'ring-2 ring-blue-500' : ''}`}>
      <Handle type="target" position={Position.Top} />
      <div className="font-medium">{data.title}</div>
      {data.description && (
        <div className="text-sm text-muted">{data.description}</div>
      )}
      <Handle type="source" position={Position.Bottom} />
    </div>
  );
}, (prev, next) => {
  return (
    prev.data.title === next.data.title &&
    prev.data.description === next.data.description &&
    prev.selected === next.selected
  );
});
```

## Type Definition Pattern

```typescript
// Define node data interface
export interface MyNodeData extends Record<string, unknown> {
  title: string;
  description?: string;
}

// Create typed node alias
export type MyNode = Node<MyNodeData, 'my-node'>;

// Register in nodeTypes map
const nodeTypes = {
  'my-node': MyNode,
  // ... other node types
};
```

## Handle Configuration

```tsx
// Multiple handles with IDs
<Handle type="target" position={Position.Top} id="input-main" />
<Handle type="target" position={Position.Left} id="input-alt" />
<Handle type="source" position={Position.Bottom} id="output-success" />
<Handle type="source" position={Position.Right} id="output-failure" />
```

### Conditional Handles

```tsx
{data.hasBranching && (
  <Handle
    type="source"
    position={Position.Right}
    id="branch"
    className="bg-yellow-500"
  />
)}
```

## Integration Steps for New Node Types

1. Define the data interface in types file
2. Create the component in `frontend/src/react/`
3. Add custom equality check in `memo()` for performance
4. Register in the `nodeTypes` map passed to `<ReactFlow>`
5. Add to schema definitions in `chainNodeSchemas.ts`
6. Add validation rules in `chainValidation.ts`
7. Update the mission card library catalog if applicable

## Styling Conventions

```tsx
// Use Tailwind classes directly on node wrapper
<div className={cn(
  'rounded-lg border bg-card p-3 shadow-sm',
  'transition-shadow duration-150',
  selected && 'ring-2 ring-primary shadow-md',
  data.hasError && 'border-destructive',
)}>
```

## Project Context

The chain editor uses typed node schemas defined in `chainNodeSchemas.ts`:
- **Trigger nodes** — events that start a chain
- **Condition nodes** — checks that gate progression
- **Action nodes** — things that happen when conditions are met
- **Mission nodes** — high-level mission references

Each node type has structured form fields (not freeform) with DB-backed pickers for game content (spaces, missions, dialogs, items, regions, mission steps).
