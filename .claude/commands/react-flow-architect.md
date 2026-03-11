# ReactFlow Architect

> Expert patterns for building interactive graph applications with ReactFlow (@xyflow/react). Apply when working on the chain editor or any node-based visual editor.

---

## Quick Start

```tsx
import { ReactFlow, type Node, type Edge } from '@xyflow/react';

const nodes: Node[] = [
  { id: '1', position: { x: 0, y: 0 }, data: { label: 'Node 1' } },
  { id: '2', position: { x: 100, y: 100 }, data: { label: 'Node 2' } },
];

const edges: Edge[] = [{ id: 'e1-2', source: '1', target: '2' }];

export default function Graph() {
  return <ReactFlow nodes={nodes} edges={edges} />;
}
```

## Core Patterns

### Hierarchical Tree Navigation

```typescript
interface TreeNode extends Node {
  data: {
    label: string;
    level: number;
    hasChildren: boolean;
    isExpanded: boolean;
    childCount: number;
    category: 'root' | 'category' | 'process' | 'detail';
  };
}
```

Build visible nodes incrementally — start with roots, recursively add visible children based on expanded state.

### Performance Optimization

#### Memoize Node Components

```tsx
const ProcessNode = memo(({ data, selected }: NodeProps) => {
  return (
    <div className={`process-node ${selected ? 'selected' : ''}`}>
      {data.label}
    </div>
  );
}, (prevProps, nextProps) => {
  return (
    prevProps.data.label === nextProps.data.label &&
    prevProps.selected === nextProps.selected &&
    prevProps.data.isExpanded === nextProps.data.isExpanded
  );
});
```

#### Memoize Edge Styles

```typescript
const styledEdges = useMemo(() => {
  return edges.map(edge => ({
    ...edge,
    style: {
      strokeWidth: selectedEdgeId === edge.id ? 3 : 2,
      stroke: selectedEdgeId === edge.id ? '#3b82f6' : '#94a3b8',
    },
    animated: selectedEdgeId === edge.id,
  }));
}, [edges, selectedEdgeId]);
```

### State Management

#### Reducer Pattern for Graph State

```typescript
type GraphAction =
  | { type: 'SELECT_NODE'; payload: string }
  | { type: 'SELECT_EDGE'; payload: string }
  | { type: 'TOGGLE_EXPAND'; payload: string }
  | { type: 'UPDATE_NODES'; payload: Node[] }
  | { type: 'UPDATE_EDGES'; payload: Edge[] }
  | { type: 'UNDO' }
  | { type: 'REDO' };
```

#### History Management (Undo/Redo)

```typescript
const useHistoryManager = (state: GraphState, dispatch: Dispatch<GraphAction>) => {
  const canUndo = state.historyIndex > 0;
  const canRedo = state.historyIndex < state.history.length - 1;

  const undo = useCallback(() => {
    if (canUndo) dispatch({ type: 'UNDO' });
  }, [canUndo]);

  const saveToHistory = useCallback(() => {
    dispatch({ type: 'SAVE_TO_HISTORY' });
  }, []);

  return { canUndo, canRedo, undo, redo, saveToHistory };
};
```

## Auto-Layout with Dagre

```typescript
import dagre from 'dagre';

const layoutOptions = {
  rankdir: 'TB',
  nodesep: 100,
  ranksep: 150,
  marginx: 50,
  marginy: 50,
};

const applyLayout = (nodes: Node[], edges: Edge[]) => {
  const g = new dagre.graphlib.Graph();
  g.setGraph(layoutOptions);
  g.setDefaultEdgeLabel(() => ({}));

  nodes.forEach(node => g.setNode(node.id, { width: 200, height: 100 }));
  edges.forEach(edge => g.setEdge(edge.source, edge.target));
  dagre.layout(g);

  return nodes.map(node => ({
    ...node,
    position: {
      x: g.node(node.id).x - 100,
      y: g.node(node.id).y - 50,
    },
  }));
};
```

## Best Practices

1. Use `React.memo` for all custom node components
2. Implement virtualization for graphs with 1000+ nodes
3. Debounce layout calculations during rapid interactions
4. Use `useCallback` for edge creation and manipulation
5. Use `Map` for O(1) lookups instead of `array.find`
6. Cache layout results
7. Proper cleanup in `useEffect` hooks
8. Use TypeScript discriminated unions for node data types

## Project Context

The **Chain Editor** (`ChainFlowWorkbench.react.tsx`) is the primary ReactFlow consumer:
- Uses `@xyflow/react` v12
- Implements mission chain authoring (triggers → conditions → actions)
- Features: drag-and-drop, undo/redo, validation, draft persistence
- Custom node types for different card types (mission, trigger, condition, action)
- Supports copy/paste, duplicate, templates
- Collaboration metadata and conflict detection

Key files:
- `frontend/src/react/ChainFlowWorkbench.react.tsx` — main workbench
- `frontend/src/react/chainNodeSchemas.ts` — typed node schemas
- `frontend/src/react/chainValidation.ts` — graph validation
- `frontend/src/react/chainReuse.ts` — clipboard/template workflows
- `frontend/src/react/chainLayout.ts` — graph layout engine
- `frontend/src/react/chainDraftPersistence.ts` — draft state
- `frontend/src/react/chainContentPersistence.ts` — content serialization
- `frontend/src/react/chainCollaboration.ts` — multi-user conflict detection
