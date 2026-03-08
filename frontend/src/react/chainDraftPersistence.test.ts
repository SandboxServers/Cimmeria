import { describe, expect, it } from 'vitest';
import type { Edge, Node } from '@xyflow/react';
import {
  buildChainAutosaveStorageKey,
  buildChainDraftStorageKey,
  createChainEditorAutosave,
  createPersistedChainEditorDraft,
  extractSpaceScopedEditorSnapshot,
  mergeSpaceScopedEditorSnapshot,
  shouldRecoverAutosave,
} from './chainDraftPersistence';

type TestNodeData = {
  kind: 'chain' | 'mission';
  spaceId?: string;
};

describe('chainDraftPersistence', () => {
  const nodes: Node<TestNodeData>[] = [
    {
      id: 'chain-a',
      position: { x: 0, y: 0 },
      data: { kind: 'chain', spaceId: 'Castle_CellBlock' },
    },
    {
      id: 'card-a1',
      parentId: 'chain-a',
      position: { x: 10, y: 20 },
      data: { kind: 'mission' },
    },
    {
      id: 'chain-b',
      position: { x: 0, y: 300 },
      data: { kind: 'chain', spaceId: 'Castle_Hallway' },
    },
    {
      id: 'card-b1',
      parentId: 'chain-b',
      position: { x: 10, y: 20 },
      data: { kind: 'mission' },
    },
  ];

  const edges: Edge[] = [
    { id: 'edge-a', source: 'card-a1', target: 'card-a1' },
    { id: 'edge-b', source: 'card-b1', target: 'card-b1' },
  ];

  it('extracts only the requested space draft', () => {
    const snapshot = extractSpaceScopedEditorSnapshot(nodes, edges, 'Castle_CellBlock');

    expect(snapshot.nodes.map((node) => node.id)).toEqual(['chain-a', 'card-a1']);
    expect(snapshot.edges.map((edge) => edge.id)).toEqual(['edge-a']);
  });

  it('merges a saved space draft without disturbing other spaces', () => {
    const replacementDraft = createPersistedChainEditorDraft({
      edges: [{ id: 'edge-a2', source: 'card-a2', target: 'card-a2' }],
      missionId: null,
      nodes: [
        {
          id: 'chain-a',
          position: { x: 100, y: 100 },
          data: { kind: 'chain', spaceId: 'Castle_CellBlock' },
        },
        {
          id: 'card-a2',
          parentId: 'chain-a',
          position: { x: 120, y: 140 },
          data: { kind: 'mission' },
        },
      ],
      selectedChainId: 'chain-a',
      selectedNodeId: 'card-a2',
      selectedSequenceId: 'sequence-1',
      spaceId: 'Castle_CellBlock',
    });

    const merged = mergeSpaceScopedEditorSnapshot(
      nodes,
      edges,
      'Castle_CellBlock',
      replacementDraft,
    );

    expect(merged.nodes.map((node) => node.id)).toEqual(['chain-b', 'card-b1', 'chain-a', 'card-a2']);
    expect(merged.edges.map((edge) => edge.id)).toEqual(['edge-b', 'edge-a2']);
  });

  it('builds stable browser storage keys', () => {
    expect(buildChainDraftStorageKey('Castle_CellBlock', null)).toBe(
      'cimmeria.chain-editor.draft:Castle_CellBlock:all',
    );
    expect(buildChainDraftStorageKey('Castle_CellBlock', '638')).toBe(
      'cimmeria.chain-editor.draft:Castle_CellBlock:638',
    );
    expect(buildChainAutosaveStorageKey('Castle_CellBlock', null)).toBe(
      'cimmeria.chain-editor.autosave:Castle_CellBlock:all',
    );
  });

  it('flags autosave recovery only when the autosave differs from the saved signature', () => {
    const draft = createPersistedChainEditorDraft({
      edges: [{ id: 'edge-a2', source: 'card-a2', target: 'card-a2' }],
      missionId: null,
      nodes: [
        {
          id: 'chain-a',
          position: { x: 100, y: 100 },
          data: { kind: 'chain', spaceId: 'Castle_CellBlock' },
        },
      ],
      selectedChainId: 'chain-a',
      selectedNodeId: '',
      selectedSequenceId: '',
      spaceId: 'Castle_CellBlock',
    });
    const autosave = createChainEditorAutosave({
      autosavedAt: '2026-03-07T12:00:00.000Z',
      draft,
      savedSignature: 'saved-signature-v1',
    });

    expect(shouldRecoverAutosave(autosave, 'saved-signature-v2')).toBe(true);
    expect(shouldRecoverAutosave(autosave, 'saved-signature-v1')).toBe(false);
    expect(shouldRecoverAutosave(null, 'saved-signature-v1')).toBe(false);
  });
});
