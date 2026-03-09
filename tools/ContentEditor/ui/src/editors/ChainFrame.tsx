import { memo } from 'react';
import type { Node, NodeProps } from '@xyflow/react';
import { familyLabel, familyTint, laneDescriptors, orderedFamilies } from './constants';
import type { ChainFrameData, EditorNodeData, PrimitiveFamily } from './types';

/**
 * Custom xyflow group node that renders a chain frame -- the large container
 * card that holds mission nodes organized into family lanes.
 *
 * Renders:
 * - Semantic badge (Story, Reward, Puzzle, etc.) with chain color
 * - Scope type / scope ID and priority badges
 * - Chain name and summary text
 * - Enabled/disabled status badge
 * - Source label and per-family node counts
 * - Background lane rows for each active primitive family,
 *   each with a title chip, blurb, and dashed drop zone
 * - Subtle gradient overlays and glow effects using the chain color
 */
export const ChainFrameNode = memo(function ChainFrameNode({
  data,
  selected,
}: NodeProps<Node<EditorNodeData>>) {
  const chainData = data as ChainFrameData;
  const activeFamilies = orderedFamilies.filter(
    (family) => (chainData.counts?.[family] ?? 0) > 0,
  );
  const visibleFamilies: PrimitiveFamily[] = activeFamilies.length
    ? activeFamilies
    : ['trigger', 'condition', 'action'];

  return (
    <div
      className="relative h-full w-full overflow-hidden rounded-[34px] border shadow-[0_30px_90px_rgba(0,0,0,0.34)] backdrop-blur-xl"
      style={{
        borderColor: selected ? `${chainData.color}88` : `${chainData.color}44`,
        background: 'linear-gradient(180deg, rgba(9,18,28,0.94), rgba(7,14,22,0.9))',
        boxShadow: selected
          ? `0 0 0 1px ${chainData.color}44, 0 28px 90px rgba(0,0,0,0.34)`
          : '0 28px 90px rgba(0,0,0,0.34)',
      }}
    >
      {/* Radial gradient overlays */}
      <div
        className="absolute inset-0"
        style={{
          background: `radial-gradient(circle at top left, ${chainData.color}18, transparent 42%), radial-gradient(circle at bottom right, ${chainData.color}10, transparent 35%)`,
        }}
      />

      {/* Header */}
      <div className="relative z-10 flex items-start justify-between gap-4 border-b border-white/6 px-6 py-5">
        <div>
          {/* Badge row */}
          <div className="flex flex-wrap items-center gap-2">
            <span
              className="rounded-full px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.22em]"
              style={{
                backgroundColor: `${chainData.color}22`,
                color: chainData.color,
              }}
            >
              {chainData.semantic}
            </span>
            <span className="rounded-full border border-white/8 bg-white/4 px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.22em] text-[rgba(228,235,240,0.72)]">
              {chainData.scopeType} / {chainData.scopeId}
            </span>
            <span className="rounded-full border border-white/8 bg-white/4 px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.22em] text-[rgba(228,235,240,0.72)]">
              priority {chainData.priority}
            </span>
          </div>

          {/* Chain name + summary */}
          <h3 className="mt-3 text-2xl font-semibold tracking-[-0.05em] text-white">
            {chainData.name}
          </h3>
          <p className="mt-2 max-w-4xl text-sm leading-6 text-[rgba(224,231,239,0.76)]">
            {chainData.summary}
          </p>
        </div>

        {/* Right-side metadata */}
        <div className="grid min-w-[250px] gap-2 text-right text-xs text-[rgba(224,231,239,0.72)]">
          <span
            className="justify-self-end rounded-full px-3 py-1 font-medium"
            style={{
              backgroundColor: chainData.enabled ? `${chainData.color}22` : 'rgba(148,163,184,0.12)',
              color: chainData.enabled ? chainData.color : '#cbd5e1',
            }}
          >
            {chainData.enabled ? 'Enabled' : 'Disabled'}
          </span>
          <span className="uppercase tracking-[0.18em] text-[rgba(160,174,192,0.72)]">
            {chainData.source}
          </span>
          <div className="flex flex-wrap justify-end gap-2">
            {orderedFamilies.map((family) => (
              <span
                className="rounded-full border border-white/8 bg-white/4 px-3 py-1"
                key={family}
              >
                {familyLabel[family]} {chainData.counts?.[family] ?? 0}
              </span>
            ))}
          </div>
        </div>
      </div>

      {/* Lane rows */}
      <div className="pointer-events-none absolute inset-x-5 bottom-5 top-[106px]">
        <div className="flex h-full flex-col gap-[26px]">
          {visibleFamilies.map((family) => {
            const lane = laneDescriptors.find((item) => item.family === family);
            if (!lane) {
              return null;
            }

            return (
              <div
                className="grid min-h-0 flex-1 grid-cols-[176px_minmax(0,1fr)] gap-4"
                key={lane.family}
              >
                {/* Lane label card */}
                <div className="rounded-[28px] border border-white/6 bg-[rgba(255,255,255,0.02)] px-4 py-4">
                  <span
                    className="rounded-full px-3 py-1 text-[10px] font-semibold uppercase tracking-[0.22em]"
                    style={{
                      backgroundColor: familyTint[lane.family].chip,
                      color: familyTint[lane.family].text,
                    }}
                  >
                    {lane.title}
                  </span>
                  <p className="mt-3 text-xs leading-5 text-[rgba(214,223,232,0.66)]">
                    {lane.blurb}
                  </p>
                </div>

                {/* Lane drop zone */}
                <div
                  className="rounded-[28px] border border-dashed border-white/6 bg-[rgba(255,255,255,0.015)]"
                  style={{
                    boxShadow: `inset 0 0 0 1px ${familyTint[lane.family].glow}`,
                  }}
                />
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
});
