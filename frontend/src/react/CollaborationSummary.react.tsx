/** @jsxImportSource react */

import type { ChainEditorCollaborationMetadata, PersistedConflictState } from './chainCollaboration';

function formatTimestampLabel(timestamp?: string) {
  if (!timestamp) {
    return 'not recorded';
  }

  const parsed = new Date(timestamp);
  if (Number.isNaN(parsed.getTime())) {
    return 'not recorded';
  }

  return parsed.toLocaleString([], {
    dateStyle: 'medium',
    timeStyle: 'short',
  });
}

function formatSavedLabel(
  label: string,
  metadata: ChainEditorCollaborationMetadata | null | undefined,
): string {
  if (!metadata) {
    return `${label}: not recorded`;
  }

  return `${label}: ${metadata.lastEditedBy} on ${formatTimestampLabel(metadata.lastEditedAt)}`;
}

type CollaborationSummaryProps = {
  actorName: string;
  contentConflict: PersistedConflictState | null;
  contentMetadata: ChainEditorCollaborationMetadata | null;
  draftConflict: PersistedConflictState | null;
  draftMetadata: ChainEditorCollaborationMetadata | null;
};

export function CollaborationSummary({
  actorName,
  contentConflict,
  contentMetadata,
  draftConflict,
  draftMetadata,
}: CollaborationSummaryProps) {
  const draftChanged = !!draftConflict?.changedSinceLoad;
  const contentChanged = !!contentConflict?.changedSinceLoad;
  const hasConflict = draftChanged || contentChanged;

  return (
    <div
      className={`rounded-[22px] border px-4 py-3 ${
        hasConflict
          ? 'border-[rgba(248,113,113,0.35)] bg-[rgba(94,24,32,0.4)]'
          : 'border-white/8 bg-[rgba(255,255,255,0.03)]'
      }`}
    >
      <div className="flex flex-wrap items-center gap-2">
        <span className="rounded-full border border-white/8 bg-white/4 px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.2em] text-[rgba(224,231,239,0.78)]">
          Collaboration
        </span>
        <span className="rounded-full border border-[rgba(94,184,179,0.24)] bg-[rgba(94,184,179,0.12)] px-3 py-1 text-xs font-medium text-[#baf4eb]">
          You: {actorName}
        </span>
        {draftChanged ? (
          <span className="rounded-full border border-[rgba(248,113,113,0.35)] bg-[rgba(248,113,113,0.16)] px-3 py-1 text-xs font-medium text-[rgba(254,202,202,0.96)]">
            Draft changed since load
          </span>
        ) : null}
        {contentChanged ? (
          <span className="rounded-full border border-[rgba(245,170,49,0.35)] bg-[rgba(245,170,49,0.16)] px-3 py-1 text-xs font-medium text-[rgba(254,240,138,0.96)]">
            Content changed since load
          </span>
        ) : null}
      </div>

      <div className="mt-3 grid gap-3 md:grid-cols-2">
        <div className="rounded-[18px] border border-white/8 bg-[rgba(7,12,18,0.56)] px-4 py-3">
          <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-[rgba(160,174,192,0.68)]">
            Draft Metadata
          </p>
          <p className="mt-2 text-sm text-white">{formatSavedLabel('Last draft save', draftMetadata)}</p>
          <p className="mt-1 text-xs leading-5 text-[rgba(160,174,192,0.76)]">
            {draftMetadata ? `Author: ${draftMetadata.author}` : 'Author metadata is not recorded yet.'}
          </p>
          {draftChanged && draftConflict?.latestMetadata ? (
            <p className="mt-2 text-xs leading-5 text-[rgba(254,202,202,0.92)]">
              Latest persisted draft was saved by {draftConflict.latestMetadata.lastEditedBy} on{' '}
              {formatTimestampLabel(draftConflict.latestMetadata.lastEditedAt)}.
            </p>
          ) : null}
        </div>

        <div className="rounded-[18px] border border-white/8 bg-[rgba(7,12,18,0.56)] px-4 py-3">
          <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-[rgba(160,174,192,0.68)]">
            Content Metadata
          </p>
          <p className="mt-2 text-sm text-white">{formatSavedLabel('Last content save', contentMetadata)}</p>
          <p className="mt-1 text-xs leading-5 text-[rgba(160,174,192,0.76)]">
            {contentMetadata
              ? `Author: ${contentMetadata.author}`
              : 'Content-engine metadata has not been recorded yet.'}
          </p>
          {contentChanged && contentConflict?.latestMetadata ? (
            <p className="mt-2 text-xs leading-5 text-[rgba(254,240,138,0.92)]">
              Latest content rows were saved by {contentConflict.latestMetadata.lastEditedBy} on{' '}
              {formatTimestampLabel(contentConflict.latestMetadata.lastEditedAt)}.
            </p>
          ) : null}
        </div>
      </div>
    </div>
  );
}
