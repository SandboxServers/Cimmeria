import type { Edge, Node } from '@xyflow/react';

type WithKind = {
  kind?: string;
};

type WithParent = {
  parentId?: string;
};

type SequenceEdgeLike = {
  sequenceId?: string;
  label?: string;
  category?: string;
  color?: string;
};

export type ChainReuseBundle<TNodeData = Record<string, unknown>, TEdgeData = Record<string, unknown>> = {
  version: 1;
  kind: 'card' | 'chain';
  label: string;
  copiedAt: string;
  nodes: Node<TNodeData>[];
  edges: Edge<TEdgeData>[];
};

export type ChainTemplateRecord<
  TNodeData = Record<string, unknown>,
  TEdgeData = Record<string, unknown>,
> = {
  id: string;
  name: string;
  savedAt: string;
  bundle: ChainReuseBundle<TNodeData, TEdgeData>;
};

type ChainTemplateShape<TNodeData = Record<string, unknown>, TEdgeData = Record<string, unknown>> =
  ChainTemplateRecord<TNodeData, TEdgeData>[];

type ReuseAllocators = {
  nextChainId: () => string;
  nextNodeId: () => string;
  nextSequenceId: () => string;
  nextTemplateId?: () => string;
};

type InstantiateResult<TNodeData = Record<string, unknown>, TEdgeData = Record<string, unknown>> = {
  edges: Edge<TEdgeData>[];
  insertedNodeIds: string[];
  nodes: Node<TNodeData>[];
  selectedChainId: string;
  selectedNodeId: string;
  selectedSequenceId: string;
};

const reuseClipboardStorageKey = 'cimmeria.chain-editor.reuse-clipboard';
const chainTemplateStorageKey = 'cimmeria.chain-editor.templates';

function isObject(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object';
}

function canUseBrowserStorage(): boolean {
  return typeof window !== 'undefined' && typeof window.localStorage !== 'undefined';
}

function isChainNode<TNodeData>(node: Node<TNodeData>): boolean {
  return (node.data as WithKind | undefined)?.kind === 'chain';
}

function isChildOfChain<TNodeData>(node: Node<TNodeData>, chainId: string): boolean {
  return (node as WithParent).parentId === chainId;
}

function isReuseBundleShape<TNodeData, TEdgeData>(
  value: unknown,
): value is ChainReuseBundle<TNodeData, TEdgeData> {
  if (!isObject(value)) {
    return false;
  }

  return (
    value.version === 1 &&
    (value.kind === 'card' || value.kind === 'chain') &&
    typeof value.label === 'string' &&
    typeof value.copiedAt === 'string' &&
    Array.isArray(value.nodes) &&
    Array.isArray(value.edges)
  );
}

function isTemplateArrayShape<TNodeData, TEdgeData>(
  value: unknown,
): value is ChainTemplateShape<TNodeData, TEdgeData> {
  return (
    Array.isArray(value) &&
    value.every((entry) => {
      if (!isObject(entry)) {
        return false;
      }

      return (
        typeof entry.id === 'string' &&
        typeof entry.name === 'string' &&
        typeof entry.savedAt === 'string' &&
        isReuseBundleShape<TNodeData, TEdgeData>(entry.bundle)
      );
    })
  );
}

export function createCardReuseBundle<TNodeData, TEdgeData>(
  nodes: Node<TNodeData>[],
  nodeId: string,
): ChainReuseBundle<TNodeData, TEdgeData> | null {
  const cardNode = nodes.find((node) => node.id === nodeId);
  if (!cardNode || isChainNode(cardNode)) {
    return null;
  }

  return {
    version: 1,
    kind: 'card',
    label: 'Copied card',
    copiedAt: new Date().toISOString(),
    nodes: [structuredClone(cardNode)],
    edges: [],
  };
}

export function createChainReuseBundle<TNodeData, TEdgeData>(
  nodes: Node<TNodeData>[],
  edges: Edge<TEdgeData>[],
  chainId: string,
): ChainReuseBundle<TNodeData, TEdgeData> | null {
  const chainNode = nodes.find((node) => node.id === chainId && isChainNode(node));
  if (!chainNode) {
    return null;
  }

  const childNodes = nodes.filter((node) => isChildOfChain(node, chainId));
  const includedIds = new Set([chainNode.id, ...childNodes.map((node) => node.id)]);
  const internalEdges = edges.filter(
    (edge) => includedIds.has(edge.source) && includedIds.has(edge.target),
  );

  return {
    version: 1,
    kind: 'chain',
    label: 'Copied chain',
    copiedAt: new Date().toISOString(),
    nodes: structuredClone([chainNode, ...childNodes]),
    edges: structuredClone(internalEdges),
  };
}

function offsetNodePosition<TNodeData>(node: Node<TNodeData>, x: number, y: number): Node<TNodeData> {
  return {
    ...node,
    position: {
      x: node.position.x + x,
      y: node.position.y + y,
    },
  };
}

function remapSequenceEdge<TNodeData, TEdgeData>(
  edge: Edge<TEdgeData>,
  idMap: Map<string, string>,
  sequenceIdMap: Map<string, string>,
  nextSequenceId: () => string,
): Edge<TEdgeData> {
  const data = edge.data as SequenceEdgeLike | undefined;
  const priorSequenceId = typeof data?.sequenceId === 'string' ? data.sequenceId : null;
  let nextData = edge.data;

  if (priorSequenceId) {
    const mappedSequenceId = sequenceIdMap.get(priorSequenceId) ?? nextSequenceId();
    if (!sequenceIdMap.has(priorSequenceId)) {
      sequenceIdMap.set(priorSequenceId, mappedSequenceId);
    }

    nextData = {
      ...(edge.data as Record<string, unknown> | undefined),
      sequenceId: mappedSequenceId,
    } as TEdgeData;
  }

  return {
    ...edge,
    id: `${idMap.get(edge.source) ?? edge.source}::${idMap.get(edge.target) ?? edge.target}::${
      edge.sourceHandle ?? 'source'
    }::${edge.targetHandle ?? 'target'}::${Math.random().toString(36).slice(2, 10)}`,
    source: idMap.get(edge.source) ?? edge.source,
    target: idMap.get(edge.target) ?? edge.target,
    data: nextData,
  };
}

export function instantiateReuseBundle<TNodeData, TEdgeData>(
  bundle: ChainReuseBundle<TNodeData, TEdgeData>,
  allocators: ReuseAllocators,
  options: {
    targetChainId?: string;
    offset?: { x: number; y: number };
  } = {},
): InstantiateResult<TNodeData, TEdgeData> {
  const offset = options.offset ?? { x: 64, y: 48 };
  const idMap = new Map<string, string>();
  const insertedNodes: Node<TNodeData>[] = [];
  const insertedEdges: Edge<TEdgeData>[] = [];
  const sequenceIdMap = new Map<string, string>();

  if (bundle.kind === 'card') {
    const [cardNode] = bundle.nodes;
    if (!cardNode || isChainNode(cardNode)) {
      throw new Error('clipboard does not contain a reusable card');
    }
    if (!options.targetChainId) {
      throw new Error('select a target chain before pasting a copied card');
    }

    const nextId = allocators.nextNodeId();
    idMap.set(cardNode.id, nextId);
    insertedNodes.push(
      offsetNodePosition(
        {
          ...structuredClone(cardNode),
          id: nextId,
          parentId: options.targetChainId,
        },
        offset.x,
        offset.y,
      ),
    );

    return {
      nodes: insertedNodes,
      edges: [],
      insertedNodeIds: insertedNodes.map((node) => node.id),
      selectedChainId: options.targetChainId,
      selectedNodeId: nextId,
      selectedSequenceId: '',
    };
  }

  const frameNode = bundle.nodes.find((node) => isChainNode(node));
  if (!frameNode) {
    throw new Error('clipboard does not contain a reusable chain frame');
  }

  const chainChildren = bundle.nodes.filter((node) => isChildOfChain(node, frameNode.id));
  const nextChainId = allocators.nextChainId();
  idMap.set(frameNode.id, nextChainId);
  insertedNodes.push(
    offsetNodePosition(
      {
        ...structuredClone(frameNode),
        id: nextChainId,
      },
      offset.x,
      offset.y,
    ),
  );

  for (const childNode of chainChildren) {
    const nextNodeId = allocators.nextNodeId();
    idMap.set(childNode.id, nextNodeId);
    insertedNodes.push(
      offsetNodePosition(
        {
          ...structuredClone(childNode),
          id: nextNodeId,
          parentId: nextChainId,
        },
        offset.x,
        offset.y,
      ),
    );
  }

  for (const edge of bundle.edges) {
    insertedEdges.push(remapSequenceEdge(edge, idMap, sequenceIdMap, allocators.nextSequenceId));
  }

  const firstMissionNode = insertedNodes.find((node) => !isChainNode(node));
  const selectedSequenceId =
    (insertedEdges.find((edge) => {
      const data = edge.data as SequenceEdgeLike | undefined;
      return typeof data?.sequenceId === 'string';
    })?.data as SequenceEdgeLike | undefined)?.sequenceId ?? '';

  return {
    nodes: insertedNodes,
    edges: insertedEdges,
    insertedNodeIds: insertedNodes.map((node) => node.id),
    selectedChainId: nextChainId,
    selectedNodeId: firstMissionNode?.id ?? '',
    selectedSequenceId,
  };
}

export function duplicateCard<TNodeData, TEdgeData>(
  nodes: Node<TNodeData>[],
  nodeId: string,
  targetChainId: string,
  allocators: ReuseAllocators,
): InstantiateResult<TNodeData, TEdgeData> {
  const bundle = createCardReuseBundle<TNodeData, TEdgeData>(nodes, nodeId);
  if (!bundle) {
    throw new Error('select a card before duplicating it');
  }

  return instantiateReuseBundle(bundle, allocators, {
    targetChainId,
  });
}

export function duplicateChain<TNodeData, TEdgeData>(
  nodes: Node<TNodeData>[],
  edges: Edge<TEdgeData>[],
  chainId: string,
  allocators: ReuseAllocators,
): InstantiateResult<TNodeData, TEdgeData> {
  const bundle = createChainReuseBundle(nodes, edges, chainId);
  if (!bundle) {
    throw new Error('select a chain before duplicating it');
  }

  return instantiateReuseBundle(bundle, allocators);
}

export function saveReuseClipboard<TNodeData, TEdgeData>(
  bundle: ChainReuseBundle<TNodeData, TEdgeData>,
): void {
  if (!canUseBrowserStorage()) {
    return;
  }

  window.localStorage.setItem(reuseClipboardStorageKey, JSON.stringify(bundle));
}

export function loadReuseClipboard<TNodeData, TEdgeData>():
  | ChainReuseBundle<TNodeData, TEdgeData>
  | null {
  if (!canUseBrowserStorage()) {
    return null;
  }

  try {
    const rawValue = window.localStorage.getItem(reuseClipboardStorageKey);
    if (!rawValue) {
      return null;
    }

    const parsed = JSON.parse(rawValue);
    return isReuseBundleShape<TNodeData, TEdgeData>(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

export function clearReuseClipboard(): void {
  if (!canUseBrowserStorage()) {
    return;
  }

  window.localStorage.removeItem(reuseClipboardStorageKey);
}

export function loadChainTemplates<TNodeData, TEdgeData>(): ChainTemplateShape<TNodeData, TEdgeData> {
  if (!canUseBrowserStorage()) {
    return [];
  }

  try {
    const rawValue = window.localStorage.getItem(chainTemplateStorageKey);
    if (!rawValue) {
      return [];
    }

    const parsed = JSON.parse(rawValue);
    return isTemplateArrayShape<TNodeData, TEdgeData>(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

export function saveChainTemplate<TNodeData, TEdgeData>(
  name: string,
  bundle: ChainReuseBundle<TNodeData, TEdgeData>,
  allocators: ReuseAllocators,
): ChainTemplateRecord<TNodeData, TEdgeData> {
  if (!canUseBrowserStorage()) {
    throw new Error('template storage is only available in the browser');
  }

  const nextTemplate: ChainTemplateRecord<TNodeData, TEdgeData> = {
    id: allocators.nextTemplateId ? allocators.nextTemplateId() : `template-${Date.now()}`,
    name,
    savedAt: new Date().toISOString(),
    bundle,
  };
  const current = loadChainTemplates<TNodeData, TEdgeData>();
  const nextTemplates = [nextTemplate, ...current].slice(0, 20);
  window.localStorage.setItem(chainTemplateStorageKey, JSON.stringify(nextTemplates));
  return nextTemplate;
}

export function deleteChainTemplate(templateId: string): void {
  if (!canUseBrowserStorage()) {
    return;
  }

  const current = loadChainTemplates();
  const nextTemplates = current.filter((template) => template.id !== templateId);
  window.localStorage.setItem(chainTemplateStorageKey, JSON.stringify(nextTemplates));
}
