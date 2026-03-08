/** @jsxImportSource react */
import type { Edge, Node } from '@xyflow/react';

export type PersistedChainEditorDraft<
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

export type ChainDraftLoadResult<
  TNodeData = Record<string, unknown>,
  TEdgeData = Record<string, unknown>,
> = {
  draft: PersistedChainEditorDraft<TNodeData, TEdgeData> | null;
  storage: 'database' | 'browser' | 'seed';
  warning?: string;
};

export type ChainDraftSaveResult = {
  storage: 'database' | 'browser';
  warning?: string;
};

const CHAIN_EDITOR_DRAFT_VERSION = 1;
const chainEditorDraftStoragePrefix = 'cimmeria.chain-editor.draft';

type MinimalChainData = {
  kind?: string;
  spaceId?: string;
};

function isSpaceChainNode<TNodeData>(node: Node<TNodeData>, spaceId: string): boolean {
  const data = node.data as MinimalChainData;
  return data?.kind === 'chain' && data.spaceId === spaceId;
}

function isPersistedDraftShape<TNodeData, TEdgeData>(
  value: unknown,
): value is PersistedChainEditorDraft<TNodeData, TEdgeData> {
  if (!value || typeof value !== 'object') {
    return false;
  }

  const candidate = value as Partial<PersistedChainEditorDraft<TNodeData, TEdgeData>>;
  return (
    candidate.version === CHAIN_EDITOR_DRAFT_VERSION &&
    typeof candidate.spaceId === 'string' &&
    Array.isArray(candidate.nodes) &&
    Array.isArray(candidate.edges)
  );
}

export function buildChainDraftStorageKey(spaceId: string, missionId: string | null): string {
  return `${chainEditorDraftStoragePrefix}:${spaceId}:${missionId ?? 'all'}`;
}

export function extractSpaceScopedEditorSnapshot<TNodeData, TEdgeData>(
  nodes: Node<TNodeData>[],
  edges: Edge<TEdgeData>[],
  spaceId: string,
): Pick<PersistedChainEditorDraft<TNodeData, TEdgeData>, 'nodes' | 'edges'> {
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

export function createPersistedChainEditorDraft<TNodeData, TEdgeData>({
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
}): PersistedChainEditorDraft<TNodeData, TEdgeData> {
  return {
    version: CHAIN_EDITOR_DRAFT_VERSION,
    spaceId,
    missionId,
    nodes,
    edges,
    selectedChainId,
    selectedNodeId,
    selectedSequenceId,
  };
}

export function mergeSpaceScopedEditorSnapshot<TNodeData, TEdgeData>(
  baseNodes: Node<TNodeData>[],
  baseEdges: Edge<TEdgeData>[],
  spaceId: string,
  draft: Pick<PersistedChainEditorDraft<TNodeData, TEdgeData>, 'nodes' | 'edges'>,
): Pick<PersistedChainEditorDraft<TNodeData, TEdgeData>, 'nodes' | 'edges'> {
  const existingScoped = extractSpaceScopedEditorSnapshot(baseNodes, baseEdges, spaceId);
  const existingScopedIds = new Set(existingScoped.nodes.map((node) => node.id));

  return {
    nodes: [
      ...baseNodes.filter((node) => !existingScopedIds.has(node.id)),
      ...draft.nodes,
    ],
    edges: [
      ...baseEdges.filter(
        (edge) => !existingScopedIds.has(edge.source) && !existingScopedIds.has(edge.target),
      ),
      ...draft.edges,
    ],
  };
}

function canUseBrowserStorage(): boolean {
  return typeof window !== 'undefined' && typeof window.localStorage !== 'undefined';
}

function isTauriRuntime(): boolean {
  if (typeof window === 'undefined') {
    return false;
  }

  return '__TAURI_INTERNALS__' in window || '__TAURI__' in window;
}

function readBrowserDraft<TNodeData, TEdgeData>(
  spaceId: string,
  missionId: string | null,
): PersistedChainEditorDraft<TNodeData, TEdgeData> | null {
  if (!canUseBrowserStorage()) {
    return null;
  }

  try {
    const rawValue = window.localStorage.getItem(buildChainDraftStorageKey(spaceId, missionId));
    if (!rawValue) {
      return null;
    }

    const parsed = JSON.parse(rawValue);
    return isPersistedDraftShape<TNodeData, TEdgeData>(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

function writeBrowserDraft<TNodeData, TEdgeData>(
  draft: PersistedChainEditorDraft<TNodeData, TEdgeData>,
): void {
  if (!canUseBrowserStorage()) {
    return;
  }

  window.localStorage.setItem(
    buildChainDraftStorageKey(draft.spaceId, draft.missionId),
    JSON.stringify(draft),
  );
}

async function invokeTauri<T>(command: string, args: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(command, args);
}

export async function loadChainEditorDraft<TNodeData, TEdgeData>(
  spaceId: string,
  missionId: string | null,
): Promise<ChainDraftLoadResult<TNodeData, TEdgeData>> {
  if (isTauriRuntime()) {
    try {
      const payload = await invokeTauri<PersistedChainEditorDraft<TNodeData, TEdgeData> | null>(
        'load_chain_editor_draft',
        {
          missionId,
          spaceId,
        },
      );

      if (payload && isPersistedDraftShape<TNodeData, TEdgeData>(payload)) {
        return {
          draft: payload,
          storage: 'database',
        };
      }
    } catch (error) {
      const browserDraft = readBrowserDraft<TNodeData, TEdgeData>(spaceId, missionId);
      if (browserDraft) {
        return {
          draft: browserDraft,
          storage: 'browser',
          warning: error instanceof Error ? error.message : String(error),
        };
      }

      return {
        draft: null,
        storage: 'seed',
        warning: error instanceof Error ? error.message : String(error),
      };
    }
  }

  const browserDraft = readBrowserDraft<TNodeData, TEdgeData>(spaceId, missionId);
  return {
    draft: browserDraft,
    storage: browserDraft ? 'browser' : 'seed',
  };
}

export async function saveChainEditorDraft<TNodeData, TEdgeData>(
  draft: PersistedChainEditorDraft<TNodeData, TEdgeData>,
): Promise<ChainDraftSaveResult> {
  if (isTauriRuntime()) {
    try {
      await invokeTauri('save_chain_editor_draft', {
        missionId: draft.missionId,
        payload: draft,
        spaceId: draft.spaceId,
      });
      return {
        storage: 'database',
      };
    } catch (error) {
      writeBrowserDraft(draft);
      return {
        storage: 'browser',
        warning: error instanceof Error ? error.message : String(error),
      };
    }
  }

  writeBrowserDraft(draft);
  return {
    storage: 'browser',
  };
}
