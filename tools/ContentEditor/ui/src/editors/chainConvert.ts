import type { Edge, Node } from '@xyflow/react';
import type { ChainData } from '../components/AppLayout';
import type {
  ChainFrameData,
  EditorNodeData,
  MissionNodeData,
  PrimitiveFamily,
  SequenceEdgeData,
} from './types';
import { AUTO_LAYOUT, orderedFamilies } from './constants';

/* ------------------------------------------------------------------ */
/*  ChainData[] (DB rows)  →  xyflow nodes + edges                   */
/* ------------------------------------------------------------------ */

const DB_EXCLUDE_KEYS = new Set([
  'chain_id',
  'trigger_id',
  'condition_id',
  'action_id',
  'counter_id',
  'sort_order',
]);

function semanticFromScope(scopeType: string): string {
  switch (scopeType) {
    case 'mission':
      return 'Story';
    case 'space':
      return 'Space';
    case 'effect':
      return 'Effect';
    default:
      return 'General';
  }
}

function colorFromScope(scopeType: string): string {
  switch (scopeType) {
    case 'mission':
      return '#22c55e';
    case 'space':
      return '#3b82f6';
    case 'effect':
      return '#8b5cf6';
    default:
      return '#94a3b8';
  }
}

function rowToProperties(row: Record<string, unknown>): Array<{ label: string; value: string }> {
  return Object.entries(row)
    .filter(([key]) => !DB_EXCLUDE_KEYS.has(key))
    .filter(([, value]) => value !== null && value !== undefined)
    .map(([label, value]) => ({ label, value: String(value) }));
}

function triggerTitle(t: Record<string, unknown>): string {
  const eventType = String(t.event_type ?? 'trigger');
  const eventKey = String(t.event_key ?? '');
  return eventKey ? `${eventType} — ${eventKey}` : eventType;
}

function conditionTitle(c: Record<string, unknown>): string {
  const condType = String(c.condition_type ?? 'condition');
  const key = c.target_key ?? c.target_id ?? '';
  return key ? `${condType} — ${key}` : condType;
}

function actionTitle(a: Record<string, unknown>): string {
  const actionType = String(a.action_type ?? 'action');
  const target = a.target_id ? ` → ${a.target_id}` : '';
  return `${actionType}${target}`;
}

type SavedLayout = Record<string, { x: number; y: number }>;
type SavedEdge = {
  id: string;
  source: string;
  target: string;
  sourceHandle?: string;
  targetHandle?: string;
  data?: SequenceEdgeData;
};

/**
 * Convert an array of ChainData (from the DB) into xyflow nodes and edges
 * suitable for the chain editor canvas.
 */
export function chainsToFlow(
  chains: ChainData[],
  spaceId: string,
): { nodes: Node<EditorNodeData>[]; edges: Edge<SequenceEdgeData>[] } {
  const nodes: Node<EditorNodeData>[] = [];
  const edges: Edge<SequenceEdgeData>[] = [];
  let frameY = 40;

  for (const chain of chains) {
    const frameId = `chain-${chain.chain_id}`;
    const savedLayout = (chain.editor_data?.layout as SavedLayout) ?? {};
    const savedEdges = (chain.editor_data?.edges as SavedEdge[]) ?? [];

    const counts: Record<PrimitiveFamily, number> = {
      anchor: 2,
      trigger: chain.triggers.length,
      condition: chain.conditions.length,
      action: chain.actions.length,
      counter: chain.counters.length,
    };

    const children: Node<EditorNodeData>[] = [];

    // Start anchor
    children.push({
      id: `${frameId}-anchor-start`,
      type: 'missionNode',
      parentId: frameId,
      extent: 'parent' as const,
      position: savedLayout[`${frameId}-anchor-start`] ?? { x: 0, y: 0 },
      data: {
        kind: 'mission',
        family: 'anchor',
        stage: 'Start',
        title: 'Sequence Start',
        detail: 'Chain entry point',
        status: 'Marker',
        scenario: 'Sequence anchor',
        accent: '#ff5e5b',
        properties: [
          { label: 'marker', value: 'START' },
          { label: 'behavior', value: 'Visual grouping' },
        ],
      },
    });

    // End anchor
    children.push({
      id: `${frameId}-anchor-end`,
      type: 'missionNode',
      parentId: frameId,
      extent: 'parent' as const,
      position: savedLayout[`${frameId}-anchor-end`] ?? { x: 0, y: 0 },
      data: {
        kind: 'mission',
        family: 'anchor',
        stage: 'End',
        title: 'Sequence End',
        detail: 'Chain exit point',
        status: 'Marker',
        scenario: 'Sequence anchor',
        accent: '#ff5e5b',
        properties: [
          { label: 'marker', value: 'END' },
          { label: 'behavior', value: 'Visual grouping' },
        ],
      },
    });

    // Triggers
    chain.triggers.forEach((t: any, i: number) => {
      const nodeId = `${frameId}-trigger-${i}`;
      children.push({
        id: nodeId,
        type: 'missionNode',
        parentId: frameId,
        extent: 'parent' as const,
        position: savedLayout[nodeId] ?? { x: 0, y: 0 },
        data: {
          kind: 'mission',
          family: 'trigger',
          stage: 'Trigger',
          title: triggerTitle(t),
          detail: t.once ? 'Fires once then disarms' : 'Repeatable trigger',
          status: String(t.event_type ?? 'trigger'),
          scenario: String(t.scope ?? 'player'),
          accent: '#13a2a4',
          properties: rowToProperties(t),
        },
      });
    });

    // Conditions
    chain.conditions.forEach((c: any, i: number) => {
      const nodeId = `${frameId}-condition-${i}`;
      children.push({
        id: nodeId,
        type: 'missionNode',
        parentId: frameId,
        extent: 'parent' as const,
        position: savedLayout[nodeId] ?? { x: 0, y: 0 },
        data: {
          kind: 'mission',
          family: 'condition',
          stage: 'Condition',
          title: conditionTitle(c),
          detail: `${c.operator ?? ''} ${c.value ?? ''}`.trim(),
          status: String(c.condition_type ?? 'condition'),
          scenario: 'gate',
          accent: '#f5aa31',
          properties: rowToProperties(c),
        },
      });
    });

    // Actions
    chain.actions.forEach((a: any, i: number) => {
      const nodeId = `${frameId}-action-${i}`;
      children.push({
        id: nodeId,
        type: 'missionNode',
        parentId: frameId,
        extent: 'parent' as const,
        position: savedLayout[nodeId] ?? { x: 0, y: 0 },
        data: {
          kind: 'mission',
          family: 'action',
          stage: 'Action',
          title: actionTitle(a),
          detail: a.target_key ? `Step: ${a.target_key}` : '',
          status: String(a.action_type ?? 'action'),
          scenario: 'mission action',
          accent: '#22c55e',
          properties: rowToProperties(a),
        },
      });
    });

    // Counters
    chain.counters.forEach((c: any, i: number) => {
      const nodeId = `${frameId}-counter-${i}`;
      children.push({
        id: nodeId,
        type: 'missionNode',
        parentId: frameId,
        extent: 'parent' as const,
        position: savedLayout[nodeId] ?? { x: 0, y: 0 },
        data: {
          kind: 'mission',
          family: 'counter',
          stage: 'Counter',
          title: String(c.counter_name ?? 'counter'),
          detail: `target: ${c.target_value ?? '?'}, reset: ${c.reset_on ?? 'never'}`,
          status: 'counter',
          scenario: 'runtime counter',
          accent: '#94a3b8',
          properties: rowToProperties(c),
        },
      });
    });

    // Compute frame dimensions from children
    const activeFamilies = orderedFamilies.filter(
      (f) => children.some((n) => n.data.kind === 'mission' && (n.data as MissionNodeData).family === f),
    );
    const maxPerFamily = Math.max(
      ...activeFamilies.map(
        (f) => children.filter((n) => n.data.kind === 'mission' && (n.data as MissionNodeData).family === f).length,
      ),
      1,
    );
    const frameWidth = Math.max(
      AUTO_LAYOUT.frameLeftPadding +
        AUTO_LAYOUT.rowLabelWidth +
        maxPerFamily * (AUTO_LAYOUT.cardWidth + AUTO_LAYOUT.cardColumnGap) +
        AUTO_LAYOUT.frameRightPadding,
      900,
    );
    const frameHeight = Math.max(
      AUTO_LAYOUT.frameTopPadding +
        activeFamilies.length * (AUTO_LAYOUT.cardHeight + AUTO_LAYOUT.rowGap) +
        AUTO_LAYOUT.frameBottomPadding,
      400,
    );

    // Auto-layout children that have no saved position
    const hasSavedPositions = Object.keys(savedLayout).length > 0;
    if (!hasSavedPositions) {
      const familyCounters: Record<string, number> = {};
      for (const child of children) {
        if (child.data.kind !== 'mission') continue;
        const family = (child.data as MissionNodeData).family;
        const laneIndex = activeFamilies.indexOf(family);
        if (laneIndex < 0) continue;
        const col = familyCounters[family] ?? 0;
        familyCounters[family] = col + 1;
        child.position = {
          x: AUTO_LAYOUT.frameLeftPadding + AUTO_LAYOUT.rowLabelWidth + col * (AUTO_LAYOUT.cardWidth + AUTO_LAYOUT.cardColumnGap),
          y: AUTO_LAYOUT.frameTopPadding + laneIndex * (AUTO_LAYOUT.cardHeight + AUTO_LAYOUT.rowGap),
        };
      }
    }

    // Chain frame group node
    nodes.push({
      id: frameId,
      type: 'chainFrame',
      position: { x: 40, y: frameY },
      data: {
        kind: 'chain',
        name: chain.name ?? `Chain #${chain.chain_id}`,
        summary: chain.description ?? '',
        scopeType: (chain.scope_type as ChainFrameData['scopeType']) || 'space',
        scopeId: String(chain.scope_id ?? ''),
        enabled: chain.enabled,
        priority: chain.priority,
        source: 'database',
        semantic: semanticFromScope(chain.scope_type),
        color: colorFromScope(chain.scope_type),
        spaceId,
        missionId: chain.scope_type === 'mission' ? String(chain.scope_id ?? '') : null,
        counts,
      },
      style: { width: frameWidth, height: frameHeight },
    });

    nodes.push(...children);

    // Restore saved edges
    for (const se of savedEdges) {
      edges.push({
        id: se.id,
        source: se.source,
        target: se.target,
        sourceHandle: se.sourceHandle,
        targetHandle: se.targetHandle,
        type: 'sequenceThread',
        data: se.data ?? {
          sequenceId: '',
          label: 'Sequence yarn',
          category: 'story',
          color: '#22c55e',
        },
      });
    }

    frameY += frameHeight + AUTO_LAYOUT.frameVerticalGap;
  }

  return { nodes, edges };
}

/* ------------------------------------------------------------------ */
/*  xyflow nodes + edges  →  ChainData[] (for saving back to DB)      */
/* ------------------------------------------------------------------ */

function propertyValue(properties: Array<{ label: string; value: string }>, key: string): string | undefined {
  return properties.find((p) => p.label === key)?.value;
}

function propertyMap(properties: Array<{ label: string; value: string }>): Record<string, string> {
  return Object.fromEntries(properties.map((p) => [p.label, p.value]));
}

/**
 * Convert xyflow state back to ChainData[] for persistence.
 * Merges with the original chains to preserve chain_id and fields
 * that aren't represented in the visual editor.
 */
export function flowToChains(
  nodes: Node<EditorNodeData>[],
  edges: Edge<SequenceEdgeData>[],
  originalChains: ChainData[],
): ChainData[] {
  const origById = new Map(originalChains.map((c) => [c.chain_id, c]));

  // Group nodes by parent frame
  const frameNodes = nodes.filter((n) => n.type === 'chainFrame');

  return frameNodes.map((frame) => {
    const chainId = Number(frame.id.replace('chain-', ''));
    const orig = origById.get(chainId);
    const frameData = frame.data as ChainFrameData;

    const children = nodes.filter(
      (n) => n.parentId === frame.id && n.data.kind === 'mission',
    );

    // Build saved layout positions
    const layout: Record<string, { x: number; y: number }> = {};
    for (const child of children) {
      layout[child.id] = child.position;
    }

    // Build saved edges for this chain
    const childIds = new Set(children.map((n) => n.id));
    const chainEdges = edges
      .filter((e) => childIds.has(e.source) || childIds.has(e.target))
      .map((e) => ({
        id: e.id,
        source: e.source,
        target: e.target,
        sourceHandle: e.sourceHandle ?? undefined,
        targetHandle: e.targetHandle ?? undefined,
        data: e.data,
      }));

    // Extract primitives by family
    const triggers = children
      .filter((n) => (n.data as MissionNodeData).family === 'trigger')
      .map((n) => propertyMap((n.data as MissionNodeData).properties));

    const conditions = children
      .filter((n) => (n.data as MissionNodeData).family === 'condition')
      .map((n) => propertyMap((n.data as MissionNodeData).properties));

    const actions = children
      .filter((n) => (n.data as MissionNodeData).family === 'action')
      .map((n) => propertyMap((n.data as MissionNodeData).properties));

    const counters = children
      .filter((n) => (n.data as MissionNodeData).family === 'counter')
      .map((n) => propertyMap((n.data as MissionNodeData).properties));

    return {
      chain_id: chainId,
      name: frameData.name,
      description: frameData.summary,
      scope_type: frameData.scopeType,
      scope_id: orig?.scope_id ?? null,
      enabled: frameData.enabled,
      priority: frameData.priority,
      editor_data: { layout, edges: chainEdges },
      triggers,
      conditions,
      actions,
      counters,
    } satisfies ChainData;
  });
}

/* ------------------------------------------------------------------ */
/*  Node ID generation                                                */
/* ------------------------------------------------------------------ */

let nextNodeSeq = 0;

export function generateNodeId(frameId: string, family: PrimitiveFamily): string {
  nextNodeSeq += 1;
  return `${frameId}-${family}-new-${nextNodeSeq}`;
}
