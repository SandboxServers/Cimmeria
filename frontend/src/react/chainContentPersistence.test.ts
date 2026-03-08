import { describe, expect, it } from 'vitest';
import type { Edge, Node } from '@xyflow/react';
import {
  createEditorSnapshotFromPersistedContentGraph,
  createPersistedContentGraph,
  extractSpaceScopedContentSnapshot,
  isPersistedContentGraph,
} from './chainContentPersistence';

type TestNodeData =
  | {
      kind: 'chain';
      name: string;
      summary: string;
      scopeType: 'mission' | 'space' | 'effect' | 'global';
      scopeId: string;
      enabled: boolean;
      priority: number;
      source: string;
      semantic: string;
      color: string;
      spaceId: string;
      missionId: string | null;
    }
  | {
      kind: 'mission';
      family: string;
      stage: string;
      title: string;
      detail: string;
      status: string;
      scenario: string;
      accent: string;
      manualPosition?: boolean;
      sortOrder?: number;
      inputs?: string[];
      outputs?: string[];
      outputConditions?: string[];
      properties: Array<{ label: string; value: string }>;
    };

type TestEdgeData = {
  sequenceId: string;
  label: string;
  category: string;
  color: string;
};

describe('chainContentPersistence', () => {
  const nodes: Node<TestNodeData>[] = [
    {
      id: 'chain-a',
      position: { x: 100, y: 200 },
      type: 'chainFrame',
      data: {
        kind: 'chain',
        name: 'Arm Yourself',
        summary: 'Onboarding chain',
        scopeType: 'mission',
        scopeId: '622',
        enabled: true,
        priority: 110,
        source: 'content_chains #1',
        semantic: 'story',
        color: '#22c55e',
        spaceId: 'Castle_CellBlock',
        missionId: '622',
      },
    },
    {
      id: 'card-a1',
      parentId: 'chain-a',
      position: { x: 320, y: 110 },
      type: 'missionNode',
      data: {
        kind: 'mission',
        family: 'trigger',
        stage: 'Trigger',
        title: 'Enter Region8',
        detail: 'Player enters the onboarding region.',
        status: 'content_triggers',
        scenario: 'player region entry',
        accent: '#13a2a4',
        manualPosition: true,
        sortOrder: 1,
        inputs: ['In'],
        outputs: ['Out'],
        outputConditions: ['event.enter_region'],
        properties: [
          { label: 'event_type', value: 'enter_region' },
          { label: 'event_key', value: 'Castle_CellBlock.Region8' },
        ],
      },
      extent: 'parent',
    },
    {
      id: 'chain-b',
      position: { x: 100, y: 700 },
      type: 'chainFrame',
      data: {
        kind: 'chain',
        name: 'Other Space',
        summary: 'Should not be saved into Castle_CellBlock content payload',
        scopeType: 'space',
        scopeId: '2',
        enabled: true,
        priority: 10,
        source: 'content_chains #9',
        semantic: 'debug',
        color: '#94a3b8',
        spaceId: 'Castle',
        missionId: null,
      },
    },
  ];

  const edges: Edge<TestEdgeData>[] = [
    {
      id: 'edge-a',
      source: 'card-a1',
      target: 'card-a1',
      type: 'sequenceThread',
      sourceHandle: 'output-0',
      targetHandle: 'input-0',
      data: {
        sequenceId: 'primary-thread',
        label: 'Primary Thread',
        category: 'Story',
        color: '#22c55e',
      },
      style: {
        stroke: '#22c55e',
        strokeWidth: 4,
      },
    },
    {
      id: 'edge-b',
      source: 'chain-b',
      target: 'chain-b',
      type: 'smoothstep',
    },
  ];

  it('extracts only the requested space snapshot for content persistence', () => {
    const snapshot = extractSpaceScopedContentSnapshot(nodes, edges, 'Castle_CellBlock');

    expect(snapshot.nodes.map((node) => node.id)).toEqual(['chain-a', 'card-a1']);
    expect(snapshot.edges.map((edge) => edge.id)).toEqual(['edge-a']);
  });

  it('round-trips flat content graphs back into editor nodes and edges', () => {
    const scopedSnapshot = extractSpaceScopedContentSnapshot(nodes, edges, 'Castle_CellBlock');
    const graph = createPersistedContentGraph({
      edges: scopedSnapshot.edges,
      missionId: null,
      nodes: scopedSnapshot.nodes,
      selectedChainId: 'chain-a',
      selectedNodeId: 'card-a1',
      selectedSequenceId: 'primary-thread',
      spaceId: 'Castle_CellBlock',
    });

    const restoredSnapshot = createEditorSnapshotFromPersistedContentGraph(graph);
    expect(restoredSnapshot.nodes.map((node) => node.id)).toEqual(['chain-a', 'card-a1']);
    expect(restoredSnapshot.edges.map((edge) => edge.id)).toEqual(['edge-a']);
    expect(restoredSnapshot.selectedChainId).toBe('chain-a');
    expect(restoredSnapshot.selectedNodeId).toBe('card-a1');
    expect(restoredSnapshot.selectedSequenceId).toBe('primary-thread');
  });

  it('validates the persisted content graph shape', () => {
    const snapshot = extractSpaceScopedContentSnapshot(nodes, edges, 'Castle_CellBlock');
    const graph = createPersistedContentGraph({
      edges: snapshot.edges,
      missionId: null,
      nodes: snapshot.nodes,
      selectedChainId: 'chain-a',
      selectedNodeId: 'card-a1',
      selectedSequenceId: 'primary-thread',
      spaceId: 'Castle_CellBlock',
    });

    expect(isPersistedContentGraph(graph)).toBe(true);
    expect(
      isPersistedContentGraph({
        version: 1,
        spaceId: 'Castle_CellBlock',
        missionId: null,
        nodes: [],
        edges: [],
      }),
    ).toBe(false);
  });
});
