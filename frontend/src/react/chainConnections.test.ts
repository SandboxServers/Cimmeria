import { describe, expect, it } from 'vitest';

import {
  getConnectionPortLabels,
  getNodeHandleSignature,
  validateConnectionRequest,
} from './chainConnections';

describe('getConnectionPortLabels', () => {
  it('provides default in/out ports for standard mission cards', () => {
    expect(
      getConnectionPortLabels({
        data: {
          family: 'trigger',
          stage: 'Trigger',
          title: 'Enter Region',
        },
      }),
    ).toEqual({
      inputs: ['In'],
      outputs: ['Out'],
    });
  });

  it('removes the default output for sequence end anchors', () => {
    expect(
      getConnectionPortLabels({
        data: {
          family: 'anchor',
          stage: 'End',
          title: 'Sequence End',
        },
      }),
    ).toEqual({
      inputs: ['In'],
      outputs: [],
    });
  });
});

describe('getNodeHandleSignature', () => {
  it('changes when dynamic ports are added so node internals can be refreshed', () => {
    const originalSignature = getNodeHandleSignature({
      data: {
        family: 'trigger',
        stage: 'Trigger',
        title: 'Dialog Choice 5020 / 2299',
        inputs: ['Jaffa branch', 'Human branch'],
        outputs: ['Choice 5020', 'Choice 2299'],
      },
    });

    const nextSignature = getNodeHandleSignature({
      data: {
        family: 'trigger',
        stage: 'Trigger',
        title: 'Dialog Choice 5020 / 2299',
        inputs: ['Jaffa branch', 'Human branch'],
        outputs: ['Choice 5020', 'Choice 2299', 'Choice 9999'],
      },
    });

    expect(nextSignature).not.toBe(originalSignature);
  });
});

describe('validateConnectionRequest', () => {
  it('rejects self-links with an exact error', () => {
    expect(() =>
      validateConnectionRequest({
        sourceHandle: 'output-0',
        sourceId: 'same-node',
        sourceNode: {
          data: {
            family: 'trigger',
            stage: 'Trigger',
            title: 'Trigger A',
          },
        },
        targetHandle: 'input-0',
        targetId: 'same-node',
        targetNode: {
          data: {
            family: 'condition',
            stage: 'Condition',
            title: 'Condition A',
          },
        },
      }),
    ).toThrowError('a card cannot connect to itself');
  });

  it('rejects missing endpoint nodes with an exact error', () => {
    expect(() =>
      validateConnectionRequest({
        sourceHandle: 'output-0',
        sourceId: 'source',
        targetHandle: 'input-0',
        targetId: 'target',
      }),
    ).toThrowError('both endpoints must be mission cards');
  });
});
