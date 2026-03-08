import { describe, expect, it, beforeEach } from 'vitest';
import type { Edge, Node } from '@xyflow/react';

import {
  clearReuseClipboard,
  createCardReuseBundle,
  createChainReuseBundle,
  deleteChainTemplate,
  duplicateCard,
  duplicateChain,
  instantiateReuseBundle,
  loadChainTemplates,
  loadReuseClipboard,
  saveChainTemplate,
  saveReuseClipboard,
} from './chainReuse';

type Family = 'anchor' | 'trigger' | 'condition' | 'action' | 'counter';

type MissionNodeData = {
  kind: 'mission';
  family: Family;
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

type ChainFrameData = {
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
};

type EditorNodeData = MissionNodeData | ChainFrameData;

type SequenceEdgeData = {
  sequenceId: string;
  label: string;
  category: string;
  color: string;
};

function createAllocators() {
  let nodeCounter = 400;
  let chainCounter = 40;
  let sequenceCounter = 10;
  let templateCounter = 1;

  return {
    nextNodeId: () => `n${nodeCounter++}`,
    nextChainId: () => `chain-${chainCounter++}`,
    nextSequenceId: () => `sequence-${sequenceCounter++}`,
    nextTemplateId: () => `template-${templateCounter++}`,
  };
}

function buildFixture() {
  const nodes: Node<EditorNodeData>[] = [
    {
      id: 'chain-alpha',
      position: { x: 120, y: 140 },
      data: {
        kind: 'chain',
        name: 'Alpha',
        summary: 'Chain alpha',
        scopeType: 'mission',
        scopeId: '622',
        enabled: true,
        priority: 110,
        source: 'content_chains',
        semantic: 'Story',
        color: '#22c55e',
        spaceId: 'Castle_CellBlock',
        missionId: '622',
      },
      type: 'chainFrame',
    },
    {
      id: 'node-start',
      parentId: 'chain-alpha',
      position: { x: 260, y: 180 },
      data: {
        kind: 'mission',
        family: 'anchor',
        stage: 'Start',
        title: 'Sequence Start',
        detail: 'Start',
        status: 'Marker',
        scenario: 'Sequence anchor',
        accent: '#ff5e5b',
        inputs: [],
        outputs: ['Out'],
        outputConditions: [''],
        properties: [{ label: 'marker', value: 'START' }],
      },
      type: 'missionNode',
    },
    {
      id: 'node-trigger',
      parentId: 'chain-alpha',
      position: { x: 620, y: 280 },
      data: {
        kind: 'mission',
        family: 'trigger',
        stage: 'Trigger',
        title: 'Enter Region',
        detail: 'Trigger',
        status: 'Event',
        scenario: 'space event',
        accent: '#06b6d4',
        inputs: ['In'],
        outputs: ['Out'],
        outputConditions: [''],
        properties: [{ label: 'event_type', value: 'enter_region' }],
      },
      type: 'missionNode',
    },
  ];

  const edges: Edge<SequenceEdgeData>[] = [
    {
      id: 'edge-1',
      source: 'node-start',
      target: 'node-trigger',
      sourceHandle: 'output-0',
      targetHandle: 'input-0',
      type: 'sequenceThread',
      data: {
        sequenceId: 'story-thread-01',
        label: 'Primary Thread',
        category: 'Story',
        color: '#22c55e',
      },
    },
  ];

  return { nodes, edges };
}

describe('chain reuse helpers', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('duplicates a selected card with a fresh id and a position offset', () => {
    const { nodes } = buildFixture();
    const result = duplicateCard<EditorNodeData, SequenceEdgeData>(
      nodes,
      'node-trigger',
      'chain-alpha',
      createAllocators(),
    );

    expect(result.nodes).toHaveLength(1);
    expect(result.edges).toHaveLength(0);
    expect(result.nodes[0].id).toBe('n400');
    expect(result.nodes[0].parentId).toBe('chain-alpha');
    expect(result.nodes[0].position).toEqual({ x: 684, y: 328 });
    expect(result.selectedNodeId).toBe('n400');
  });

  it('duplicates a chain and remaps child ids, parent ids, and sequence ids', () => {
    const { nodes, edges } = buildFixture();
    const result = duplicateChain<EditorNodeData, SequenceEdgeData>(
      nodes,
      edges,
      'chain-alpha',
      createAllocators(),
    );

    expect(result.selectedChainId).toBe('chain-40');
    expect(result.nodes.map((node) => node.id)).toEqual(['chain-40', 'n400', 'n401']);
    expect(result.nodes[1].parentId).toBe('chain-40');
    expect(result.nodes[2].parentId).toBe('chain-40');
    expect(result.edges).toHaveLength(1);
    expect(result.edges[0].source).toBe('n400');
    expect(result.edges[0].target).toBe('n401');
    expect(result.edges[0].data?.sequenceId).toBe('sequence-10');
    expect(result.selectedSequenceId).toBe('sequence-10');
  });

  it('requires a target chain when pasting a copied card', () => {
    const { nodes } = buildFixture();
    const bundle = createCardReuseBundle<EditorNodeData, SequenceEdgeData>(nodes, 'node-trigger');
    expect(bundle).not.toBeNull();

    expect(() =>
      instantiateReuseBundle(bundle!, createAllocators()),
    ).toThrowError('select a target chain before pasting a copied card');
  });

  it('persists clipboard payloads in browser storage', () => {
    const { nodes, edges } = buildFixture();
    const bundle = createChainReuseBundle<EditorNodeData, SequenceEdgeData>(
      nodes,
      edges,
      'chain-alpha',
    );
    expect(bundle).not.toBeNull();

    saveReuseClipboard(bundle!);
    const loaded = loadReuseClipboard<EditorNodeData, SequenceEdgeData>();
    expect(loaded?.kind).toBe('chain');
    expect(loaded?.nodes).toHaveLength(3);

    clearReuseClipboard();
    expect(loadReuseClipboard<EditorNodeData, SequenceEdgeData>()).toBeNull();
  });

  it('stores and deletes local chain templates', () => {
    const { nodes, edges } = buildFixture();
    const bundle = createChainReuseBundle<EditorNodeData, SequenceEdgeData>(
      nodes,
      edges,
      'chain-alpha',
    );
    expect(bundle).not.toBeNull();

    const template = saveChainTemplate('Alpha Template', bundle!, createAllocators());
    expect(template.id).toBe('template-1');
    expect(loadChainTemplates<EditorNodeData, SequenceEdgeData>()).toHaveLength(1);

    deleteChainTemplate(template.id);
    expect(loadChainTemplates<EditorNodeData, SequenceEdgeData>()).toHaveLength(0);
  });
});
