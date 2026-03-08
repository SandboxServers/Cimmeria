import type { Edge, Node } from '@xyflow/react';

export type PersistedContentGraph<
  TNodeData = Record<string, unknown>,
  TEdgeData = Record<string, unknown>,
> = {
  version: 1;
  spaceId: string;
  missionId: string | null;
  nodes: Node<TNodeData>[];
  edges: Edge<TEdgeData>[];
  selectedChainId: string;
  selectedNodeId: string;
  selectedSequenceId: string;
};

export type ChainContentLoadResult<
  TNodeData = Record<string, unknown>,
  TEdgeData = Record<string, unknown>,
> = {
  graph: PersistedContentGraph<TNodeData, TEdgeData> | null;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

export function isPersistedContentGraph<
  TNodeData = Record<string, unknown>,
  TEdgeData = Record<string, unknown>,
>(value: unknown): value is PersistedContentGraph<TNodeData, TEdgeData> {
  if (!isRecord(value)) {
    return false;
  }

  return (
    value.version === 1 &&
    typeof value.spaceId === 'string' &&
    Array.isArray(value.nodes) &&
    Array.isArray(value.edges) &&
    typeof value.selectedChainId === 'string' &&
    typeof value.selectedNodeId === 'string' &&
    typeof value.selectedSequenceId === 'string'
  );
}

type MinimalChainData = {
  kind?: string;
  spaceId?: string;
};

function isSpaceChainNode<TNodeData>(node: Node<TNodeData>, spaceId: string): boolean {
  const data = node.data as MinimalChainData;
  return data?.kind === 'chain' && data.spaceId === spaceId;
}

export function extractSpaceScopedContentSnapshot<TNodeData, TEdgeData>(
  nodes: Node<TNodeData>[],
  edges: Edge<TEdgeData>[],
  spaceId: string,
): Pick<PersistedContentGraph<TNodeData, TEdgeData>, 'nodes' | 'edges'> {
  const chainIds = new Set(
    nodes.filter((node) => isSpaceChainNode(node, spaceId)).map((node) => node.id),
  );
  const scopedNodes = nodes.filter(
    (node) => chainIds.has(node.id) || (!!node.parentId && chainIds.has(node.parentId)),
  );
  const scopedNodeIds = new Set(scopedNodes.map((node) => node.id));
  const scopedEdges = edges.filter(
    (edge) => scopedNodeIds.has(edge.source) && scopedNodeIds.has(edge.target),
  );

  return {
    edges: scopedEdges,
    nodes: scopedNodes,
  };
}

export function createPersistedContentGraph<TNodeData, TEdgeData>({
  edges,
  missionId,
  nodes,
  selectedChainId,
  selectedNodeId,
  selectedSequenceId,
  spaceId,
}: {
  edges: Edge<TEdgeData>[];
  missionId: string | null;
  nodes: Node<TNodeData>[];
  selectedChainId: string;
  selectedNodeId: string;
  selectedSequenceId: string;
  spaceId: string;
}): PersistedContentGraph<TNodeData, TEdgeData> {
  return {
    version: 1,
    spaceId,
    missionId,
    nodes,
    edges,
    selectedChainId,
    selectedNodeId,
    selectedSequenceId,
  };
}

export function createEditorSnapshotFromPersistedContentGraph<TNodeData, TEdgeData>(
  graph: PersistedContentGraph<TNodeData, TEdgeData>,
): {
  edges: Edge<TEdgeData>[];
  nodes: Node<TNodeData>[];
  selectedChainId: string;
  selectedNodeId: string;
  selectedSequenceId: string;
} {
  return {
    edges: graph.edges,
    nodes: graph.nodes,
    selectedChainId: graph.selectedChainId,
    selectedNodeId: graph.selectedNodeId,
    selectedSequenceId: graph.selectedSequenceId,
  };
}

function isTauriRuntime(): boolean {
  if (typeof window === 'undefined') {
    return false;
  }
  return '__TAURI_INTERNALS__' in window || '__TAURI__' in window;
}

async function invokeTauri<T>(command: string, args: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(command, args);
}

function buildContentStorageKey(spaceId: string, missionId: string | null): string {
  return missionId
    ? `chain-content:${spaceId}:${missionId}`
    : `chain-content:${spaceId}`;
}

export async function loadChainEditorContent<
  TNodeData = Record<string, unknown>,
  TEdgeData = Record<string, unknown>,
>(
  spaceId: string,
  missionId: string | null,
): Promise<ChainContentLoadResult<TNodeData, TEdgeData>> {
  if (isTauriRuntime()) {
    try {
      const payload = await invokeTauri<PersistedContentGraph<TNodeData, TEdgeData> | null>(
        'load_chain_editor_content',
        {
          missionId,
          spaceId,
        },
      );

      return {
        graph:
          payload && isPersistedContentGraph<TNodeData, TEdgeData>(payload) ? payload : null,
      };
    } catch {
      // Fall through to browser storage
    }
  }

  // Browser: try REST API first
  try {
    const missionParam = missionId ? `/${encodeURIComponent(missionId)}` : '';
    const res = await fetch(
      `/api/content/editor/content/${encodeURIComponent(spaceId)}${missionParam}`,
    );
    const json = await res.json();
    if (json.available && json.payload && isPersistedContentGraph<TNodeData, TEdgeData>(json.payload)) {
      return { graph: json.payload };
    }
  } catch {
    // REST API unavailable — fall through to localStorage
  }

  // Last resort: localStorage
  try {
    const raw = window.localStorage.getItem(buildContentStorageKey(spaceId, missionId));
    if (raw) {
      const parsed = JSON.parse(raw);
      if (isPersistedContentGraph<TNodeData, TEdgeData>(parsed)) {
        return { graph: parsed };
      }
    }
  } catch {
    // Corrupted localStorage entry — treat as empty
  }

  return { graph: null };
}

export async function saveChainEditorContent<
  TNodeData = Record<string, unknown>,
  TEdgeData = Record<string, unknown>,
>(
  graph: PersistedContentGraph<TNodeData, TEdgeData>,
): Promise<void> {
  if (isTauriRuntime()) {
    await invokeTauri('save_chain_editor_content', {
      missionId: graph.missionId,
      payload: graph,
      spaceId: graph.spaceId,
    });
    return;
  }

  // Browser: try REST API first
  try {
    const res = await fetch('/api/content/editor/content', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        spaceId: graph.spaceId,
        missionId: graph.missionId,
        payload: graph,
      }),
    });
    if (res.ok) {
      return;
    }
  } catch {
    // REST API unavailable — fall through to localStorage
  }

  // Last resort: localStorage
  window.localStorage.setItem(
    buildContentStorageKey(graph.spaceId, graph.missionId),
    JSON.stringify(graph),
  );
}
