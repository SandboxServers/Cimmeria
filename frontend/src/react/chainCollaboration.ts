/** @jsxImportSource react */

export type ChainEditorCollaborationMetadata = {
  author: string;
  lastEditedAt: string;
  lastEditedBy: string;
};

export type PersistedConflictState = {
  changedSinceLoad: boolean;
  checkedAt: string;
  latestMetadata: ChainEditorCollaborationMetadata | null;
  latestSignature: string | null;
};

const chainEditorActorStorageKey = 'cimmeria.chain-editor.actor-name';
const defaultChainEditorActor = 'Local operator';

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

export function resolveChainEditorActorName(): string {
  const configuredActor =
    typeof import.meta !== 'undefined' &&
    typeof import.meta.env?.VITE_CHAIN_EDITOR_ACTOR_NAME === 'string'
      ? import.meta.env.VITE_CHAIN_EDITOR_ACTOR_NAME.trim()
      : '';
  if (configuredActor) {
    return configuredActor;
  }

  if (typeof window === 'undefined') {
    return defaultChainEditorActor;
  }

  const storedActor = window.localStorage.getItem(chainEditorActorStorageKey)?.trim();
  return storedActor || defaultChainEditorActor;
}

export function setChainEditorActorName(actorName: string): void {
  if (typeof window === 'undefined') {
    return;
  }

  const normalizedActor = actorName.trim();
  if (!normalizedActor) {
    window.localStorage.removeItem(chainEditorActorStorageKey);
    return;
  }

  window.localStorage.setItem(chainEditorActorStorageKey, normalizedActor);
}

export function isChainEditorCollaborationMetadata(
  value: unknown,
): value is ChainEditorCollaborationMetadata {
  return (
    isRecord(value) &&
    typeof value.author === 'string' &&
    typeof value.lastEditedAt === 'string' &&
    typeof value.lastEditedBy === 'string'
  );
}

export function readChainEditorCollaborationMetadata(
  value: unknown,
): ChainEditorCollaborationMetadata | null {
  if (!isRecord(value) || !isChainEditorCollaborationMetadata(value.collaboration)) {
    return null;
  }

  return value.collaboration;
}

export function attachChainEditorCollaborationMetadata<T extends object>(
  value: T,
  actorName: string,
  timestamp: string,
): T & { collaboration: ChainEditorCollaborationMetadata } {
  const existingMetadata = readChainEditorCollaborationMetadata(value);

  return {
    ...value,
    collaboration: {
      author: existingMetadata?.author || actorName,
      lastEditedAt: timestamp,
      lastEditedBy: actorName,
    },
  };
}

export function buildPersistedConflictState({
  latestMetadata,
  latestSignature,
  loadedSignature,
  timestamp,
}: {
  latestMetadata: ChainEditorCollaborationMetadata | null;
  latestSignature: string | null;
  loadedSignature: string;
  timestamp: string;
}): PersistedConflictState {
  return {
    changedSinceLoad:
      !!latestSignature && latestSignature.length > 0 && latestSignature !== loadedSignature,
    checkedAt: timestamp,
    latestMetadata,
    latestSignature,
  };
}
