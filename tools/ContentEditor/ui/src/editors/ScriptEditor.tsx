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
import { ScriptNode } from './ScriptNode';
import { ScriptEdge } from './ScriptEdge';
import { ScriptCommentNode } from './ScriptCommentNode';
import type {
  ScriptCommentNodeData,
  ScriptConnectionData,
  ScriptEdgeData,
  ScriptFileData,
  ScriptNodeData,
  ScriptNodeInstance,
  ScriptNodeTemplate,
} from './scriptTypes';
import { nodeTypeColors, portTypeColors } from './scriptTypes';

/* ----- Custom type registrations ----- */

const nodeTypes: NodeTypes = {
  scriptNode: ScriptNode,
  scriptComment: ScriptCommentNode,
};

const edgeTypes: EdgeTypes = {
  scriptEdge: ScriptEdge,
};

/* ----- Public handle exposed via ref ----- */

export interface ScriptEditorHandle {
  getScriptData: () => ScriptFileData;
  addNode: (templateRef: string, position?: { x: number; y: number }) => void;
}

/* ----- Props ----- */

interface ScriptEditorProps {
  script: ScriptFileData;
  templates: ScriptNodeTemplate[];
  onNodeSelect: (nodeId: number | null) => void;
}

/* ----- Conversion helpers ----- */

function findTemplate(
  templates: ScriptNodeTemplate[],
  refName: string,
): ScriptNodeTemplate | null {
  return templates.find((t) => t.ref_name === refName) ?? null;
}

/**
 * Determine which ports on an instance are connected so we know which
 * hidden-by-default ports to show.
 */
function getConnectedPorts(
  nodeId: number,
  connections: ScriptConnectionData[],
): Set<string> {
  const connected = new Set<string>();
  for (const conn of connections) {
    if (conn.out_node === nodeId) connected.add(`out-${conn.out_port}`);
    if (conn.in_node === nodeId) connected.add(`in-${conn.in_port}`);
  }
  return connected;
}

/**
 * Convert a ScriptNodeInstance + its template into xyflow Node<ScriptNodeData>.
 */
function instanceToFlowNode(
  inst: ScriptNodeInstance,
  template: ScriptNodeTemplate | null,
  connections: ScriptConnectionData[],
): Node<ScriptNodeData> {
  const connectedPorts = getConnectedPorts(inst.id, connections);
  const propsMap: Record<string, string> = {};
  for (const [k, v] of inst.properties) {
    propsMap[k] = v;
  }

  const inputPorts = (template?.ports ?? [])
    .filter((p) => p.direction === 'in')
    .map((p) => ({
      name: p.name,
      portType: p.port_type,
      visible: !p.default_hide || connectedPorts.has(`in-${p.name}`),
    }));

  const outputPorts = (template?.ports ?? [])
    .filter((p) => p.direction === 'out')
    .map((p) => ({
      name: p.name,
      portType: p.port_type,
      visible: !p.default_hide || connectedPorts.has(`out-${p.name}`),
    }));

  return {
    id: `script-${inst.id}`,
    type: 'scriptNode',
    position: { x: inst.x, y: inst.y },
    data: {
      nodeId: inst.id,
      templateRef: inst.ref_name,
      displayName: template?.display_name ?? inst.ref_name,
      nodeType: template?.node_type ?? 'Action',
      category: template?.category ?? '',
      description: template?.description ?? '',
      inputPorts,
      outputPorts,
      properties: propsMap,
      enabled: propsMap['_enabled'] !== 'false',
      debug: propsMap['_debug'] === 'true',
      comment: propsMap['_comment'] ?? '',
    },
  };
}

/**
 * Convert a ScriptConnectionData into an xyflow Edge<ScriptEdgeData>.
 */
function connectionToFlowEdge(
  conn: ScriptConnectionData,
  index: number,
  templates: ScriptNodeTemplate[],
  instances: ScriptNodeInstance[],
): Edge<ScriptEdgeData> {
  // Determine port type from the source node's output port
  let portType = '';
  const srcInstance = instances.find((n) => n.id === conn.out_node);
  if (srcInstance) {
    const srcTemplate = findTemplate(templates, srcInstance.ref_name);
    const srcPort = srcTemplate?.ports.find(
      (p) => p.name === conn.out_port && p.direction === 'out',
    );
    if (srcPort) portType = srcPort.port_type;
  }

  return {
    id: `sedge-${conn.out_node}-${conn.out_port}-${conn.in_node}-${conn.in_port}-${index}`,
    source: `script-${conn.out_node}`,
    sourceHandle: `out-${conn.out_port}`,
    target: `script-${conn.in_node}`,
    targetHandle: `in-${conn.in_port}`,
    type: 'scriptEdge',
    data: { portType },
  };
}

/**
 * Convert ScriptFileData into xyflow nodes and edges.
 */
function scriptToFlow(
  script: ScriptFileData,
  templates: ScriptNodeTemplate[],
): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = [];
  const edges: Edge[] = [];

  // Script nodes
  for (const inst of script.nodes) {
    const template = findTemplate(templates, inst.ref_name);
    nodes.push(instanceToFlowNode(inst, template, script.connections));
  }

  // Comment nodes
  for (const comment of script.comments) {
    const commentNode: Node<ScriptCommentNodeData> = {
      id: `comment-${comment.id}`,
      type: 'scriptComment',
      position: { x: comment.x, y: comment.y },
      data: {
        commentId: comment.id,
        text: comment.text,
        color: comment.color,
        width: comment.width,
        height: comment.height,
      },
      selectable: true,
      draggable: true,
    };
    nodes.push(commentNode);
  }

  // Connections
  script.connections.forEach((conn, index) => {
    edges.push(connectionToFlowEdge(conn, index, templates, script.nodes));
  });

  return { nodes, edges };
}

/**
 * Convert current xyflow state back into ScriptFileData for saving.
 */
function flowToScript(
  nodes: Node[],
  edges: Edge[],
  baseScript: ScriptFileData,
): ScriptFileData {
  const scriptNodes: ScriptNodeInstance[] = [];
  const comments = [...baseScript.comments];
  let maxId = baseScript.next_id;

  for (const node of nodes) {
    if (node.type === 'scriptNode') {
      const data = node.data as ScriptNodeData;
      const props: Array<[string, string]> = Object.entries(data.properties);
      scriptNodes.push({
        id: data.nodeId,
        ref_name: data.templateRef,
        x: Math.round(node.position.x),
        y: Math.round(node.position.y),
        properties: props,
        ports: [], // Port visibility state; currently not tracked beyond connections
      });
      if (data.nodeId >= maxId) maxId = data.nodeId + 1;
    } else if (node.type === 'scriptComment') {
      const data = node.data as ScriptCommentNodeData;
      const existing = comments.find((c) => c.id === data.commentId);
      if (existing) {
        existing.x = Math.round(node.position.x);
        existing.y = Math.round(node.position.y);
        existing.text = data.text;
        existing.color = data.color;
        existing.width = data.width;
        existing.height = data.height;
      }
    }
  }

  const connections: ScriptConnectionData[] = edges
    .filter((e) => e.sourceHandle && e.targetHandle)
    .map((e) => ({
      out_node: parseInt(e.source.replace('script-', ''), 10),
      out_port: e.sourceHandle!.replace('out-', ''),
      in_node: parseInt(e.target.replace('script-', ''), 10),
      in_port: e.targetHandle!.replace('in-', ''),
    }));

  return {
    ...baseScript,
    next_id: maxId,
    nodes: scriptNodes,
    connections,
    comments,
  };
}

/* ----- Port type compatibility check ----- */

/**
 * Checks whether two ports can be connected based on their types.
 * Only same-type ports can connect (Trigger->Trigger, Boolean->Boolean, etc.).
 */
function arePortsCompatible(
  sourceHandleId: string,
  targetHandleId: string,
  sourceNodeId: string,
  targetNodeId: string,
  nodes: Node[],
): boolean {
  const srcNode = nodes.find((n) => n.id === sourceNodeId);
  const tgtNode = nodes.find((n) => n.id === targetNodeId);
  if (!srcNode || !tgtNode) return false;
  if (srcNode.type !== 'scriptNode' || tgtNode.type !== 'scriptNode') return false;

  const srcData = srcNode.data as ScriptNodeData;
  const tgtData = tgtNode.data as ScriptNodeData;

  const srcPortName = sourceHandleId.replace('out-', '');
  const tgtPortName = targetHandleId.replace('in-', '');

  const srcPort = srcData.outputPorts.find((p) => p.name === srcPortName);
  const tgtPort = tgtData.inputPorts.find((p) => p.name === tgtPortName);

  if (!srcPort || !tgtPort) return false;
  return srcPort.portType === tgtPort.portType;
}

/* ----- Inner component (needs ReactFlowProvider context) ----- */

function ScriptEditorInner({
  script,
  templates,
  onNodeSelect,
  editorRef,
}: ScriptEditorProps & { editorRef: React.Ref<ScriptEditorHandle> }) {
  const initialFlow = useMemo(() => scriptToFlow(script, templates), [script, templates]);
  const [nodes, setNodes, onNodesChange] = useNodesState(initialFlow.nodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialFlow.edges);

  /* --- Expose imperative API --- */

  useImperativeHandle(
    editorRef,
    () => ({
      getScriptData: () => flowToScript(nodes, edges, script),

      addNode: (templateRef: string, position?: { x: number; y: number }) => {
        const template = findTemplate(templates, templateRef);
        if (!template) return;

        const nodeId = script.next_id + nodes.filter((n) => n.type === 'scriptNode').length;
        const pos = position ?? { x: 200, y: 200 };

        const newNode = instanceToFlowNode(
          {
            id: nodeId,
            ref_name: templateRef,
            x: pos.x,
            y: pos.y,
            properties: template.properties.map((p) => [p.name, p.default_value] as [string, string]),
            ports: [],
          },
          template,
          [],
        );

        setNodes((prev) => [...prev, newNode]);
      },
    }),
    [nodes, edges, script, templates, setNodes],
  );

  /* --- Connections --- */

  const onConnect = useCallback(
    (connection: Connection) => {
      // Validate port type compatibility
      if (
        connection.sourceHandle &&
        connection.targetHandle &&
        connection.source &&
        connection.target &&
        !arePortsCompatible(
          connection.sourceHandle,
          connection.targetHandle,
          connection.source,
          connection.target,
          nodes,
        )
      ) {
        return; // Incompatible port types - reject connection
      }

      // Determine port type for the edge color
      let portType = '';
      if (connection.source && connection.sourceHandle) {
        const srcNode = nodes.find((n) => n.id === connection.source);
        if (srcNode?.type === 'scriptNode') {
          const srcData = srcNode.data as ScriptNodeData;
          const portName = connection.sourceHandle.replace('out-', '');
          const port = srcData.outputPorts.find((p) => p.name === portName);
          if (port) portType = port.portType;
        }
      }

      const newEdge: Edge<ScriptEdgeData> = {
        ...connection,
        id: `sedge-${connection.source}-${connection.sourceHandle}-${connection.target}-${connection.targetHandle}-${Date.now()}`,
        type: 'scriptEdge',
        data: { portType },
      };
      setEdges((eds) => addEdge(newEdge, eds));
    },
    [nodes, setEdges],
  );

  /* --- Selection --- */

  const onSelectionChange = useCallback(
    ({ nodes: selected }: OnSelectionChangeParams) => {
      if (selected.length === 0) {
        onNodeSelect(null);
        return;
      }
      const first = selected[0];
      if (first.type === 'scriptNode') {
        const data = first.data as ScriptNodeData;
        onNodeSelect(data.nodeId);
      } else {
        onNodeSelect(null);
      }
    },
    [onNodeSelect],
  );

  /* --- MiniMap color --- */

  const miniMapColor = useCallback((node: Node<Record<string, unknown>>) => {
    if (node.type === 'scriptComment') return 'rgba(148,163,184,0.3)';
    const data = node.data as unknown as ScriptNodeData | undefined;
    const colors = nodeTypeColors[data?.nodeType ?? ''];
    return colors?.border ?? '#64748b';
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
      fitViewOptions={{ padding: 0.15 }}
      minZoom={0.05}
      maxZoom={2.5}
      snapToGrid
      snapGrid={[12, 12]}
      proOptions={{ hideAttribution: true }}
      defaultEdgeOptions={{ type: 'scriptEdge' }}
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

export const ScriptEditor = forwardRef<ScriptEditorHandle, ScriptEditorProps>(
  function ScriptEditor(props, ref) {
    return (
      <ReactFlowProvider>
        <div className="h-full w-full">
          <ScriptEditorInner {...props} editorRef={ref} />
        </div>
      </ReactFlowProvider>
    );
  },
);
