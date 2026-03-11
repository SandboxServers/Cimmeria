import {
  forwardRef,
  useCallback,
  useImperativeHandle,
  useMemo,
} from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  addEdge,
  useNodesState,
  useEdgesState,
  type Connection,
  type Edge,
  type EdgeTypes,
  type Node,
  type NodeTypes,
  type OnSelectionChangeParams,
  ReactFlowProvider,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { MissionNode } from './MissionNode';
import { ChainFrameNode } from './ChainFrame';
import { SequenceThreadEdge } from './SequenceEdge';
import { chainsToFlow, flowToChains, generateNodeId } from './chainConvert';
import type {
  EditorNodeData,
  MissionNodeData,
  SequenceEdgeData,
} from './types';
import type { ChainData } from '../components/AppLayout';
import type { ScenarioTemplate } from '../lib/missionCardCatalog';

/* ----- Custom type registrations ----- */

const nodeTypes: NodeTypes = {
  missionNode: MissionNode,
  chainFrame: ChainFrameNode,
};

const edgeTypes: EdgeTypes = {
  sequenceThread: SequenceThreadEdge,
};

/* ----- Public handle exposed via ref ----- */

export interface ChainEditorHandle {
  getChainData: () => ChainData[];
  addNodeFromTemplate: (template: ScenarioTemplate, targetFrameId?: string) => void;
}

/* ----- Props ----- */

interface ChainEditorProps {
  chains: ChainData[];
  spaceId: string;
  selectedFrameId: string | null;
  onNodeSelect: (node: Node<EditorNodeData> | null) => void;
}

/* ----- Inner component (needs ReactFlowProvider context) ----- */

function ChainEditorInner({
  chains,
  spaceId,
  selectedFrameId,
  onNodeSelect,
  editorRef,
}: ChainEditorProps & { editorRef: React.Ref<ChainEditorHandle> }) {
  const initialFlow = useMemo(() => chainsToFlow(chains, spaceId), [chains, spaceId]);
  const [nodes, setNodes, onNodesChange] = useNodesState(initialFlow.nodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialFlow.edges);

  /* --- Expose imperative API --- */

  useImperativeHandle(
    editorRef,
    () => ({
      getChainData: () => flowToChains(nodes, edges, chains),

      addNodeFromTemplate: (template: ScenarioTemplate, targetFrameId?: string) => {
        const frameId = targetFrameId ?? selectedFrameId;
        if (!frameId) return;

        const nodeId = generateNodeId(frameId, template.family);
        const newNode: Node<EditorNodeData> = {
          id: nodeId,
          type: 'missionNode',
          parentId: frameId,
          extent: 'parent' as const,
          position: { x: 300, y: 200 },
          data: {
            kind: 'mission',
            family: template.family,
            stage: template.stage,
            title: template.title,
            detail: template.detail,
            status: template.status,
            scenario: template.scenario,
            accent: template.accent,
            properties: [...template.properties],
          } satisfies MissionNodeData,
        };

        setNodes((prev) => [...prev, newNode]);
      },
    }),
    [nodes, edges, chains, selectedFrameId, setNodes],
  );

  /* --- Connections --- */

  const onConnect = useCallback(
    (connection: Connection) => {
      const newEdge: Edge<SequenceEdgeData> = {
        ...connection,
        id: `seq-${connection.source}-${connection.target}-${Date.now()}`,
        type: 'sequenceThread',
        data: {
          sequenceId: '',
          label: 'Sequence yarn',
          category: 'story',
          color: '#22c55e',
        },
      };
      setEdges((eds) => addEdge(newEdge, eds));
    },
    [setEdges],
  );

  /* --- Selection --- */

  const onSelectionChange = useCallback(
    ({ nodes: selected }: OnSelectionChangeParams) => {
      onNodeSelect((selected[0] as Node<EditorNodeData> | undefined) ?? null);
    },
    [onNodeSelect],
  );

  /* --- MiniMap color --- */

  const miniMapColor = useCallback((node: Node<Record<string, unknown>>) => {
    if (node.type === 'chainFrame') return 'rgba(34,197,94,0.4)';
    const data = node.data as unknown as MissionNodeData | undefined;
    switch (data?.family) {
      case 'anchor':
        return '#ff5e5b';
      case 'trigger':
        return '#13a2a4';
      case 'condition':
        return '#f5aa31';
      case 'action':
        return '#22c55e';
      case 'counter':
        return '#94a3b8';
      default:
        return '#64748b';
    }
  }, []);

  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      nodeTypes={nodeTypes}
      edgeTypes={edgeTypes}
      onNodesChange={onNodesChange}
      onEdgesChange={onEdgesChange}
      onConnect={onConnect}
      onSelectionChange={onSelectionChange}
      fitView
      fitViewOptions={{ padding: 0.12 }}
      minZoom={0.02}
      maxZoom={2}
      snapToGrid
      snapGrid={[12, 12]}
      proOptions={{ hideAttribution: true }}
      defaultEdgeOptions={{ type: 'sequenceThread' }}
    >
      <Background gap={24} size={1} color="rgba(255,255,255,0.04)" />
      <Controls
        showInteractive={false}
        className="!border-[rgba(255,255,255,0.08)] !bg-[rgba(9,18,28,0.92)] !shadow-lg [&_button]:!border-[rgba(255,255,255,0.08)] [&_button]:!bg-[rgba(9,18,28,0.92)] [&_button]:!fill-[rgba(224,231,239,0.72)] hover:[&_button]:!bg-[rgba(255,255,255,0.08)]"
      />
      <MiniMap
        className="!border-[rgba(255,255,255,0.08)] !bg-[rgba(9,18,28,0.8)]"
        nodeColor={miniMapColor}
        maskColor="rgba(0,0,0,0.6)"
      />
    </ReactFlow>
  );
}

/* ----- Outer wrapper with ReactFlowProvider ----- */

export const ChainEditor = forwardRef<ChainEditorHandle, ChainEditorProps>(
  function ChainEditor(props, ref) {
    return (
      <ReactFlowProvider>
        <div className="h-full w-full">
          <ChainEditorInner {...props} editorRef={ref} />
        </div>
      </ReactFlowProvider>
    );
  },
);
