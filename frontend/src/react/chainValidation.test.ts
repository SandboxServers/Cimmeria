import { describe, expect, it } from 'vitest';
import { buildValidationReport, type ValidationChainSummary, type ValidationNodeData } from './chainValidation';
import type { Edge, Node } from '@xyflow/react';

function missionNode(
  id: string,
  parentId: string,
  patch: Partial<Extract<ValidationNodeData, { kind: 'mission' }>> = {},
): Node<ValidationNodeData> {
  return {
    id,
    parentId,
    position: { x: 0, y: 0 },
    data: {
      kind: 'mission',
      family: 'trigger',
      stage: 'Trigger',
      title: 'Trigger Card',
      detail: 'Fires when the player enters.',
      scenario: 'Region entry',
      inputs: ['In'],
      outputs: ['Out'],
      outputConditions: [''],
      ...patch,
    },
  } as Node<ValidationNodeData>;
}

function chainSummary(
  id: string,
  patch: Partial<ValidationChainSummary> = {},
): ValidationChainSummary {
  return {
    kind: 'chain',
    name: 'Arm Yourself! Onboarding',
    summary: 'Initial onboarding content chain.',
    scopeType: 'mission',
    scopeId: '622',
    enabled: true,
    priority: 110,
    source: 'content_chains',
    semantic: 'Story',
    color: '#22c55e',
    spaceId: 'Castle_CellBlock',
    missionId: '622',
    nodeId: id,
    childIds: [],
    edgeCount: 0,
    sequenceCount: 0,
    ...patch,
  };
}

describe('buildValidationReport', () => {
  it('reports missing chain anchors and blank node metadata', () => {
    const chain = chainSummary('chain-1', { summary: '' });
    const nodes: Node<ValidationNodeData>[] = [
      missionNode('n1', 'chain-1', {
        title: '',
        detail: '',
        scenario: '',
      }),
    ];

    const report = buildValidationReport({
      chains: [{ ...chain, childIds: ['n1'] }],
      nodes,
      edges: [],
    });

    expect(report.errorCount).toBeGreaterThanOrEqual(2);
    expect(report.warningCount).toBeGreaterThanOrEqual(4);
    expect(report.issues.some((issue) => issue.id === 'chain-start-chain-1')).toBe(true);
    expect(report.issues.some((issue) => issue.id === 'chain-end-chain-1')).toBe(true);
    expect(report.issues.some((issue) => issue.id === 'node-title-n1')).toBe(true);
    expect(report.issues.some((issue) => issue.id === 'node-detail-n1')).toBe(true);
  });

  it('flags branching cards whose output conditions drift from outputs', () => {
    const chain = chainSummary('chain-1');
    const nodes: Node<ValidationNodeData>[] = [
      missionNode('start', 'chain-1', {
        family: 'anchor',
        stage: 'Start',
        inputs: [],
        outputs: ['Primary', 'Fallback'],
        outputConditions: ['dialog.choice == 5020'],
        title: 'Sequence Start',
      }),
      missionNode('end', 'chain-1', {
        family: 'anchor',
        stage: 'End',
        inputs: ['In'],
        outputs: [],
        outputConditions: [],
        title: 'Sequence End',
      }),
    ];
    const edges: Edge[] = [
      {
        id: 'e1',
        source: 'start',
        target: 'end',
        sourceHandle: 'output-0',
        targetHandle: 'input-0',
        type: 'sequenceThread',
        data: {
          sequenceId: 'seq-1',
          label: 'Primary Thread',
          category: 'Story',
          color: '#22c55e',
        },
      },
    ];

    const report = buildValidationReport({
      chains: [{ ...chain, childIds: ['start', 'end'] }],
      nodes,
      edges,
    });

    expect(report.issues.some((issue) => issue.id === 'node-branch-count-start')).toBe(true);
  });

  it('accepts a minimally healthy linear chain', () => {
    const chain = chainSummary('chain-1');
    const nodes: Node<ValidationNodeData>[] = [
      missionNode('start', 'chain-1', {
        family: 'anchor',
        stage: 'Start',
        inputs: [],
        outputs: ['Out'],
        outputConditions: [''],
        title: 'Sequence Start',
      }),
      missionNode('trigger', 'chain-1'),
      missionNode('end', 'chain-1', {
        family: 'anchor',
        stage: 'End',
        inputs: ['In'],
        outputs: [],
        outputConditions: [],
        title: 'Sequence End',
      }),
    ];
    const edges: Edge[] = [
      {
        id: 'e1',
        source: 'start',
        target: 'trigger',
        sourceHandle: 'output-0',
        targetHandle: 'input-0',
        type: 'sequenceThread',
        data: {
          sequenceId: 'seq-1',
          label: 'Primary Thread',
          category: 'Story',
          color: '#22c55e',
        },
      },
      {
        id: 'e2',
        source: 'trigger',
        target: 'end',
        sourceHandle: 'output-0',
        targetHandle: 'input-0',
        type: 'sequenceThread',
        data: {
          sequenceId: 'seq-1',
          label: 'Primary Thread',
          category: 'Story',
          color: '#22c55e',
        },
      },
    ];

    const report = buildValidationReport({
      chains: [{ ...chain, childIds: ['start', 'trigger', 'end'] }],
      nodes,
      edges,
    });

    expect(report.errorCount).toBe(0);
    expect(report.warningCount).toBe(0);
  });
});
