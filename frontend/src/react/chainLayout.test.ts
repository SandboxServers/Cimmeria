import { describe, expect, it } from 'vitest';
import type { Node } from '@xyflow/react';

import { computePackedChainLayouts, resolveAutoLayoutNodePositions } from './chainLayout';
import { validateConnectionRequest } from './chainConnections';

type TestNode = Node<{ kind: string }>;

const layoutOptions = {
  cardHeight: 252,
  cardWidth: 286,
  frameBottomPadding: 36,
  frameRightPadding: 72,
  minFrameHeight: 350,
  minFrameWidth: 1280,
  topPadding: 20,
  verticalGap: 80,
};

describe('computePackedChainLayouts', () => {
  it('packs chain frames vertically when larger child bounds would overlap', () => {
    const nodes: TestNode[] = [
      {
        data: { kind: 'chain' },
        id: 'chain-a',
        position: { x: -340, y: 20 },
        style: { height: 390, width: 1710 },
      },
      {
        data: { kind: 'chain' },
        id: 'chain-b',
        position: { x: -120, y: 470 },
        style: { height: 410, width: 1920 },
      },
      {
        data: { kind: 'mission' },
        id: 'a-1',
        parentId: 'chain-a',
        position: { x: 70, y: 165 },
      },
      {
        data: { kind: 'mission' },
        id: 'a-2',
        parentId: 'chain-a',
        position: { x: 1500, y: 165 },
      },
      {
        data: { kind: 'mission' },
        id: 'b-1',
        parentId: 'chain-b',
        position: { x: 70, y: 180 },
      },
      {
        data: { kind: 'mission' },
        id: 'b-2',
        parentId: 'chain-b',
        position: { x: 1680, y: 250 },
      },
    ];

    const layouts = computePackedChainLayouts(
      [{ nodeId: 'chain-a' }, { nodeId: 'chain-b' }],
      nodes,
      new Map(nodes.map((node) => [node.id, node])),
      layoutOptions,
    );

    const chainA = layouts.get('chain-a');
    const chainB = layouts.get('chain-b');

    expect(chainA).toBeDefined();
    expect(chainB).toBeDefined();
    expect(chainA?.framePosition.x).toBe(-340);
    expect(chainB?.framePosition.x).toBe(-120);
    expect(chainA?.frameStyle.height).toBeGreaterThanOrEqual(453);
    expect(chainB?.framePosition.y).toBeGreaterThanOrEqual(
      (chainA?.framePosition.y ?? 0) + (chainA?.frameStyle.height ?? 0) + layoutOptions.verticalGap,
    );
  });

  it('grows a frame when a child card is moved beyond the authored width', () => {
    const nodes: TestNode[] = [
      {
        data: { kind: 'chain' },
        id: 'chain-a',
        position: { x: 24, y: 20 },
        style: { height: 390, width: 1280 },
      },
      {
        data: { kind: 'mission' },
        id: 'a-1',
        parentId: 'chain-a',
        position: { x: 70, y: 120 },
      },
      {
        data: { kind: 'mission' },
        id: 'a-2',
        parentId: 'chain-a',
        position: { x: 1800, y: 120 },
      },
    ];

    const layouts = computePackedChainLayouts(
      [{ nodeId: 'chain-a' }],
      nodes,
      new Map(nodes.map((node) => [node.id, node])),
      layoutOptions,
    );

    expect(layouts.get('chain-a')?.frameStyle.width).toBeGreaterThan(1280);
  });
});

describe('resolveAutoLayoutNodePositions', () => {
  it('auto-sorts untouched cards into their lane positions', () => {
    const positions = resolveAutoLayoutNodePositions(
      [{ nodeId: 'chain-a' }],
      [
        {
          data: { kind: 'chain' },
          id: 'chain-a',
          position: { x: 24, y: 20 },
        },
        {
          data: { family: 'trigger', kind: 'mission', sortOrder: 0 },
          id: 'trigger-a',
          parentId: 'chain-a',
          position: { x: 999, y: 999 },
        },
        {
          data: { family: 'condition', kind: 'mission', sortOrder: 1 },
          id: 'condition-a',
          parentId: 'chain-a',
          position: { x: 999, y: 999 },
        },
      ],
      [
        {
          source: 'trigger-a',
          target: 'condition-a',
          type: 'sequenceThread',
        },
      ],
      {
        cardColumnGap: 420,
        cardHeight: 252,
        cardWidth: 286,
        frameLeftPadding: 56,
        frameTopPadding: 118,
        orderedFamilies: ['anchor', 'trigger', 'condition', 'action', 'counter'],
        rowGap: 72,
        rowLabelWidth: 176,
      },
    );

    expect(positions.get('trigger-a')).toEqual({ x: 232, y: 118 });
    expect(positions.get('condition-a')).toEqual({ x: 938, y: 442 });
  });

  it('preserves manual card positions while leaving other cards auto-laid', () => {
    const positions = resolveAutoLayoutNodePositions(
      [{ nodeId: 'chain-a' }],
      [
        {
          data: { kind: 'chain' },
          id: 'chain-a',
          position: { x: 24, y: 20 },
        },
        {
          data: { family: 'trigger', kind: 'mission', manualPosition: true, sortOrder: 0 },
          id: 'trigger-a',
          parentId: 'chain-a',
          position: { x: 1337, y: 444 },
        },
        {
          data: { family: 'condition', kind: 'mission', sortOrder: 1 },
          id: 'condition-a',
          parentId: 'chain-a',
          position: { x: 999, y: 999 },
        },
      ],
      [
        {
          source: 'trigger-a',
          target: 'condition-a',
          type: 'sequenceThread',
        },
      ],
      {
        cardColumnGap: 420,
        cardHeight: 252,
        cardWidth: 286,
        frameLeftPadding: 56,
        frameTopPadding: 118,
        orderedFamilies: ['anchor', 'trigger', 'condition', 'action', 'counter'],
        rowGap: 72,
        rowLabelWidth: 176,
      },
    );

    expect(positions.get('trigger-a')).toEqual({ x: 1337, y: 444 });
    expect(positions.get('condition-a')).toEqual({ x: 938, y: 442 });
  });
});

describe('validateConnectionRequest', () => {
  it('allows a newly added mission card to connect into a fresh sequence yarn with default ports', () => {
    expect(() =>
      validateConnectionRequest({
        sourceHandle: 'output-0',
        sourceId: 'new-card',
        sourceNode: {
          data: {
            family: 'trigger',
            stage: 'Trigger',
            title: 'New Trigger Card',
          },
        },
        targetHandle: 'input-0',
        targetId: 'sequence-end',
        targetNode: {
          data: {
            family: 'anchor',
            stage: 'End',
            title: 'Sequence End',
            inputs: ['In'],
          },
        },
      }),
    ).not.toThrow();
  });

  it('surfaces an exact error when a new card tries to connect to an invalid handle', () => {
    expect(() =>
      validateConnectionRequest({
        sourceHandle: 'output-1',
        sourceId: 'new-card',
        sourceNode: {
          data: {
            family: 'trigger',
            stage: 'Trigger',
            title: 'New Trigger Card',
          },
        },
        targetHandle: 'input-0',
        targetId: 'sequence-end',
        targetNode: {
          data: {
            family: 'anchor',
            stage: 'End',
            title: 'Sequence End',
            inputs: ['In'],
          },
        },
      }),
    ).toThrowError('invalid output on New Trigger Card');
  });
});
