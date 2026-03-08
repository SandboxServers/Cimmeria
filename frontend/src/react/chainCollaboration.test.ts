import { describe, expect, it, vi } from 'vitest';
import {
  attachChainEditorCollaborationMetadata,
  buildPersistedConflictState,
  readChainEditorCollaborationMetadata,
  resolveChainEditorActorName,
} from './chainCollaboration';

describe('chainCollaboration', () => {
  it('preserves the original author while updating the last editor metadata', () => {
    const initialValue = attachChainEditorCollaborationMetadata(
      { spaceId: 'Castle_CellBlock' },
      'Derek',
      '2026-03-08T01:00:00.000Z',
    );
    const updatedValue = attachChainEditorCollaborationMetadata(
      initialValue,
      'Mission Designer',
      '2026-03-08T02:00:00.000Z',
    );

    expect(readChainEditorCollaborationMetadata(updatedValue)).toEqual({
      author: 'Derek',
      lastEditedAt: '2026-03-08T02:00:00.000Z',
      lastEditedBy: 'Mission Designer',
    });
  });

  it('detects persisted signature drift after load', () => {
    const conflict = buildPersistedConflictState({
      latestMetadata: {
        author: 'Derek',
        lastEditedAt: '2026-03-08T02:30:00.000Z',
        lastEditedBy: 'Mission Designer',
      },
      latestSignature: 'sig-b',
      loadedSignature: 'sig-a',
      timestamp: '2026-03-08T02:31:00.000Z',
    });

    expect(conflict.changedSinceLoad).toBe(true);
    expect(conflict.latestMetadata?.lastEditedBy).toBe('Mission Designer');
  });

  it('falls back to a default actor name when none is configured', () => {
    vi.stubGlobal('window', undefined);

    expect(resolveChainEditorActorName()).toBe('Local operator');

    vi.unstubAllGlobals();
  });
});
