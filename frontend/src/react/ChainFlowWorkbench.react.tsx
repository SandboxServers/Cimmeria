/** @jsxImportSource react */
import {
  memo,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type DragEvent as ReactDragEvent,
  type ReactNode,
} from 'react';
import MissionCardLibrary from './MissionCardLibrary';
import type { ScenarioTemplate as LibraryScenarioTemplate } from './missionCardCatalog';
import {
  Background,
  BackgroundVariant,
  BaseEdge,
  Controls,
  type Edge,
  type EdgeProps,
  Handle,
  type Node,
  type NodeProps,
  type OnConnect,
  type OnEdgesChange,
  type OnNodesChange,
  Panel,
  Position,
  ReactFlow,
  ReactFlowProvider,
  addEdge,
  getBezierPath,
  useUpdateNodeInternals,
  useEdgesState,
  useNodesState,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import {
  getConnectionPortLabels,
  getNodeHandleSignature,
  validateConnectionRequest,
} from './chainConnections';
import {
  clearChainEditorAutosave,
  createChainEditorAutosave,
  createPersistedChainEditorDraft,
  extractSpaceScopedEditorSnapshot,
  loadChainEditorDraft,
  loadChainEditorAutosave,
  mergeSpaceScopedEditorSnapshot,
  saveChainEditorAutosave,
  saveChainEditorDraft,
  shouldRecoverAutosave,
  type ChainEditorAutosave,
  type PersistedChainEditorDraft,
} from './chainDraftPersistence';
import { computePackedChainLayouts, resolveAutoLayoutNodePositions } from './chainLayout';

type PrimitiveFamily = 'anchor' | 'trigger' | 'condition' | 'action' | 'counter';

type MissionNodeData = {
  kind: 'mission';
  family: PrimitiveFamily;
  stage: string;
  title: string;
  detail: string;
  status: string;
  scenario: string;
  accent: string;
  threadColor?: string;
  threadName?: string;
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
  counts?: Record<PrimitiveFamily, number>;
  sequenceCount?: number;
};

type EditorNodeData = MissionNodeData | ChainFrameData;

type ScenarioTemplate = {
  id: string;
  family: PrimitiveFamily;
  stage: string;
  title: string;
  detail: string;
  status: string;
  scenario: string;
  accent: string;
  properties: Array<{ label: string; value: string }>;
};

type SequenceStyle = {
  id: string;
  label: string;
  category: string;
  color: string;
  description: string;
};

type SequenceEdgeData = {
  sequenceId: string;
  label: string;
  category: string;
  color: string;
};

type ChainSummary = ChainFrameData & {
  nodeId: string;
  childIds: string[];
  edgeCount: number;
};

type SpaceOption = {
  id: string;
  label: string;
};

type MissionOption = {
  id: string;
  label: string;
  spaceId: string;
};

type NodeRecord = Node<EditorNodeData>;
type InspectorPanelId = 'chain' | 'sequence' | 'node';

type InspectorCollapsedState = Record<InspectorPanelId, boolean>;
type ToastItem = {
  id: number;
  message: string;
};

type EditorSnapshot = {
  edges: Edge<SequenceEdgeData>[];
  nodes: Node<EditorNodeData>[];
};

type DraftPersistenceState = {
  hydrated: boolean;
  lastSavedAt?: string;
  persistedDraft?: PersistedChainEditorDraft<EditorNodeData, SequenceEdgeData>;
  recoveredFromAutosave?: boolean;
  savedSignature: string;
  storage: 'database' | 'browser' | 'seed';
};

const inspectorPanelIds: InspectorPanelId[] = ['chain', 'sequence', 'node'];
const inspectorPanelOrderStorageKey = 'cimmeria.chain-editor.inspector-order';
const inspectorPanelCollapseStorageKey = 'cimmeria.chain-editor.inspector-collapsed';
const defaultInspectorCollapsedState: InspectorCollapsedState = {
  chain: false,
  sequence: false,
  node: false,
};

function isInspectorPanelId(value: string): value is InspectorPanelId {
  return inspectorPanelIds.includes(value as InspectorPanelId);
}

function readInspectorPanelOrder(): InspectorPanelId[] {
  if (typeof window === 'undefined') {
    return [...inspectorPanelIds];
  }

  try {
    const rawValue = window.localStorage.getItem(inspectorPanelOrderStorageKey);
    if (!rawValue) {
      return [...inspectorPanelIds];
    }

    const parsed = JSON.parse(rawValue);
    if (!Array.isArray(parsed)) {
      return [...inspectorPanelIds];
    }

    const deduped = parsed.filter((value): value is InspectorPanelId => isInspectorPanelId(value));
    const missing = inspectorPanelIds.filter((value) => !deduped.includes(value));
    return [...deduped, ...missing];
  } catch {
    return [...inspectorPanelIds];
  }
}

function readInspectorCollapsedState(): InspectorCollapsedState {
  if (typeof window === 'undefined') {
    return { ...defaultInspectorCollapsedState };
  }

  try {
    const rawValue = window.localStorage.getItem(inspectorPanelCollapseStorageKey);
    if (!rawValue) {
      return { ...defaultInspectorCollapsedState };
    }

    const parsed = JSON.parse(rawValue);
    if (!parsed || typeof parsed !== 'object') {
      return { ...defaultInspectorCollapsedState };
    }

    return {
      chain: Boolean((parsed as Partial<InspectorCollapsedState>).chain),
      sequence: Boolean((parsed as Partial<InspectorCollapsedState>).sequence),
      node: Boolean((parsed as Partial<InspectorCollapsedState>).node),
    };
  } catch {
    return { ...defaultInspectorCollapsedState };
  }
}

function reorderInspectorPanels(
  currentOrder: InspectorPanelId[],
  sourceId: InspectorPanelId,
  targetId: InspectorPanelId,
): InspectorPanelId[] {
  if (sourceId === targetId) {
    return currentOrder;
  }

  const nextOrder = currentOrder.filter((panelId) => panelId !== sourceId);
  const targetIndex = nextOrder.indexOf(targetId);

  if (targetIndex === -1) {
    return [...nextOrder, sourceId];
  }

  nextOrder.splice(targetIndex, 0, sourceId);
  return nextOrder;
}

function cloneEditorSnapshot(snapshot: EditorSnapshot): EditorSnapshot {
  if (typeof structuredClone === 'function') {
    return structuredClone(snapshot);
  }

  return JSON.parse(JSON.stringify(snapshot)) as EditorSnapshot;
}

function formatTimestampLabel(value?: string): string {
  if (!value) {
    return 'Not saved yet';
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat(undefined, {
    hour: 'numeric',
    minute: '2-digit',
    month: 'short',
    day: 'numeric',
  }).format(date);
}

const familyLabel: Record<PrimitiveFamily, string> = {
  anchor: 'Sequence',
  trigger: 'Trigger',
  condition: 'Condition',
  action: 'Action',
  counter: 'Counter',
};

const familyTint: Record<PrimitiveFamily, { chip: string; text: string; glow: string }> = {
  anchor: {
    chip: 'rgba(255,94,91,0.16)',
    text: '#ffd0cf',
    glow: 'rgba(255,94,91,0.32)',
  },
  trigger: {
    chip: 'rgba(19,162,164,0.16)',
    text: '#8ae5e2',
    glow: 'rgba(19,162,164,0.28)',
  },
  condition: {
    chip: 'rgba(245,170,49,0.16)',
    text: '#ffd38a',
    glow: 'rgba(245,170,49,0.32)',
  },
  action: {
    chip: 'rgba(34,197,94,0.16)',
    text: '#c7ffd5',
    glow: 'rgba(34,197,94,0.28)',
  },
  counter: {
    chip: 'rgba(148,163,184,0.16)',
    text: '#d4dce5',
    glow: 'rgba(148,163,184,0.24)',
  },
};

const laneDescriptors: Array<{ family: PrimitiveFamily; title: string; blurb: string }> = [
  {
    family: 'anchor',
    title: 'Sequence Lane',
    blurb: 'Visible thread markers that make each mission beat readable at a glance.',
  },
  {
    family: 'trigger',
    title: 'Trigger Lane',
    blurb: 'The runtime subscription surface: region entry, dialog choice, interact tag, player load.',
  },
  {
    family: 'condition',
    title: 'Condition Lane',
    blurb: 'Subscribe -> Check -> Act. Status, step, item, and archetype gates live here.',
  },
  {
    family: 'action',
    title: 'Action Lane',
    blurb: 'Accept, advance, complete, display dialog, grant item, and play sequence effects.',
  },
  {
    family: 'counter',
    title: 'Counter Lane',
    blurb: 'Optional runtime counters for kill counts, collection gates, and hidden background progress.',
  },
];

const orderedFamilies: PrimitiveFamily[] = [
  'anchor',
  'trigger',
  'condition',
  'action',
  'counter',
];

const AUTO_LAYOUT = {
  frameLeftPadding: 56,
  frameRightPadding: 72,
  frameTopPadding: 118,
  frameBottomPadding: 36,
  frameVerticalGap: 80,
  rowLabelWidth: 176,
  rowGap: 72,
  cardColumnGap: 420,
  cardWidth: 286,
  cardHeight: 252,
} as const;

const spaceCatalog: SpaceOption[] = [
  { id: 'Castle_CellBlock', label: 'Castle CellBlock' },
  { id: 'Castle', label: 'Castle' },
  { id: 'Harset', label: 'Harset' },
  { id: 'SGC_W1', label: 'SGC W1' },
];

const missionCatalog: MissionOption[] = [
  { id: '622', label: '622 - Arm Yourself!', spaceId: 'Castle_CellBlock' },
  { id: '638', label: '638 - Speak to Prisoner 329', spaceId: 'Castle_CellBlock' },
  { id: '639', label: '639 - Find Ambernol', spaceId: 'Castle_CellBlock' },
  { id: '680', label: '680 - Escape the Cellblock', spaceId: 'Castle_CellBlock' },
  { id: '681', label: '681 - Mess Hall Controller', spaceId: 'Castle_CellBlock' },
  { id: '684', label: '684 - Hallway03 Controller', spaceId: 'Castle_CellBlock' },
  { id: '687', label: '687 - Aftermath', spaceId: 'Castle_CellBlock' },
];

const sequencePalette: SequenceStyle[] = [
  {
    id: 'story',
    label: 'Story',
    category: 'Narrative spine',
    color: '#22c55e',
    description: 'Main story progression and critical mission beats.',
  },
  {
    id: 'combat',
    label: 'Combat',
    category: 'Encounter',
    color: '#ef4444',
    description: 'Escalations, ambushes, and active battle threads.',
  },
  {
    id: 'stealth',
    label: 'Stealth',
    category: 'Infiltration',
    color: '#3b82f6',
    description: 'Sneak routes, alert handling, and silent objectives.',
  },
  {
    id: 'puzzle',
    label: 'Puzzle',
    category: 'Logic gate',
    color: '#8b5cf6',
    description: 'Switches, sequencing, dialog branches, and interaction puzzles.',
  },
  {
    id: 'traversal',
    label: 'Traversal',
    category: 'Movement',
    color: '#06b6d4',
    description: 'Travel, ring transport, and spatial progression.',
  },
  {
    id: 'optional',
    label: 'Optional',
    category: 'Side beat',
    color: '#f59e0b',
    description: 'Bonus objectives and optional mission branches.',
  },
  {
    id: 'reward',
    label: 'Reward',
    category: 'Payout',
    color: '#eab308',
    description: 'Rewards, completion beats, and loot resolution.',
  },
  {
    id: 'debug',
    label: 'Debug',
    category: 'Temporary',
    color: '#94a3b8',
    description: 'Temporary wiring and designer-only scaffolding.',
  },
];

const sequenceAnchorCatalog: ScenarioTemplate[] = [
  {
    id: 'sequence-start',
    family: 'anchor',
    stage: 'Start',
    title: 'Sequence Start',
    detail: 'Place this at the first readable beat in a chain frame, then pull a sequence yarn into the first trigger or action.',
    status: 'Marker',
    scenario: 'Sequence anchor',
    accent: '#ff5e5b',
    properties: [
      { label: 'marker', value: 'START' },
      { label: 'behavior', value: 'Visual grouping' },
    ],
  },
  {
    id: 'sequence-end',
    family: 'anchor',
    stage: 'End',
    title: 'Sequence End',
    detail: 'Place this at the end of a beat so designers can see exactly where a mission branch resolves.',
    status: 'Marker',
    scenario: 'Sequence anchor',
    accent: '#ff5e5b',
    properties: [
      { label: 'marker', value: 'END' },
      { label: 'behavior', value: 'Visual grouping' },
    ],
  },
];

const triggerReferenceCatalog: ScenarioTemplate[] = [
  {
    id: 'trigger-enter-region',
    family: 'trigger',
    stage: 'Trigger',
    title: 'Enter Region Trigger',
    detail: 'Player enters a named spatial region such as Castle_CellBlock.Region8 or Region4.',
    status: 'ref_event_types',
    scenario: 'space event',
    accent: '#06b6d4',
    properties: [
      { label: 'event_type', value: 'enter_region' },
      { label: 'event_key_format', value: 'Zone.RegionN' },
      { label: 'scope', value: 'player or space' },
    ],
  },
  {
    id: 'trigger-interact-tag',
    family: 'trigger',
    stage: 'Trigger',
    title: 'Interact Tag Trigger',
    detail: 'Player interacts with a tagged mission entity or mission prop.',
    status: 'ref_event_types',
    scenario: 'entity event',
    accent: '#13a2a4',
    properties: [
      { label: 'event_type', value: 'interact_tag' },
      { label: 'event_key_format', value: 'Entity tag string' },
      { label: 'scope', value: 'player' },
    ],
  },
  {
    id: 'trigger-dialog-choice',
    family: 'trigger',
    stage: 'Trigger',
    title: 'Dialog Choice Trigger',
    detail: 'Player selects a dialog option, keyed by dialog id and often used to fork mission progression.',
    status: 'ref_event_types',
    scenario: 'ui event',
    accent: '#8b5cf6',
    properties: [
      { label: 'event_type', value: 'dialog_choice' },
      { label: 'event_key_format', value: 'dialog id' },
      { label: 'scope', value: 'player' },
    ],
  },
];

const conditionReferenceCatalog: ScenarioTemplate[] = [
  {
    id: 'condition-mission-status',
    family: 'condition',
    stage: 'Condition',
    title: 'Mission Status Check',
    detail: 'Check whether a mission is active, completed, failed, or not active before letting the chain continue.',
    status: 'ref_condition_types',
    scenario: 'mission gate',
    accent: '#22c55e',
    properties: [
      { label: 'condition_type', value: 'mission_status' },
      { label: 'operators', value: 'eq, neq' },
      { label: 'values', value: 'active, completed, not_active' },
    ],
  },
  {
    id: 'condition-step-status',
    family: 'condition',
    stage: 'Condition',
    title: 'Mission Step Check',
    detail: 'Gate the flow on whether a specific mission step is active, completed, or not yet started.',
    status: 'ref_condition_types',
    scenario: 'mission gate',
    accent: '#f59e0b',
    properties: [
      { label: 'condition_type', value: 'step_status' },
      { label: 'target_id', value: 'Mission ID' },
      { label: 'target_key', value: 'Step ID' },
    ],
  },
  {
    id: 'condition-archetype',
    family: 'condition',
    stage: 'Condition',
    title: 'Archetype Branch Check',
    detail: 'Split the chain on player archetype so Jaffa and Human branches stay explicit in the editor.',
    status: 'ref_condition_types',
    scenario: 'player gate',
    accent: '#3b82f6',
    properties: [
      { label: 'condition_type', value: 'archetype' },
      { label: 'operator', value: 'gte or lt' },
      { label: 'value', value: '8 or 5' },
    ],
  },
];

const actionReferenceCatalog: ScenarioTemplate[] = [
  {
    id: 'action-accept-mission',
    family: 'action',
    stage: 'Action',
    title: 'Accept Mission Action',
    detail: 'Player accepts a mission immediately. Useful for space-script style orchestration chains.',
    status: 'ref_action_types',
    scenario: 'mission action',
    accent: '#22c55e',
    properties: [
      { label: 'action_type', value: 'accept_mission' },
      { label: 'target_id', value: 'Mission ID' },
      { label: 'delay_ms', value: '0' },
    ],
  },
  {
    id: 'action-advance-step',
    family: 'action',
    stage: 'Action',
    title: 'Advance Step Action',
    detail: 'Advance to a specified mission step inside a chain after the condition lane passes.',
    status: 'ref_action_types',
    scenario: 'mission action',
    accent: '#06b6d4',
    properties: [
      { label: 'action_type', value: 'advance_step' },
      { label: 'target_key', value: 'Step ID' },
      { label: 'delay_ms', value: '0' },
    ],
  },
  {
    id: 'action-display-dialog',
    family: 'action',
    stage: 'Action',
    title: 'Display Dialog Action',
    detail: 'Open a dialog immediately, often as a bridge between mission beats and player-facing story.',
    status: 'ref_action_types',
    scenario: 'ui action',
    accent: '#8b5cf6',
    properties: [
      { label: 'action_type', value: 'display_dialog' },
      { label: 'target_id', value: 'Dialog ID' },
      { label: 'params', value: '{ npc: null }' },
    ],
  },
  {
    id: 'action-display-rewards',
    family: 'action',
    stage: 'Action',
    title: 'Display Rewards Action',
    detail: 'Open the reward selection surface at the end of a sequence or a mission completion handoff.',
    status: 'ref_action_types',
    scenario: 'mission action',
    accent: '#eab308',
    properties: [
      { label: 'action_type', value: 'display_rewards' },
      { label: 'category', value: 'mission' },
      { label: 'delay_ms', value: '0' },
    ],
  },
];

const counterReferenceCatalog: ScenarioTemplate[] = [
  {
    id: 'counter-threshold',
    family: 'counter',
    stage: 'Counter',
    title: 'Counter Threshold',
    detail: 'Track hidden hallway controllers, kill counts, or collection beats before an action fires.',
    status: 'content_counters',
    scenario: 'runtime counter',
    accent: '#94a3b8',
    properties: [
      { label: 'counter_key', value: 'hallway_01_progress' },
      { label: 'operator', value: 'gte' },
      { label: 'value', value: '1' },
    ],
  },
];

const initialNodes: NodeRecord[] = [
  {
    id: 'chain-arm-yourself',
    type: 'chainFrame',
    position: { x: -340, y: 20 },
    style: { width: 1710, height: 390 },
    data: {
      kind: 'chain',
      name: 'Arm Yourself! Onboarding',
      summary:
        'Space-scoped opening content chain. The zone seeds mission 622, then hands the player into the first explicit mission beat.',
      scopeType: 'mission',
      scopeId: '622',
      enabled: true,
      priority: 100,
      source: 'content_chains #1',
      semantic: 'Story',
      color: '#22c55e',
      spaceId: 'Castle_CellBlock',
      missionId: '622',
    },
  },
  {
    id: 'chain-frost-body',
    type: 'chainFrame',
    position: { x: -120, y: 470 },
    style: { width: 1920, height: 410 },
    data: {
      kind: 'chain',
      name: 'Frost Body Hand-off',
      summary:
        'Mission-scoped chain that reacts to the corpse dialog, grants the weapon and letter, and bridges into Speak to Prisoner 329.',
      scopeType: 'mission',
      scopeId: '622',
      enabled: true,
      priority: 110,
      source: 'content_chains #2-3',
      semantic: 'Reward',
      color: '#eab308',
      spaceId: 'Castle_CellBlock',
      missionId: '622',
    },
  },
  {
    id: 'chain-prisoner-329',
    type: 'chainFrame',
    position: { x: 160, y: 960 },
    style: { width: 1760, height: 390 },
    data: {
      kind: 'chain',
      name: 'Prisoner 329 Branch',
      summary:
        'Mission 638 branch logic. Interact-tag input routes through archetype checks and dialog choice completion into mission 639.',
      scopeType: 'mission',
      scopeId: '638',
      enabled: true,
      priority: 120,
      source: 'content_chains #4-7',
      semantic: 'Puzzle',
      color: '#8b5cf6',
      spaceId: 'Castle_CellBlock',
      missionId: '638',
    },
  },
  {
    id: 'chain-hallway-controller',
    type: 'chainFrame',
    position: { x: 560, y: 1420 },
    style: { width: 1480, height: 350 },
    data: {
      kind: 'chain',
      name: 'Hidden Hallway Controller',
      summary:
        'Background space chain for hidden controller missions. This is the type of invisible progression the docs say must stay visible in tooling.',
      scopeType: 'mission',
      scopeId: '684',
      enabled: true,
      priority: 90,
      source: 'mission-chains docs',
      semantic: 'Debug',
      color: '#94a3b8',
      spaceId: 'Castle_CellBlock',
      missionId: '684',
    },
  },
  {
    id: 's1',
    parentId: 'chain-arm-yourself',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 70, y: 165 },
    data: {
      kind: 'mission',
      family: 'anchor',
      stage: 'Start',
      title: 'Sequence Start',
      detail: 'The first visible onboarding beat inside the Arm Yourself opening frame.',
      status: 'Marker',
      scenario: 'Primary sequence',
      accent: '#ff5e5b',
      properties: [
        { label: 'marker', value: 'START' },
        { label: 'thread', value: 'story_thread_01' },
      ],
    },
  },
  {
    id: '1',
    parentId: 'chain-arm-yourself',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 300, y: 110 },
    data: {
      kind: 'mission',
      family: 'trigger',
      stage: 'Trigger',
      title: 'Enter Region8',
      detail: 'Player enters Castle_CellBlock.Region8. This replaces the hidden space-script opening with an explicit content trigger card.',
      status: 'content_triggers',
      scenario: 'player region entry',
      accent: '#13a2a4',
      properties: [
        { label: 'event_type', value: 'enter_region' },
        { label: 'event_key', value: 'Castle_CellBlock.Region8' },
        { label: 'scope', value: 'space' },
      ],
    },
  },
  {
    id: '2',
    parentId: 'chain-arm-yourself',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 690, y: 110 },
    data: {
      kind: 'mission',
      family: 'condition',
      stage: 'Condition',
      title: 'Mission 622 Not Active',
      detail: 'Gate the opening chain so it only seeds the player once and does not replay after completion.',
      status: 'content_conditions',
      scenario: 'mission status gate',
      accent: '#f5aa31',
      properties: [
        { label: 'condition_type', value: 'mission_status' },
        { label: 'target_id', value: '622' },
        { label: 'value', value: 'not_active' },
      ],
    },
  },
  {
    id: '3',
    parentId: 'chain-arm-yourself',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 1075, y: 95 },
    data: {
      kind: 'mission',
      family: 'action',
      stage: 'Action',
      title: 'Accept 622 + Seed Dialog',
      detail: 'Accept Arm Yourself, display dialog 2982, and wire the first corpse interaction set so the onboarding beat is visible to designers.',
      status: 'content_actions',
      scenario: 'space orchestrator',
      accent: '#22c55e',
      properties: [
        { label: 'action_type', value: 'accept_mission' },
        { label: 'dialog_id', value: '2982' },
        { label: 'dialog_set', value: '5229' },
      ],
    },
  },
  {
    id: 'e1',
    parentId: 'chain-arm-yourself',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 1415, y: 165 },
    data: {
      kind: 'mission',
      family: 'anchor',
      stage: 'End',
      title: 'Sequence End',
      detail: 'The space chain hands the player into the mission-scoped Frost body sequence at this point.',
      status: 'Marker',
      scenario: 'Primary sequence',
      accent: '#ff5e5b',
      properties: [
        { label: 'marker', value: 'END' },
        { label: 'thread', value: 'story_thread_01' },
      ],
    },
  },
  {
    id: 's2',
    parentId: 'chain-frost-body',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 70, y: 180 },
    data: {
      kind: 'mission',
      family: 'anchor',
      stage: 'Start',
      title: 'Sequence Start',
      detail: 'The Frost body mission hand-off opens here for the content-engine version of mission 622.',
      status: 'Marker',
      scenario: 'Loot and hand-off',
      accent: '#ff5e5b',
      properties: [
        { label: 'marker', value: 'START' },
        { label: 'thread', value: 'reward_thread_01' },
      ],
    },
  },
  {
    id: '4',
    parentId: 'chain-frost-body',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 270, y: 110 },
    data: {
      kind: 'mission',
      family: 'trigger',
      stage: 'Trigger',
      title: 'Interact Wooden Crate',
      detail: 'Player interacts with Cellblock_WoodenCrate while the active mission beat is in progress.',
      status: 'content_triggers',
      scenario: 'mission interaction',
      accent: '#13a2a4',
      properties: [
        { label: 'event_type', value: 'interact_tag' },
        { label: 'event_key', value: 'Cellblock_WoodenCrate' },
        { label: 'scope', value: 'player' },
      ],
    },
  },
  {
    id: '5',
    parentId: 'chain-frost-body',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 640, y: 110 },
    data: {
      kind: 'mission',
      family: 'condition',
      stage: 'Condition',
      title: 'Step 2114 Active',
      detail: 'Only continue if the player is on the active mission step that corresponds to locating the weapon and letter.',
      status: 'content_conditions',
      scenario: 'step gate',
      accent: '#f5aa31',
      properties: [
        { label: 'condition_type', value: 'step_status' },
        { label: 'target_id', value: '622' },
        { label: 'target_key', value: '2114' },
      ],
    },
  },
  {
    id: '6',
    parentId: 'chain-frost-body',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 1010, y: 95 },
    data: {
      kind: 'mission',
      family: 'action',
      stage: 'Action',
      title: 'Grant Pistol + Letter',
      detail: 'Grant the starter weapon, Frost letter, and present the story-facing dialog beat that closes mission 622.',
      status: 'content_actions',
      scenario: 'item grant',
      accent: '#22c55e',
      properties: [
        { label: 'action_type', value: 'add_item + display_dialog' },
        { label: 'item_id', value: '55 + 3730' },
        { label: 'dialog_id', value: '3995' },
      ],
    },
  },
  {
    id: '7',
    parentId: 'chain-frost-body',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 1380, y: 110 },
    data: {
      kind: 'mission',
      family: 'trigger',
      stage: 'Trigger',
      title: 'Dialog Open 3995',
      detail: 'The corpse dialog is the visible trigger that closes Arm Yourself and bridges into the next mission.',
      status: 'content_triggers',
      scenario: 'dialog event',
      accent: '#8b5cf6',
      properties: [
        { label: 'event_type', value: 'dialog_open' },
        { label: 'event_key', value: '3995' },
        { label: 'scope', value: 'player' },
      ],
    },
  },
  {
    id: '8',
    parentId: 'chain-frost-body',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 1680, y: 95 },
    data: {
      kind: 'mission',
      family: 'action',
      stage: 'Action',
      title: 'Complete 622 + Accept 638',
      detail: 'Explicit handoff card that makes the next mission link visible on the canvas instead of hiding it in a space script.',
      status: 'content_actions',
      scenario: 'mission handoff',
      accent: '#22c55e',
      properties: [
        { label: 'action_type', value: 'complete_mission + accept_mission' },
        { label: 'target_id', value: '622 -> 638' },
        { label: 'sequence_id', value: '10000' },
      ],
    },
  },
  {
    id: 'e2',
    parentId: 'chain-frost-body',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 1685, y: 250 },
    data: {
      kind: 'mission',
      family: 'anchor',
      stage: 'End',
      title: 'Sequence End',
      detail: 'Mission 622 resolves here and the next mission-owned chain takes over.',
      status: 'Marker',
      scenario: 'Loot and hand-off',
      accent: '#ff5e5b',
      properties: [
        { label: 'marker', value: 'END' },
        { label: 'thread', value: 'reward_thread_01' },
      ],
    },
  },
  {
    id: 's3',
    parentId: 'chain-prisoner-329',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 70, y: 160 },
    data: {
      kind: 'mission',
      family: 'anchor',
      stage: 'Start',
      title: 'Sequence Start',
      detail: 'Prisoner 329 branch opens here after mission 638 is accepted.',
      status: 'Marker',
      scenario: 'Dialog branch',
      accent: '#ff5e5b',
      properties: [
        { label: 'marker', value: 'START' },
        { label: 'thread', value: 'puzzle_thread_01' },
      ],
    },
  },
  {
    id: '9',
    parentId: 'chain-prisoner-329',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 285, y: 105 },
    data: {
      kind: 'mission',
      family: 'trigger',
      stage: 'Trigger',
      title: 'Interact 329 Cell Door Button',
      detail: 'Interact-tag entry point for the Prisoner 329 mission branch.',
      status: 'content_triggers',
      scenario: 'button interact',
      accent: '#13a2a4',
      properties: [
        { label: 'event_type', value: 'interact_tag' },
        { label: 'event_key', value: '329_CellDoorButton' },
        { label: 'scope', value: 'player' },
      ],
    },
  },
  {
    id: '10',
    parentId: 'chain-prisoner-329',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 660, y: 105 },
    data: {
      kind: 'mission',
      family: 'condition',
      stage: 'Condition',
      title: 'Archetype Branch',
      detail: 'Resolve whether the player should see the Jaffa or Human dialog path before the door unlock completes.',
      status: 'content_conditions',
      scenario: 'archetype gate',
      accent: '#f5aa31',
      outputs: ['Jaffa path', 'Human path'],
      outputConditions: ['archetype >= 8', 'archetype < 5'],
      properties: [
        { label: 'condition_type', value: 'archetype' },
        { label: 'operator', value: 'gte or lt' },
        { label: 'value', value: '8 / 5' },
      ],
    },
  },
  {
    id: '11',
    parentId: 'chain-prisoner-329',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 1020, y: 105 },
    data: {
      kind: 'mission',
      family: 'trigger',
      stage: 'Trigger',
      title: 'Dialog Choice 5020 / 2299',
      detail: 'The branch converges on a dialog-choice trigger that finalizes the cell door release path.',
      status: 'content_triggers',
      scenario: 'dialog choice',
      accent: '#8b5cf6',
      inputs: ['Jaffa branch', 'Human branch'],
      outputs: ['Choice 5020', 'Choice 2299'],
      outputConditions: ['dialog.choice 5020', 'dialog.choice 2299'],
      properties: [
        { label: 'event_type', value: 'dialog_choice' },
        { label: 'event_key', value: '5020 or 2299' },
        { label: 'scope', value: 'player' },
      ],
    },
  },
  {
    id: '12',
    parentId: 'chain-prisoner-329',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 1370, y: 90 },
    data: {
      kind: 'mission',
      family: 'action',
      stage: 'Action',
      title: 'Advance 638 + Accept 639',
      detail: 'Advance the step, play the unlock sequence, complete mission 638, and accept mission 639 in one explicit hand-off card.',
      status: 'content_actions',
      scenario: 'branch convergence',
      accent: '#22c55e',
      inputs: ['Resolve Jaffa', 'Resolve Human'],
      outputConditions: [],
      properties: [
        { label: 'action_type', value: 'advance_step + complete_mission + accept_mission' },
        { label: 'target_id', value: '638 -> 639' },
        { label: 'sequence_id', value: '1749' },
      ],
    },
  },
  {
    id: 'e3',
    parentId: 'chain-prisoner-329',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 1390, y: 235 },
    data: {
      kind: 'mission',
      family: 'anchor',
      stage: 'End',
      title: 'Sequence End',
      detail: 'Mission 638 resolves here and hands into Find Ambernol.',
      status: 'Marker',
      scenario: 'Dialog branch',
      accent: '#ff5e5b',
      properties: [
        { label: 'marker', value: 'END' },
        { label: 'thread', value: 'puzzle_thread_01' },
      ],
    },
  },
  {
    id: 's4',
    parentId: 'chain-hallway-controller',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 65, y: 135 },
    data: {
      kind: 'mission',
      family: 'anchor',
      stage: 'Start',
      title: 'Sequence Start',
      detail: 'Hidden controller missions stay visible in the editor even when players never see them in the tracker.',
      status: 'Marker',
      scenario: 'Hidden background',
      accent: '#ff5e5b',
      properties: [
        { label: 'marker', value: 'START' },
        { label: 'thread', value: 'controller_thread_01' },
      ],
    },
  },
  {
    id: '13',
    parentId: 'chain-hallway-controller',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 305, y: 90 },
    data: {
      kind: 'mission',
      family: 'trigger',
      stage: 'Trigger',
      title: 'Enter Region4',
      detail: 'Space trigger that accepts the hidden Hallway03 controller mission on region entry.',
      status: 'content_triggers',
      scenario: 'hidden controller',
      accent: '#13a2a4',
      properties: [
        { label: 'event_type', value: 'enter_region' },
        { label: 'event_key', value: 'Castle_CellBlock.Region4' },
        { label: 'scope', value: 'space' },
      ],
    },
  },
  {
    id: '14',
    parentId: 'chain-hallway-controller',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 650, y: 90 },
    data: {
      kind: 'mission',
      family: 'condition',
      stage: 'Condition',
      title: 'Mission 684 Not Active',
      detail: 'Keep the hidden controller idempotent so the background hallway chain only seeds once.',
      status: 'content_conditions',
      scenario: 'hidden mission gate',
      accent: '#f5aa31',
      properties: [
        { label: 'condition_type', value: 'mission_status' },
        { label: 'target_id', value: '684' },
        { label: 'value', value: 'not_active' },
      ],
    },
  },
  {
    id: '15',
    parentId: 'chain-hallway-controller',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 1015, y: 80 },
    data: {
      kind: 'mission',
      family: 'action',
      stage: 'Action',
      title: 'Accept 684 Background Mission',
      detail: 'Designer-visible representation of the hidden hallway controller mission that the docs call out as parallel background logic.',
      status: 'content_actions',
      scenario: 'hidden controller',
      accent: '#22c55e',
      properties: [
        { label: 'action_type', value: 'accept_mission' },
        { label: 'target_id', value: '684' },
        { label: 'is_hidden', value: 'true' },
      ],
    },
  },
  {
    id: 'e4',
    parentId: 'chain-hallway-controller',
    extent: 'parent',
    type: 'missionNode',
    position: { x: 1140, y: 185 },
    data: {
      kind: 'mission',
      family: 'anchor',
      stage: 'End',
      title: 'Sequence End',
      detail: 'Background controller beat complete. The player never sees it, but designers still need to understand it.',
      status: 'Marker',
      scenario: 'Hidden background',
      accent: '#ff5e5b',
      properties: [
        { label: 'marker', value: 'END' },
        { label: 'thread', value: 'controller_thread_01' },
      ],
    },
  },
];

const initialEdges: Edge<SequenceEdgeData>[] = [
  {
    id: 'seq-arm-yourself-start',
    source: 's1',
    target: '1',
    type: 'sequenceThread',
    data: {
      sequenceId: 'primary-thread',
      label: 'Primary Thread',
      category: 'Story',
      color: '#22c55e',
    },
  },
  {
    id: 'seq-arm-yourself-middle',
    source: '1',
    target: '2',
    type: 'sequenceThread',
    data: {
      sequenceId: 'primary-thread',
      label: 'Primary Thread',
      category: 'Story',
      color: '#22c55e',
    },
  },
  {
    id: 'seq-arm-yourself-action',
    source: '2',
    target: '3',
    type: 'sequenceThread',
    data: {
      sequenceId: 'primary-thread',
      label: 'Primary Thread',
      category: 'Story',
      color: '#22c55e',
    },
  },
  {
    id: 'seq-arm-yourself-end',
    source: '3',
    target: 'e1',
    type: 'sequenceThread',
    data: {
      sequenceId: 'primary-thread',
      label: 'Primary Thread',
      category: 'Story',
      color: '#22c55e',
    },
  },
  {
    id: 'seq-frost-start',
    source: 's2',
    target: '4',
    type: 'sequenceThread',
    data: {
      sequenceId: 'reward-thread',
      label: 'Frost Loot Thread',
      category: 'Reward',
      color: '#eab308',
    },
  },
  {
    id: 'seq-frost-middle-1',
    source: '4',
    target: '5',
    type: 'sequenceThread',
    data: {
      sequenceId: 'reward-thread',
      label: 'Frost Loot Thread',
      category: 'Reward',
      color: '#eab308',
    },
  },
  {
    id: 'seq-frost-middle-2',
    source: '5',
    target: '6',
    type: 'sequenceThread',
    data: {
      sequenceId: 'reward-thread',
      label: 'Frost Loot Thread',
      category: 'Reward',
      color: '#eab308',
    },
  },
  {
    id: 'seq-frost-middle-3',
    source: '6',
    target: '7',
    type: 'sequenceThread',
    data: {
      sequenceId: 'reward-thread',
      label: 'Frost Loot Thread',
      category: 'Reward',
      color: '#eab308',
    },
  },
  {
    id: 'seq-frost-middle-4',
    source: '7',
    target: '8',
    type: 'sequenceThread',
    data: {
      sequenceId: 'reward-thread',
      label: 'Frost Loot Thread',
      category: 'Reward',
      color: '#eab308',
    },
  },
  {
    id: 'seq-frost-end',
    source: '8',
    target: 'e2',
    type: 'sequenceThread',
    data: {
      sequenceId: 'reward-thread',
      label: 'Frost Loot Thread',
      category: 'Reward',
      color: '#eab308',
    },
  },
  {
    id: 'seq-prisoner-start',
    source: 's3',
    target: '9',
    type: 'sequenceThread',
    data: {
      sequenceId: 'puzzle-thread',
      label: 'Prisoner Branch',
      category: 'Puzzle',
      color: '#8b5cf6',
    },
  },
  {
    id: 'seq-prisoner-middle-1',
    source: '9',
    target: '10',
    sourceHandle: 'output-0',
    targetHandle: 'input-0',
    type: 'sequenceThread',
    data: {
      sequenceId: 'puzzle-thread',
      label: 'Prisoner Branch',
      category: 'Puzzle',
      color: '#8b5cf6',
    },
  },
  {
    id: 'seq-prisoner-middle-2',
    source: '10',
    target: '11',
    sourceHandle: 'output-0',
    targetHandle: 'input-0',
    type: 'sequenceThread',
    data: {
      sequenceId: 'puzzle-thread',
      label: 'Prisoner Branch',
      category: 'Puzzle',
      color: '#8b5cf6',
    },
  },
  {
    id: 'seq-prisoner-middle-3',
    source: '11',
    target: '12',
    sourceHandle: 'output-0',
    targetHandle: 'input-0',
    type: 'sequenceThread',
    data: {
      sequenceId: 'puzzle-thread',
      label: 'Prisoner Branch',
      category: 'Puzzle',
      color: '#8b5cf6',
    },
  },
  {
    id: 'seq-prisoner-end',
    source: '12',
    target: 'e3',
    type: 'sequenceThread',
    data: {
      sequenceId: 'puzzle-thread',
      label: 'Prisoner Branch',
      category: 'Puzzle',
      color: '#8b5cf6',
    },
  },
  {
    id: 'seq-hidden-start',
    source: 's4',
    target: '13',
    type: 'sequenceThread',
    data: {
      sequenceId: 'controller-thread',
      label: 'Hidden Controller',
      category: 'Debug',
      color: '#94a3b8',
    },
  },
  {
    id: 'seq-hidden-middle-1',
    source: '13',
    target: '14',
    type: 'sequenceThread',
    data: {
      sequenceId: 'controller-thread',
      label: 'Hidden Controller',
      category: 'Debug',
      color: '#94a3b8',
    },
  },
  {
    id: 'seq-hidden-middle-2',
    source: '14',
    target: '15',
    type: 'sequenceThread',
    data: {
      sequenceId: 'controller-thread',
      label: 'Hidden Controller',
      category: 'Debug',
      color: '#94a3b8',
    },
  },
  {
    id: 'seq-hidden-end',
    source: '15',
    target: 'e4',
    type: 'sequenceThread',
    data: {
      sequenceId: 'controller-thread',
      label: 'Hidden Controller',
      category: 'Debug',
      color: '#94a3b8',
    },
  },
  {
    id: 'link-space-to-mission',
    source: '3',
    target: 's2',
    type: 'smoothstep',
    animated: true,
    style: {
      stroke: '#5eb8b3',
      strokeDasharray: '8 8',
      strokeWidth: 2,
    },
  },
  {
    id: 'link-mission-to-prisoner',
    source: '8',
    target: 's3',
    type: 'smoothstep',
    animated: true,
    style: {
      stroke: '#5eb8b3',
      strokeDasharray: '8 8',
      strokeWidth: 2,
    },
  },
  {
    id: 'link-prisoner-to-hidden',
    source: '12',
    target: 's4',
    type: 'smoothstep',
    animated: true,
    style: {
      stroke: '#5eb8b3',
      strokeDasharray: '8 8',
      strokeWidth: 2,
    },
  },
  {
    id: 'branch-prisoner-human',
    source: '10',
    sourceHandle: 'output-1',
    target: '11',
    targetHandle: 'input-1',
    type: 'smoothstep',
    animated: true,
    style: {
      stroke: '#f59e0b',
      strokeDasharray: '8 8',
      strokeWidth: 2,
    },
  },
  {
    id: 'branch-choice-human',
    source: '11',
    sourceHandle: 'output-1',
    target: '12',
    targetHandle: 'input-1',
    type: 'smoothstep',
    animated: true,
    style: {
      stroke: '#f97316',
      strokeDasharray: '8 8',
      strokeWidth: 2,
    },
  },
];

function isMissionData(data: EditorNodeData): data is MissionNodeData {
  return data.kind === 'mission';
}

function isChainData(data: EditorNodeData): data is ChainFrameData {
  return data.kind === 'chain';
}

function getPortLabels(data: MissionNodeData) {
  return getConnectionPortLabels({ data });
}

function getOutputConditions(data: MissionNodeData) {
  const { outputs } = getPortLabels(data);

  return outputs.map((_, index) => data.outputConditions?.[index] ?? '');
}

function getOutputRuleText(data: MissionNodeData, handleId?: string | null) {
  if (!handleId?.startsWith('output-')) {
    return '';
  }

  const index = Number(handleId.replace('output-', ''));
  if (Number.isNaN(index)) {
    return '';
  }

  const { outputs } = getPortLabels(data);
  const outputLabel = outputs[index] ?? '';
  const outputCondition = getOutputConditions(data)[index] ?? '';

  if (!outputLabel && !outputCondition) {
    return '';
  }

  if (!outputCondition) {
    return outputLabel;
  }

  if (!outputLabel) {
    return outputCondition;
  }

  return `${outputLabel} · ${outputCondition}`;
}

function getPortTop(index: number, count: number) {
  if (count <= 1) {
    return '50%';
  }

  return `${((index + 1) / (count + 1)) * 100}%`;
}

function getChainFrameOrder(
  visibleChains: ChainSummary[],
  nodesById: Map<string, Node<EditorNodeData>>,
  edges: Array<Edge<SequenceEdgeData>>,
) {
  const visibleChainIds = new Set(visibleChains.map((chain) => chain.nodeId));
  const adjacency = new Map<string, Set<string>>();
  const incoming = new Map<string, number>();
  const baseOrder = new Map(visibleChains.map((chain, index) => [chain.nodeId, index] as const));

  for (const chain of visibleChains) {
    adjacency.set(chain.nodeId, new Set());
    incoming.set(chain.nodeId, 0);
  }

  for (const edge of edges) {
    if (edge.type === 'sequenceThread') {
      continue;
    }

    const sourceNode = nodesById.get(edge.source);
    const targetNode = nodesById.get(edge.target);
    const sourceChainId = sourceNode?.parentId;
    const targetChainId = targetNode?.parentId;

    if (
      !sourceChainId ||
      !targetChainId ||
      sourceChainId === targetChainId ||
      !visibleChainIds.has(sourceChainId) ||
      !visibleChainIds.has(targetChainId)
    ) {
      continue;
    }

    const nextSet = adjacency.get(sourceChainId);
    if (!nextSet || nextSet.has(targetChainId)) {
      continue;
    }

    nextSet.add(targetChainId);
    incoming.set(targetChainId, (incoming.get(targetChainId) ?? 0) + 1);
  }

  const queue = visibleChains
    .filter((chain) => (incoming.get(chain.nodeId) ?? 0) === 0)
    .sort((a, b) => (baseOrder.get(a.nodeId) ?? 0) - (baseOrder.get(b.nodeId) ?? 0))
    .map((chain) => chain.nodeId);
  const ordered: string[] = [];

  while (queue.length) {
    const current = queue.shift();
    if (!current) {
      continue;
    }

    ordered.push(current);

    const nextChains = [...(adjacency.get(current) ?? [])].sort(
      (a, b) => (baseOrder.get(a) ?? 0) - (baseOrder.get(b) ?? 0),
    );

    for (const next of nextChains) {
      const nextIncoming = (incoming.get(next) ?? 0) - 1;
      incoming.set(next, nextIncoming);
      if (nextIncoming === 0) {
        queue.push(next);
      }
    }
  }

  for (const chain of visibleChains) {
    if (!ordered.includes(chain.nodeId)) {
      ordered.push(chain.nodeId);
    }
  }

  return ordered;
}

const ChainFrameNode = memo(({ data, selected }: NodeProps<Node<EditorNodeData>>) => {
  const chainData = data as ChainFrameData;
  const activeFamilies = orderedFamilies.filter((family) => (chainData.counts?.[family] ?? 0) > 0);
  const visibleFamilies: PrimitiveFamily[] = activeFamilies.length
    ? activeFamilies
    : ['trigger', 'condition', 'action'];

  return (
    <div
      className="relative h-full w-full overflow-hidden rounded-[34px] border shadow-[0_30px_90px_rgba(0,0,0,0.34)] backdrop-blur-xl"
      style={{
        borderColor: selected ? `${chainData.color}88` : `${chainData.color}44`,
        background: `linear-gradient(180deg, rgba(9,18,28,0.94), rgba(7,14,22,0.9))`,
        boxShadow: selected
          ? `0 0 0 1px ${chainData.color}44, 0 28px 90px rgba(0,0,0,0.34)`
          : `0 28px 90px rgba(0,0,0,0.34)`,
      }}
    >
      <div
        className="absolute inset-0"
        style={{
          background: `radial-gradient(circle at top left, ${chainData.color}18, transparent 42%), radial-gradient(circle at bottom right, ${chainData.color}10, transparent 35%)`,
        }}
      />

      <div className="relative z-10 flex items-start justify-between gap-4 border-b border-white/6 px-6 py-5">
        <div>
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
          <h3 className="mt-3 text-2xl font-semibold tracking-[-0.05em] text-white">
            {chainData.name}
          </h3>
          <p className="mt-2 max-w-4xl text-sm leading-6 text-[rgba(224,231,239,0.76)]">
            {chainData.summary}
          </p>
        </div>

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

const MissionNode = memo(({ data, id, selected }: NodeProps<Node<EditorNodeData>>) => {
  const updateNodeInternals = useUpdateNodeInternals();
  const missionData = data as MissionNodeData;
  const tint = familyTint[missionData.family];
  const { inputs, outputs } = getPortLabels(missionData);
  const handleSignature = getNodeHandleSignature({ data: missionData });

  useEffect(() => {
    updateNodeInternals(id);
  }, [handleSignature, id, updateNodeInternals]);

  return (
    <div
      className={`relative min-w-[250px] rounded-[28px] border px-5 py-4 shadow-[0_22px_50px_rgba(0,0,0,0.28)] backdrop-blur-lg transition-all ${
        selected
          ? 'border-[#f5aa31] bg-[rgba(245,170,49,0.14)]'
          : 'border-[rgba(255,255,255,0.08)] bg-[rgba(9,18,28,0.9)]'
      }`}
      style={{
        boxShadow: selected
          ? `0 0 0 1px ${missionData.accent}33, 0 22px 50px rgba(0,0,0,0.28)`
          : undefined,
      }}
    >
      <div
        className="absolute left-4 top-4 h-2.5 w-2.5 rounded-full shadow-[0_0_18px_rgba(255,255,255,0.25)]"
        style={{ backgroundColor: missionData.threadColor ?? missionData.accent }}
      />
      {missionData.threadColor && (missionData.stage === 'Start' || missionData.stage === 'End') ? (
        <div
          className="absolute inset-x-0 top-0 h-1 rounded-t-[28px]"
          style={{ backgroundColor: missionData.threadColor }}
        />
      ) : null}

      <div className="ml-5 flex items-center justify-between gap-3">
        <span
          className="rounded-full px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.22em]"
          style={{
            backgroundColor: tint.chip,
            color: tint.text,
          }}
        >
          {familyLabel[missionData.family]}
        </span>
        <span
          className="rounded-full px-3 py-1 text-[11px] font-medium"
          style={{
            backgroundColor: missionData.threadColor ? `${missionData.threadColor}22` : tint.chip,
            color: missionData.threadColor ?? tint.text,
          }}
        >
          {missionData.status}
        </span>
      </div>

      <h3 className="mt-4 text-lg font-semibold tracking-[-0.04em] text-white">
        {missionData.title}
      </h3>
      <p className="mt-1 text-xs uppercase tracking-[0.22em] text-[rgba(160,174,192,0.72)]">
        {missionData.scenario}
      </p>

      {missionData.threadColor && missionData.threadName ? (
        <div className="mt-2">
          <span
            className="rounded-full px-3 py-1 text-[10px] font-semibold uppercase tracking-[0.2em]"
            style={{
              backgroundColor: `${missionData.threadColor}22`,
              border: `1px solid ${missionData.threadColor}55`,
              color: missionData.threadColor,
            }}
          >
            {missionData.threadName}
          </span>
        </div>
      ) : null}

      <p className="mt-3 text-sm leading-6 text-[rgba(224,231,239,0.76)]">{missionData.detail}</p>

      <div className="mt-4 space-y-2 rounded-[20px] border border-[rgba(255,255,255,0.06)] bg-[rgba(0,0,0,0.18)] p-3">
        {missionData.properties.slice(0, 3).map((item) => (
          <div className="flex items-center justify-between gap-3 text-xs" key={item.label}>
            <span className="uppercase tracking-[0.2em] text-[rgba(160,174,192,0.74)]">
              {item.label}
            </span>
            <span className="font-mono text-[rgba(244,247,250,0.92)]">{item.value}</span>
          </div>
        ))}
      </div>

      {inputs.map((label, index) => {
        const top = getPortTop(index, inputs.length);
        return (
          <div
            className="pointer-events-none absolute -left-[108px] hidden -translate-y-1/2 xl:block"
            key={`input-label-${label}-${index}`}
            style={{ top }}
          >
            <span className="block max-w-[92px] overflow-hidden text-ellipsis whitespace-nowrap rounded-full border border-[rgba(255,94,91,0.28)] bg-[rgba(255,94,91,0.14)] px-2 py-1 text-[9px] font-semibold uppercase tracking-[0.12em] text-[#ffd0cf]">
              {label}
            </span>
          </div>
        );
      })}
      {outputs.map((label, index) => {
        const top = getPortTop(index, outputs.length);
        return (
          <div
            className="pointer-events-none absolute -right-[108px] hidden -translate-y-1/2 xl:block"
            key={`output-label-${label}-${index}`}
            style={{ top }}
          >
            <span className="block max-w-[92px] overflow-hidden text-ellipsis whitespace-nowrap rounded-full border border-[rgba(255,94,91,0.28)] bg-[rgba(255,94,91,0.14)] px-2 py-1 text-[9px] font-semibold uppercase tracking-[0.12em] text-[#ffd0cf]">
              {label}
            </span>
          </div>
        );
      })}

      {inputs.map((_, index) => (
        <Handle
          id={`input-${index}`}
          key={`input-${index}`}
          position={Position.Left}
          style={{
            background: '#ff5e5b',
            border: '2px solid rgba(255, 208, 207, 0.9)',
            boxShadow: `0 0 0 4px ${familyTint.anchor.glow}`,
            height: 16,
            left: -10,
            top: getPortTop(index, inputs.length),
            width: 16,
          }}
          type="target"
        />
      ))}
      {outputs.map((_, index) => (
        <Handle
          id={`output-${index}`}
          key={`output-${index}`}
          position={Position.Right}
          style={{
            background: '#ff5e5b',
            border: '2px solid rgba(255, 208, 207, 0.9)',
            boxShadow: `0 0 0 4px ${familyTint.anchor.glow}`,
            height: 16,
            right: -10,
            top: getPortTop(index, outputs.length),
            width: 16,
          }}
          type="source"
        />
      ))}
    </div>
  );
});

const SequenceThreadEdge = memo(
  ({
    id,
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
    data,
    selected,
  }: EdgeProps) => {
    const [path, labelX, labelY] = getBezierPath({
      sourceX,
      sourceY,
      targetX,
      targetY,
      sourcePosition,
      targetPosition,
      curvature: 0.28,
    });
    const color = typeof data?.color === 'string' ? data.color : '#ff5e5b';

    return (
      <>
        <BaseEdge
          id={`${id}-glow`}
          path={path}
          style={{
            stroke: `${color}33`,
            strokeWidth: selected ? 14 : 10,
            strokeLinecap: 'round',
          }}
        />
        <BaseEdge
          id={id}
          path={path}
          style={{
            stroke: color,
            strokeWidth: selected ? 5 : 4,
            strokeLinecap: 'round',
          }}
        />
        <circle cx={sourceX} cy={sourceY} fill={color} r={6} />
        <circle cx={targetX} cy={targetY} fill="#e5eef3" r={5} stroke={color} strokeWidth={2} />

        <foreignObject height={48} width={220} x={labelX - 110} y={labelY - 24}>
          <div className="flex h-full items-center justify-center">
            <span
              className="inline-flex items-center gap-2 rounded-full px-4 py-2 text-[11px] font-semibold uppercase tracking-[0.2em] shadow-[0_10px_26px_rgba(0,0,0,0.45)]"
              style={{
                backgroundColor: 'rgba(4, 10, 18, 0.97)',
                border: `1px solid ${color}66`,
                color: '#f8fafc',
              }}
            >
              <span className="h-2.5 w-2.5 rounded-full" style={{ backgroundColor: color }} />
              {typeof data?.label === 'string' ? data.label : 'Sequence yarn'}
            </span>
          </div>
        </foreignObject>
      </>
    );
  },
);

const nodeTypes = {
  chainFrame: ChainFrameNode,
  missionNode: MissionNode,
};

const edgeTypes = {
  sequenceThread: SequenceThreadEdge,
};

function CatalogCard({
  template,
  onAdd,
}: {
  template: ScenarioTemplate;
  onAdd: (template: ScenarioTemplate) => void;
}) {
  return (
    <button
      className="w-full rounded-[24px] border border-white/8 bg-[rgba(255,255,255,0.03)] p-4 text-left transition-colors hover:border-[rgba(245,170,49,0.3)] hover:bg-[rgba(245,170,49,0.08)]"
      onClick={() => onAdd(template)}
      type="button"
    >
      <div className="flex items-center justify-between gap-3">
        <span className="text-sm font-medium text-white">{template.title}</span>
        <span
          className="rounded-full px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.18em]"
          style={{
            backgroundColor: `${template.accent}22`,
            color: template.accent,
          }}
        >
          {template.stage}
        </span>
      </div>
      <p className="mt-2 text-sm leading-6 text-[rgba(224,231,239,0.76)]">{template.detail}</p>
      <div className="mt-3 flex items-center justify-between gap-3">
        <p className="text-xs uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
          {template.scenario}
        </p>
        <span
          className="rounded-full px-3 py-1 text-[10px] font-semibold uppercase tracking-[0.2em]"
          style={{
            backgroundColor: familyTint[template.family].chip,
            color: familyTint[template.family].text,
          }}
        >
          {familyLabel[template.family]}
        </span>
      </div>
    </button>
  );
}

function ToastStack({
  toasts,
}: {
  toasts: ToastItem[];
}) {
  return (
    <div className="pointer-events-none fixed right-6 top-6 z-[70] flex w-full max-w-md flex-col gap-4">
      {toasts.map((toast) => (
        <div
          className="rounded-[26px] border border-[rgba(245,170,49,0.24)] bg-[rgba(9,18,28,0.94)] px-5 py-4 shadow-[0_18px_48px_rgba(0,0,0,0.34)] backdrop-blur"
          key={toast.id}
        >
          <p className="text-sm font-semibold uppercase tracking-[0.2em] text-[rgba(245,170,49,0.88)]">
            Content Engine Update
          </p>
          <p className="mt-2 text-base leading-7 text-white">{toast.message}</p>
        </div>
      ))}
    </div>
  );
}

function InspectorPanelShell({
  eyebrow,
  title,
  badge,
  collapsed,
  dragging,
  onToggle,
  onDragStart,
  onDragEnd,
  onDragOver,
  onDrop,
  children,
}: {
  eyebrow: string;
  title: string;
  badge: ReactNode;
  collapsed: boolean;
  dragging: boolean;
  onToggle: () => void;
  onDragStart: (event: ReactDragEvent<HTMLButtonElement>) => void;
  onDragEnd: () => void;
  onDragOver: (event: ReactDragEvent<HTMLElement>) => void;
  onDrop: (event: ReactDragEvent<HTMLElement>) => void;
  children: ReactNode;
}) {
  return (
    <section
      className={`rounded-[32px] border bg-[rgba(9,18,28,0.8)] p-6 shadow-[0_24px_80px_rgba(0,0,0,0.22)] transition-colors ${
        dragging ? 'border-[rgba(245,170,49,0.32)]' : 'border-white/8'
      }`}
      onDragOver={onDragOver}
      onDrop={onDrop}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 items-start gap-3">
          <button
            aria-label={`Reorder ${title}`}
            className="mt-0.5 flex h-10 w-10 shrink-0 cursor-grab items-center justify-center rounded-[16px] border border-white/8 bg-white/4 text-sm font-semibold text-[rgba(224,231,239,0.72)] transition-colors hover:border-[rgba(245,170,49,0.28)] hover:bg-[rgba(245,170,49,0.08)] active:cursor-grabbing"
            draggable
            onDragEnd={onDragEnd}
            onDragStart={onDragStart}
            type="button"
          >
            ::
          </button>
          <div className="min-w-0">
            <p className="text-xs font-semibold uppercase tracking-[0.24em] text-[rgba(160,174,192,0.72)]">
              {eyebrow}
            </p>
            <h3 className="mt-2 truncate text-xl font-semibold tracking-[-0.04em] text-white">
              {title}
            </h3>
          </div>
        </div>

        <div className="flex items-start gap-2">
          {badge}
          <button
            aria-expanded={!collapsed}
            className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-white/8 bg-white/4 text-sm font-semibold text-[rgba(224,231,239,0.76)] transition-colors hover:border-[rgba(245,170,49,0.28)] hover:bg-[rgba(245,170,49,0.08)]"
            onClick={onToggle}
            type="button"
          >
            {collapsed ? 'v' : '^'}
          </button>
        </div>
      </div>

      {!collapsed ? <div className="mt-5 space-y-4">{children}</div> : null}
    </section>
  );
}

function FlowContent() {
  const [nodes, setNodes, onNodesChange] = useNodesState<EditorNodeData>(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState<SequenceEdgeData>(initialEdges);
  const [selectedSpaceId, setSelectedSpaceId] = useState<string>(spaceCatalog[0]?.id ?? '');
  const [selectedMissionId, setSelectedMissionId] = useState<string>('none');
  const [selectedNodeId, setSelectedNodeId] = useState<string>('1');
  const [selectedChainId, setSelectedChainId] = useState<string>('chain-arm-yourself');
  const [selectedSequenceId, setSelectedSequenceId] = useState<string>('primary-thread');
  const [edgeMode, setEdgeMode] = useState<'default' | 'sequenceThread'>('sequenceThread');
  const [draftSequence, setDraftSequence] = useState<SequenceEdgeData | null>(null);
  const [draggingNodeId, setDraggingNodeId] = useState<string>('');
  const [panelOrder, setPanelOrder] = useState<InspectorPanelId[]>(() => readInspectorPanelOrder());
  const [collapsedPanels, setCollapsedPanels] = useState<InspectorCollapsedState>(() =>
    readInspectorCollapsedState(),
  );
  const [draggingPanelId, setDraggingPanelId] = useState<InspectorPanelId | null>(null);
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const [savingDraft, setSavingDraft] = useState(false);
  const [draftStateBySpace, setDraftStateBySpace] = useState<Record<string, DraftPersistenceState>>(
    {},
  );
  const [autosaveRecoveryBySpace, setAutosaveRecoveryBySpace] = useState<
    Record<string, ChainEditorAutosave<EditorNodeData, SequenceEdgeData> | null>
  >({});
  const nextNodeId = useRef(200);
  const nextSequenceId = useRef(1);
  const nextToastId = useRef(1);
  const nodesRef = useRef(nodes);
  const edgesRef = useRef(edges);
  const undoHistoryRef = useRef<EditorSnapshot[]>([]);
  const redoHistoryRef = useRef<EditorSnapshot[]>([]);

  useEffect(() => {
    nodesRef.current = nodes;
  }, [nodes]);

  useEffect(() => {
    edgesRef.current = edges;
  }, [edges]);

  const pushToast = useCallback((message: string) => {
    const id = nextToastId.current++;
    setToasts((current) => [...current, { id, message }]);
    window.setTimeout(() => {
      setToasts((current) => current.filter((toast) => toast.id !== id));
    }, 6400);
  }, []);

  const recordHistorySnapshot = useCallback(() => {
    undoHistoryRef.current.push(
      cloneEditorSnapshot({
        edges: edgesRef.current,
        nodes: nodesRef.current,
      }),
    );
    redoHistoryRef.current = [];

    if (undoHistoryRef.current.length > 50) {
      undoHistoryRef.current.shift();
    }
  }, []);

  useEffect(() => {
    window.localStorage.setItem(inspectorPanelOrderStorageKey, JSON.stringify(panelOrder));
  }, [panelOrder]);

  useEffect(() => {
    window.localStorage.setItem(
      inspectorPanelCollapseStorageKey,
      JSON.stringify(collapsedPanels),
    );
  }, [collapsedPanels]);

  const nodesById = useMemo(
    () => new Map(nodes.map((node) => [node.id, node] as const)),
    [nodes],
  );

  const chainSummaries = useMemo(() => {
    const frameNodes = nodes.filter((node) => isChainData(node.data));

    return frameNodes.map((frameNode) => {
      const childNodes = nodes.filter(
        (node) => node.parentId === frameNode.id && isMissionData(node.data),
      );
      const childIds = childNodes.map((node) => node.id);
      const childSet = new Set(childIds);
      const counts: Record<PrimitiveFamily, number> = {
        anchor: 0,
        trigger: 0,
        condition: 0,
        action: 0,
        counter: 0,
      };
      const sequences = new Set<string>();

      for (const child of childNodes) {
        counts[child.data.family] += 1;
      }

      for (const edge of edges) {
        if (edge.type === 'sequenceThread' && edge.data) {
          if (childSet.has(edge.source) || childSet.has(edge.target)) {
            sequences.add(edge.data.sequenceId);
          }
        }
      }

      return {
        ...(frameNode.data as ChainFrameData),
        nodeId: frameNode.id,
        childIds,
        counts,
        edgeCount: sequences.size,
        sequenceCount: sequences.size,
      };
    });
  }, [edges, nodes]);

  const selectedNode = useMemo(() => {
    const found = nodesById.get(selectedNodeId);
    return found && isMissionData(found.data) ? found : undefined;
  }, [nodesById, selectedNodeId]);

  const selectedNodePorts = useMemo(
    () =>
      selectedNode ? getPortLabels(selectedNode.data) : { inputs: [], outputs: [] },
    [selectedNode],
  );

  const selectedNodeOutputConditions = useMemo(
    () => (selectedNode ? getOutputConditions(selectedNode.data) : []),
    [selectedNode],
  );

  const visibleChains = useMemo(
    () =>
      chainSummaries.filter(
        (chain) =>
          chain.spaceId === selectedSpaceId &&
          (selectedMissionId === 'none' || chain.missionId === selectedMissionId),
      ),
    [chainSummaries, selectedMissionId, selectedSpaceId],
  );

  const visibleChainIds = useMemo(
    () => new Set(visibleChains.map((chain) => chain.nodeId)),
    [visibleChains],
  );

  const selectedChain = useMemo(
    () => visibleChains.find((chain) => chain.nodeId === selectedChainId) ?? visibleChains[0],
    [selectedChainId, visibleChains],
  );

  const availableMissions = useMemo(
    () => missionCatalog.filter((mission) => mission.spaceId === selectedSpaceId),
    [selectedSpaceId],
  );

  const displayNodes = useMemo(() => {
    const anchorThreads = new Map<string, { color: string; label: string }>();
    const chainSummaryMap = new Map(chainSummaries.map((chain) => [chain.nodeId, chain] as const));
    const resolvedNodePositions = resolveAutoLayoutNodePositions(chainSummaries, nodes, edges, {
      cardColumnGap: AUTO_LAYOUT.cardColumnGap,
      cardHeight: AUTO_LAYOUT.cardHeight,
      cardWidth: AUTO_LAYOUT.cardWidth,
      frameLeftPadding: AUTO_LAYOUT.frameLeftPadding,
      frameTopPadding: AUTO_LAYOUT.frameTopPadding,
      orderedFamilies,
      rowGap: AUTO_LAYOUT.rowGap,
      rowLabelWidth: AUTO_LAYOUT.rowLabelWidth,
    });

    const layoutNodes = nodes.map((node) => {
      if (isChainData(node.data)) {
        return node;
      }

      return {
        ...node,
        position: resolvedNodePositions.get(node.id) ?? node.position,
      };
    });
    const layoutNodesById = new Map(layoutNodes.map((node) => [node.id, node] as const));
    const chainLayouts = computePackedChainLayouts(chainSummaries, layoutNodes, layoutNodesById, {
      cardHeight: AUTO_LAYOUT.cardHeight,
      cardWidth: AUTO_LAYOUT.cardWidth,
      frameBottomPadding: AUTO_LAYOUT.frameBottomPadding,
      frameRightPadding: AUTO_LAYOUT.frameRightPadding,
      minFrameHeight: 350,
      minFrameWidth: 1280,
      topPadding: 20,
      verticalGap: AUTO_LAYOUT.frameVerticalGap,
    });

    for (const edge of edges) {
      if (edge.type !== 'sequenceThread' || !edge.data) {
        continue;
      }

      for (const nodeId of [edge.source, edge.target]) {
        const node = nodesById.get(nodeId);
        if (!node || !isMissionData(node.data)) {
          continue;
        }

        if (node.data.family === 'anchor') {
          anchorThreads.set(nodeId, {
            color: edge.data.color,
            label: edge.data.label,
          });
        }
      }
    }

    return nodes.map((node) => {
      if (isChainData(node.data)) {
        const summary = chainSummaryMap.get(node.id);
        const layout = chainLayouts.get(node.id);
        return {
          ...node,
          draggable: false,
          position: layout?.framePosition ?? node.position,
          style: {
            ...node.style,
            ...layout?.frameStyle,
          },
          data: {
            ...node.data,
            counts: summary?.counts,
            sequenceCount: summary?.sequenceCount,
          },
        };
      }

      const thread = anchorThreads.get(node.id);
      return {
        ...node,
        draggable: true,
        position: resolvedNodePositions.get(node.id) ?? node.position,
        data: {
          ...node.data,
          threadColor: thread?.color,
          threadName: thread?.label,
        },
      };
    });
  }, [chainSummaries, edges, nodes, nodesById]);

  const visibleNodes = useMemo(
    () =>
      displayNodes.filter((node) =>
        isChainData(node.data)
          ? visibleChainIds.has(node.id)
          : !!node.parentId && visibleChainIds.has(node.parentId),
      ),
    [displayNodes, visibleChainIds],
  );

  const visibleNodeIds = useMemo(
    () => new Set(visibleNodes.map((node) => node.id)),
    [visibleNodes],
  );

  const visibleEdges = useMemo(
    () =>
      edges
        .filter((edge) => visibleNodeIds.has(edge.source) && visibleNodeIds.has(edge.target))
        .map((edge) => {
          if (edge.type !== 'sequenceThread' || !edge.data) {
            return {
              ...edge,
              zIndex: 12,
            };
          }

          const sourceNode = nodesById.get(edge.source);
          if (!sourceNode || !isMissionData(sourceNode.data)) {
            return edge;
          }

          const ruleLabel = getOutputRuleText(sourceNode.data, edge.sourceHandle);
          if (!ruleLabel) {
            return edge;
          }

          return {
            ...edge,
            zIndex: 20,
            data: {
              ...edge.data,
              label: ruleLabel,
            },
          };
        }),
    [edges, nodesById, visibleNodeIds],
  );

  const sequences = useMemo(() => {
    const map = new Map<string, SequenceEdgeData & { edgeCount: number }>();

    for (const edge of visibleEdges) {
      if (edge.type !== 'sequenceThread' || !edge.data) {
        continue;
      }

      const existing = map.get(edge.data.sequenceId);
      if (existing) {
        existing.edgeCount += 1;
      } else {
        map.set(edge.data.sequenceId, { ...edge.data, edgeCount: 1 });
      }
    }

    const values = Array.from(map.values());
    return draftSequence ? [{ ...draftSequence, edgeCount: 0 }, ...values] : values;
  }, [draftSequence, visibleEdges]);

  const selectedSequence = useMemo(
    () => sequences.find((sequence) => sequence.sequenceId === selectedSequenceId) ?? sequences[0],
    [selectedSequenceId, sequences],
  );

  const selectedChainName = selectedChain?.name ?? 'Untitled chain';
  const selectedCardTitle = selectedNode?.data.title ?? 'Selected card';
  const activeSpaceDraft = useMemo(() => {
    const scopedSnapshot = extractSpaceScopedEditorSnapshot(nodes, edges, selectedSpaceId);

    return createPersistedChainEditorDraft<EditorNodeData, SequenceEdgeData>({
      edges: scopedSnapshot.edges,
      missionId: null,
      nodes: scopedSnapshot.nodes,
      selectedChainId,
      selectedNodeId,
      selectedSequenceId,
      spaceId: selectedSpaceId,
    });
  }, [edges, nodes, selectedChainId, selectedNodeId, selectedSequenceId, selectedSpaceId]);

  const activeSpaceDraftSignature = useMemo(
    () => JSON.stringify(activeSpaceDraft),
    [activeSpaceDraft],
  );
  const activeDraftState = draftStateBySpace[selectedSpaceId];
  const isDraftDirty = !!activeDraftState?.hydrated &&
    activeDraftState.savedSignature !== activeSpaceDraftSignature;
  const activeAutosaveRecovery = autosaveRecoveryBySpace[selectedSpaceId] ?? null;

  const beginDraftSequence = useCallback(() => {
    const id = `sequence-${nextSequenceId.current++}`;
    const nextDraft = {
      sequenceId: id,
      label: `New Sequence ${nextSequenceId.current - 1}`,
      category: 'Debug',
      color: '#94a3b8',
    };

    setDraftSequence(nextDraft);
    setSelectedSequenceId(id);
    setEdgeMode('sequenceThread');
    pushToast(`Created ${nextDraft.label} in ${selectedChainName}.`);
  }, [pushToast, selectedChainName]);

  const handleSaveDraft = useCallback(async () => {
    setSavingDraft(true);

    try {
      const result = await saveChainEditorDraft(activeSpaceDraft);
      const savedAt = new Date().toISOString();
      setDraftStateBySpace((current) => ({
        ...current,
        [selectedSpaceId]: {
          hydrated: true,
          lastSavedAt: savedAt,
          persistedDraft: activeSpaceDraft,
          recoveredFromAutosave: false,
          savedSignature: activeSpaceDraftSignature,
          storage: result.storage,
        },
      }));
      clearChainEditorAutosave(selectedSpaceId, null);
      setAutosaveRecoveryBySpace((current) => ({
        ...current,
        [selectedSpaceId]: null,
      }));

      if (result.storage === 'database') {
        pushToast(`Saved ${spaceCatalog.find((space) => space.id === selectedSpaceId)?.label ?? selectedSpaceId} draft to PostgreSQL.`);
      } else if (result.warning) {
        pushToast(`Saved ${selectedSpaceId} draft locally because database save failed: ${result.warning}.`);
      } else {
        pushToast(`Saved ${selectedSpaceId} draft to the browser fallback store.`);
      }
    } catch (error) {
      pushToast(`Error: ${error instanceof Error ? error.message : 'could not save the current draft'}.`);
    } finally {
      setSavingDraft(false);
    }
  }, [activeSpaceDraft, activeSpaceDraftSignature, pushToast, selectedSpaceId]);

  const handleRevertDraft = useCallback(() => {
    const persistedDraft = activeDraftState?.persistedDraft;
    if (!persistedDraft) {
      pushToast('Error: no saved draft is available to revert.');
      return;
    }

    recordHistorySnapshot();
    const mergedSnapshot = mergeSpaceScopedEditorSnapshot(
      nodesRef.current,
      edgesRef.current,
      selectedSpaceId,
      persistedDraft,
    );
    setNodes(mergedSnapshot.nodes);
    setEdges(mergedSnapshot.edges);
    setSelectedChainId(persistedDraft.selectedChainId || '');
    setSelectedNodeId(persistedDraft.selectedNodeId || '');
    setSelectedSequenceId(persistedDraft.selectedSequenceId || '');
    redoHistoryRef.current = [];
    clearChainEditorAutosave(selectedSpaceId, null);
    setAutosaveRecoveryBySpace((current) => ({
      ...current,
      [selectedSpaceId]: null,
    }));
    setDraftStateBySpace((current) => ({
      ...current,
      [selectedSpaceId]: current[selectedSpaceId]
        ? {
            ...current[selectedSpaceId],
            recoveredFromAutosave: false,
          }
        : current[selectedSpaceId],
    }));
    pushToast(`Reverted ${selectedSpaceId} to the last saved draft.`);
  }, [activeDraftState, pushToast, recordHistorySnapshot, selectedSpaceId, setEdges, setNodes]);

  const handleRecoverAutosave = useCallback(() => {
    if (!activeAutosaveRecovery) {
      pushToast('Error: no autosave recovery snapshot is available.');
      return;
    }

    recordHistorySnapshot();
    const mergedSnapshot = mergeSpaceScopedEditorSnapshot(
      nodesRef.current,
      edgesRef.current,
      selectedSpaceId,
      activeAutosaveRecovery.draft,
    );
    setNodes(mergedSnapshot.nodes);
    setEdges(mergedSnapshot.edges);
    setSelectedChainId(activeAutosaveRecovery.draft.selectedChainId || '');
    setSelectedNodeId(activeAutosaveRecovery.draft.selectedNodeId || '');
    setSelectedSequenceId(activeAutosaveRecovery.draft.selectedSequenceId || '');
    setAutosaveRecoveryBySpace((current) => ({
      ...current,
      [selectedSpaceId]: null,
    }));
    setDraftStateBySpace((current) => ({
      ...current,
      [selectedSpaceId]: current[selectedSpaceId]
        ? {
            ...current[selectedSpaceId],
            recoveredFromAutosave: true,
          }
        : current[selectedSpaceId],
    }));
    pushToast(`Recovered autosave from ${formatTimestampLabel(activeAutosaveRecovery.autosavedAt)}.`);
  }, [activeAutosaveRecovery, pushToast, recordHistorySnapshot, selectedSpaceId, setEdges, setNodes]);

  const dismissAutosaveRecovery = useCallback(() => {
    clearChainEditorAutosave(selectedSpaceId, null);
    setAutosaveRecoveryBySpace((current) => ({
      ...current,
      [selectedSpaceId]: null,
    }));
    pushToast(`Dismissed autosave recovery for ${selectedSpaceId}.`);
  }, [pushToast, selectedSpaceId]);

  useEffect(() => {
    if (!activeDraftState?.hydrated || !isDraftDirty) {
      return;
    }

    const timeoutId = window.setTimeout(() => {
      const autosave = createChainEditorAutosave({
        autosavedAt: new Date().toISOString(),
        draft: activeSpaceDraft,
        savedSignature: activeDraftState.savedSignature,
      });
      saveChainEditorAutosave(autosave);
    }, 1200);

    return () => window.clearTimeout(timeoutId);
  }, [activeDraftState, activeSpaceDraft, isDraftDirty]);

  useEffect(() => {
    let cancelled = false;

    async function hydrateSpaceDraft() {
      const seedSnapshot = extractSpaceScopedEditorSnapshot(
        nodesRef.current,
        edgesRef.current,
        selectedSpaceId,
      );
      const seedDraft = createPersistedChainEditorDraft<EditorNodeData, SequenceEdgeData>({
        edges: seedSnapshot.edges,
        missionId: null,
        nodes: seedSnapshot.nodes,
        selectedChainId,
        selectedNodeId,
        selectedSequenceId,
        spaceId: selectedSpaceId,
      });

      const result = await loadChainEditorDraft<EditorNodeData, SequenceEdgeData>(
        selectedSpaceId,
        null,
      );
      if (cancelled) {
        return;
      }

      if (result.draft) {
        const mergedSnapshot = mergeSpaceScopedEditorSnapshot(
          nodesRef.current,
          edgesRef.current,
          selectedSpaceId,
          result.draft,
        );

        setNodes(mergedSnapshot.nodes);
        setEdges(mergedSnapshot.edges);
        setSelectedChainId(result.draft.selectedChainId || '');
        setSelectedNodeId(result.draft.selectedNodeId || '');
        setSelectedSequenceId(result.draft.selectedSequenceId || '');
        setDraftStateBySpace((current) => ({
          ...current,
          [selectedSpaceId]: {
            hydrated: true,
            persistedDraft: result.draft,
            recoveredFromAutosave: false,
            savedSignature: JSON.stringify(result.draft),
            storage: result.storage,
          },
        }));
        const autosave = loadChainEditorAutosave<EditorNodeData, SequenceEdgeData>(
          selectedSpaceId,
          null,
        );
        setAutosaveRecoveryBySpace((current) => ({
          ...current,
          [selectedSpaceId]: shouldRecoverAutosave(
            autosave,
            JSON.stringify(result.draft),
          )
            ? autosave
            : null,
        }));

        if (result.warning) {
          pushToast(
            `Loaded ${selectedSpaceId} from browser fallback because database load failed: ${result.warning}.`,
          );
        } else if (result.storage !== 'seed') {
          pushToast(
            `Loaded ${selectedSpaceId} draft from ${result.storage === 'database' ? 'PostgreSQL' : 'browser storage'}.`,
          );
        }
        if (shouldRecoverAutosave(autosave, JSON.stringify(result.draft))) {
          pushToast(`Autosave recovery is available for ${selectedSpaceId}.`);
        }
        return;
      }

      setDraftStateBySpace((current) => ({
        ...current,
        [selectedSpaceId]: {
          hydrated: true,
          persistedDraft: seedDraft,
          recoveredFromAutosave: false,
          savedSignature: JSON.stringify(seedDraft),
          storage: result.storage,
        },
      }));
      const autosave = loadChainEditorAutosave<EditorNodeData, SequenceEdgeData>(
        selectedSpaceId,
        null,
      );
      setAutosaveRecoveryBySpace((current) => ({
        ...current,
        [selectedSpaceId]: shouldRecoverAutosave(
          autosave,
          JSON.stringify(seedDraft),
        )
          ? autosave
          : null,
      }));

      if (result.warning) {
        pushToast(
          `Using seeded ${selectedSpaceId} content because draft load failed: ${result.warning}.`,
        );
      }
      if (shouldRecoverAutosave(autosave, JSON.stringify(seedDraft))) {
        pushToast(`Autosave recovery is available for ${selectedSpaceId}.`);
      }
    }

    hydrateSpaceDraft();
    return () => {
      cancelled = true;
    };
  }, [pushToast, selectedSpaceId, setEdges, setNodes]);

  const handleConnect = useCallback<OnConnect>(
    (connection) => {
      try {
        const sourceNode = connection.source ? nodesById.get(connection.source) : undefined;
        const targetNode = connection.target ? nodesById.get(connection.target) : undefined;

        if (!connection.source || !connection.target) {
          throw new Error('missing a source or target handle');
        }

        if (!sourceNode || !targetNode || !isMissionData(sourceNode.data) || !isMissionData(targetNode.data)) {
          throw new Error('both endpoints must be mission cards');
        }

        if (connection.source === connection.target) {
          throw new Error('a card cannot connect to itself');
        }

        validateConnectionRequest({
          sourceHandle: connection.sourceHandle,
          sourceId: connection.source,
          sourceNode,
          targetHandle: connection.targetHandle,
          targetId: connection.target,
          targetNode,
        });

        recordHistorySnapshot();

        const branchLabel = getOutputRuleText(sourceNode.data, connection.sourceHandle);
        const sourceTitle = sourceNode.data.title;
        const targetTitle = targetNode.data.title;

        if (edgeMode === 'sequenceThread') {
          const activeSequence =
            draftSequence ??
            edges.find(
              (edge) =>
                edge.type === 'sequenceThread' && edge.data?.sequenceId === selectedSequenceId,
            )?.data ?? {
              sequenceId: `sequence-${nextSequenceId.current++}`,
              label: `New Sequence ${nextSequenceId.current - 1}`,
              category: 'Debug',
              color: '#94a3b8',
            };

          setEdges((current) =>
            addEdge(
              {
                ...connection,
                type: 'sequenceThread',
                data: activeSequence,
                label: branchLabel || undefined,
                zIndex: 20,
              },
              current,
            ),
          );
          pushToast(`Connected ${sourceTitle} to ${targetTitle} in ${activeSequence.label}.`);
        } else {
          setEdges((current) =>
            addEdge(
              {
                ...connection,
                animated: true,
                style: {
                  stroke: '#5eb8b3',
                  strokeDasharray: '8 8',
                  strokeWidth: 2,
                },
                type: 'smoothstep',
                zIndex: 12,
              },
              current,
            ),
          );
          pushToast(`Linked ${sourceTitle} to ${targetTitle} across ${selectedChainName}.`);
        }

        if (edgeMode === 'sequenceThread' && draftSequence) {
          setDraftSequence(null);
        }
      } catch (error) {
        pushToast(
          `Error: ${error instanceof Error ? error.message : 'could not create the requested link'}.`,
        );
      }
    },
    [
      draftSequence,
      edgeMode,
      edges,
      nodesById,
      pushToast,
      recordHistorySnapshot,
      selectedChainName,
      selectedSequenceId,
      setEdges,
    ],
  );

  const handleNodesChange = useCallback<OnNodesChange>(
    (changes) => {
      onNodesChange(changes);

      for (const change of changes) {
        if (change.type !== 'select' || !change.selected) {
          continue;
        }

        const found = nodesRef.current.find((node) => node.id === change.id);
        if (!found) {
          continue;
        }

        if (isChainData(found.data)) {
          setSelectedChainId(found.id);
          continue;
        }

        setSelectedNodeId(found.id);
        if (found.parentId) {
          setSelectedChainId(found.parentId);
        }
      }
    },
    [onNodesChange],
  );

  const handleEdgesChange = useCallback<OnEdgesChange>(
    (changes) => onEdgesChange(changes),
    [onEdgesChange],
  );

  const handleNodeDragStop = useCallback(
    (_event: unknown, node: Node<EditorNodeData>) => {
      setDraggingNodeId('');

      if (!node.parentId || !isMissionData(node.data)) {
        return;
      }

      recordHistorySnapshot();

      const clampedPosition = {
        x: Math.max(AUTO_LAYOUT.frameLeftPadding + AUTO_LAYOUT.rowLabelWidth, node.position.x),
        y: Math.max(AUTO_LAYOUT.frameTopPadding, node.position.y),
      };

      const siblingNodes = displayNodes
        .filter(
          (entry) =>
            entry.parentId === node.parentId &&
            entry.id !== node.id &&
            isMissionData(entry.data),
        )
        .sort((a, b) => a.position.x - b.position.x);

      let insertAt = siblingNodes.length;
      for (let index = 0; index < siblingNodes.length; index += 1) {
        if (node.position.x < siblingNodes[index].position.x) {
          insertAt = index;
          break;
        }
      }

      const nextOrder = [...siblingNodes];
      nextOrder.splice(insertAt, 0, node);

      setNodes((current) =>
        current.map((entry) => {
          if (entry.parentId !== node.parentId || !isMissionData(entry.data)) {
            return entry;
          }

          const orderIndex = nextOrder.findIndex((orderedNode) => orderedNode.id === entry.id);
          if (orderIndex === -1) {
            return entry;
          }

          return {
            ...entry,
            position: entry.id === node.id ? clampedPosition : entry.position,
            data: {
              ...entry.data,
              manualPosition: entry.id === node.id ? true : entry.data.manualPosition,
              sortOrder: orderIndex,
            },
          };
        }),
      );
      pushToast(`Reordered ${node.data.title} inside ${selectedChainName}.`);
    },
    [displayNodes, pushToast, recordHistorySnapshot, selectedChainName, setNodes],
  );

  const updateSelectedNode = useCallback(
    (patch: Partial<MissionNodeData>) => {
      if (!selectedNode) {
        return;
      }

      setNodes((current) =>
        current.map((node) =>
          node.id === selectedNode.id && isMissionData(node.data)
            ? {
                ...node,
                data: {
                  ...node.data,
                  ...patch,
                },
              }
            : node,
        ),
      );
    },
    [selectedNode, setNodes],
  );

  const updateSelectedChain = useCallback(
    (patch: Partial<ChainFrameData>) => {
      if (!selectedChainId) {
        return;
      }

      setNodes((current) =>
        current.map((node) =>
          node.id === selectedChainId && isChainData(node.data)
            ? {
                ...node,
                data: {
                  ...node.data,
                  ...patch,
                },
              }
            : node,
        ),
      );
    },
    [selectedChainId, setNodes],
  );

  const updateSelectedSequence = useCallback(
    (patch: Partial<SequenceEdgeData>) => {
      if (!selectedSequenceId) {
        return;
      }

      if (draftSequence && draftSequence.sequenceId === selectedSequenceId) {
        setDraftSequence((current) =>
          current ? { ...current, ...patch } : current,
        );
      }

      setEdges((current) =>
        current.map((edge) =>
          edge.type === 'sequenceThread' && edge.data?.sequenceId === selectedSequenceId
            ? {
                ...edge,
                data: {
                  ...edge.data,
                  ...patch,
                },
              }
            : edge,
        ),
      );
    },
    [draftSequence, selectedSequenceId, setEdges],
  );

  const deleteSelectedNode = useCallback(() => {
    if (!selectedNodeId) {
      return;
    }

    const node = nodesById.get(selectedNodeId);
    if (!node || !isMissionData(node.data)) {
      return;
    }

    recordHistorySnapshot();
    setNodes((current) => current.filter((entry) => entry.id !== selectedNodeId));
    setEdges((current) =>
      current.filter((edge) => edge.source !== selectedNodeId && edge.target !== selectedNodeId),
    );
    setSelectedNodeId('');
    pushToast(`Deleted ${node.data.title} from ${selectedChainName}.`);
  }, [nodesById, pushToast, recordHistorySnapshot, selectedChainName, selectedNodeId, setEdges, setNodes]);

  const applySequenceStyle = useCallback(
    (style: SequenceStyle) => {
      recordHistorySnapshot();
      updateSelectedSequence({
        category: style.label,
        color: style.color,
      });
      pushToast(`Updated ${selectedSequence?.label ?? 'sequence yarn'} to ${style.label} style.`);
    },
    [pushToast, recordHistorySnapshot, selectedSequence, updateSelectedSequence],
  );

  const applyChainTint = useCallback(
    (style: SequenceStyle) => {
      recordHistorySnapshot();
      updateSelectedChain({
        color: style.color,
        semantic: style.label,
      });
      pushToast(`Updated ${selectedChainName} tint to ${style.label}.`);
    },
    [pushToast, recordHistorySnapshot, selectedChainName, updateSelectedChain],
  );

  const togglePanelCollapsed = useCallback((panelId: InspectorPanelId) => {
    setCollapsedPanels((current) => ({
      ...current,
      [panelId]: !current[panelId],
    }));
  }, []);

  const handlePanelDragStart = useCallback(
    (panelId: InspectorPanelId) => (event: ReactDragEvent<HTMLButtonElement>) => {
      setDraggingPanelId(panelId);
      event.dataTransfer.effectAllowed = 'move';
      event.dataTransfer.setData('text/inspector-panel', panelId);
    },
    [],
  );

  const handlePanelDragOver = useCallback(
    (event: ReactDragEvent<HTMLElement>) => {
      if (!draggingPanelId) {
        return;
      }

      event.preventDefault();
      event.dataTransfer.dropEffect = 'move';
    },
    [draggingPanelId],
  );

  const handlePanelDrop = useCallback(
    (targetId: InspectorPanelId) => (event: ReactDragEvent<HTMLElement>) => {
      event.preventDefault();
      const droppedPanelId =
        draggingPanelId ?? event.dataTransfer.getData('text/inspector-panel');

      if (!isInspectorPanelId(droppedPanelId) || droppedPanelId === targetId) {
        setDraggingPanelId(null);
        return;
      }

      setPanelOrder((current) => reorderInspectorPanels(current, droppedPanelId, targetId));
      setDraggingPanelId(null);
    },
    [draggingPanelId],
  );

  const handlePanelDragEnd = useCallback(() => {
    setDraggingPanelId(null);
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const tagName = target?.tagName?.toLowerCase();
      const isEditingField =
        tagName === 'input' ||
        tagName === 'textarea' ||
        tagName === 'select' ||
        target?.isContentEditable;

      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'z' && !isEditingField) {
        const currentSnapshot = cloneEditorSnapshot({
          edges: edgesRef.current,
          nodes: nodesRef.current,
        });
        const previousSnapshot = undoHistoryRef.current.pop();
        if (!previousSnapshot) {
          pushToast('Error: nothing to undo.');
          return;
        }

        event.preventDefault();
        redoHistoryRef.current.push(currentSnapshot);
        setNodes(previousSnapshot.nodes);
        setEdges(previousSnapshot.edges);
        pushToast('Undid the last editor change.');
        return;
      }

      const wantsRedo =
        (event.ctrlKey && event.key.toLowerCase() === 'y') ||
        ((event.metaKey || event.ctrlKey) &&
          event.shiftKey &&
          event.key.toLowerCase() === 'z');

      if (wantsRedo && !isEditingField) {
        const nextSnapshot = redoHistoryRef.current.pop();
        if (!nextSnapshot) {
          pushToast('Error: nothing to redo.');
          return;
        }

        event.preventDefault();
        undoHistoryRef.current.push(
          cloneEditorSnapshot({
            edges: edgesRef.current,
            nodes: nodesRef.current,
          }),
        );
        setNodes(nextSnapshot.nodes);
        setEdges(nextSnapshot.edges);
        pushToast('Redid the last editor change.');
        return;
      }

      if (event.key !== 'Delete' && event.key !== 'Backspace') {
        return;
      }

      if (isEditingField || !selectedNodeId) {
        return;
      }

      event.preventDefault();
      deleteSelectedNode();
    };

    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [deleteSelectedNode, pushToast, selectedNodeId, setEdges, setNodes]);

  const addScenarioNode = useCallback(
    (template: LibraryScenarioTemplate) => {
      const targetChain = nodesById.get(selectedChainId);
      if (!targetChain || !isChainData(targetChain.data)) {
        pushToast('Error: select a target content chain before adding a card.');
        return;
      }
      const id = `n${nextNodeId.current++}`;
      const siblingNodes = nodes.filter(
        (node) => node.parentId === selectedChainId && isMissionData(node.data),
      );
      const nextSortOrder = siblingNodes.reduce(
        (maxOrder, node) => Math.max(maxOrder, node.data.sortOrder ?? -1),
        -1,
      ) + 1;
      const familyNodes = siblingNodes.filter((node) => node.data.family === template.family);
      const activeFamilies = Array.from(
        new Set([...siblingNodes.map((node) => node.data.family), template.family]),
      ).sort((a, b) => orderedFamilies.indexOf(a) - orderedFamilies.indexOf(b));
      const fallbackPosition = getLanePosition(template.family, nextSortOrder, activeFamilies);
      const nextPosition = familyNodes.length
        ? {
            x:
              Math.max(...familyNodes.map((node) => node.position.x)) +
              AUTO_LAYOUT.cardWidth +
              Math.floor(AUTO_LAYOUT.cardColumnGap / 2),
            y: familyNodes[0]?.position.y ?? fallbackPosition.y,
          }
        : fallbackPosition;

      recordHistorySnapshot();
      setNodes((current) => [
        ...current,
        {
          id,
          parentId: selectedChainId,
          extent: 'parent',
          position: nextPosition,
          type: 'missionNode',
          data: {
            kind: 'mission',
            family: template.family,
            stage: template.stage,
            title: template.title,
            detail: template.detail,
            status: template.status,
            scenario: template.scenario,
            accent: template.accent,
            manualPosition: false,
            sortOrder: nextSortOrder,
            inputs:
              template.family === 'anchor' && template.stage === 'Start'
                ? []
                : ['In'],
            outputs:
              template.family === 'anchor' && template.stage === 'End'
                ? []
                : ['Out'],
            outputConditions: template.family === 'anchor' && template.stage === 'End' ? [] : [''],
            properties: template.properties.map((item) => ({ ...item })),
          },
        },
      ]);
      setSelectedNodeId(id);
      pushToast(`Added ${template.title} to ${targetChain.data.name}.`);
    },
    [nodes, nodesById, pushToast, recordHistorySnapshot, selectedChainId, setNodes],
  );

  useEffect(() => {
    const validMissionNodes = visibleNodes.filter((node) => isMissionData(node.data));

    if (!validMissionNodes.length) {
      setSelectedNodeId('');
      return;
    }

    if (selectedNodeId && validMissionNodes.some((node) => node.id === selectedNodeId)) {
      return;
    }

    setSelectedNodeId(validMissionNodes[0].id);
  }, [selectedNodeId, visibleNodes]);

  useEffect(() => {
    if (!visibleChains.length) {
      setSelectedChainId('');
      return;
    }

    if (selectedChainId && visibleChains.some((chain) => chain.nodeId === selectedChainId)) {
      return;
    }

    setSelectedChainId(visibleChains[0].nodeId);
  }, [selectedChainId, visibleChains]);

  useEffect(() => {
    if (selectedMissionId === 'none') {
      return;
    }

    if (availableMissions.some((mission) => mission.id === selectedMissionId)) {
      return;
    }

    setSelectedMissionId('none');
  }, [availableMissions, selectedMissionId]);

  useEffect(() => {
    if (!sequences.length) {
      setSelectedSequenceId('');
      return;
    }

    if (selectedSequenceId && sequences.some((sequence) => sequence.sequenceId === selectedSequenceId)) {
      return;
    }

    setSelectedSequenceId(sequences[0].sequenceId);
  }, [selectedSequenceId, sequences]);

  const inspectorPanels: Record<
    InspectorPanelId,
    {
      eyebrow: string;
      title: string;
      badge: ReactNode;
      content: ReactNode;
    }
  > = {
    chain: {
      eyebrow: 'Selected Chain',
      title: selectedChain?.name ?? 'No chain selected',
      badge: (
        <span
          className="rounded-full px-3 py-1 text-xs font-medium"
          style={{
            backgroundColor: `${selectedChain?.color ?? '#94a3b8'}22`,
            color: selectedChain?.color ?? '#cbd5e1',
          }}
        >
          {selectedChain?.semantic ?? 'Untinted'}
        </span>
      ),
      content: (
        <>
          <label className="block">
            <span className="mb-2 block text-xs font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
              Chain name
            </span>
            <input
              className="w-full rounded-[20px] border border-white/8 bg-white/4 px-4 py-3 text-sm text-white outline-none transition-colors focus:border-[#f5aa31]"
              onChange={(event) => updateSelectedChain({ name: event.currentTarget.value })}
              onBlur={() => pushToast(`Updated chain name to ${selectedChain?.name ?? 'Untitled chain'}.`)}
              value={selectedChain?.name ?? ''}
            />
          </label>

          <label className="block">
            <span className="mb-2 block text-xs font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
              Chain summary
            </span>
            <textarea
              className="min-h-28 w-full rounded-[24px] border border-white/8 bg-white/4 px-4 py-3 text-sm leading-6 text-white outline-none transition-colors focus:border-[#f5aa31]"
              onChange={(event) => updateSelectedChain({ summary: event.currentTarget.value })}
              onBlur={() => pushToast(`Updated summary for ${selectedChain?.name ?? 'Untitled chain'}.`)}
              value={selectedChain?.summary ?? ''}
            />
          </label>

          <div className="grid gap-3 md:grid-cols-2">
            <label className="block">
              <span className="mb-2 block text-xs font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
                Space
              </span>
              <select
                className="w-full rounded-[20px] border border-white/8 bg-[rgba(11,19,29,0.96)] px-4 py-3 text-sm text-white outline-none transition-colors focus:border-[#f5aa31]"
                onChange={(event) => {
                  setSelectedSpaceId(event.currentTarget.value);
                  setSelectedMissionId('none');
                }}
                value={selectedSpaceId}
              >
                {spaceCatalog.map((space) => (
                  <option key={space.id} value={space.id}>
                    {space.label}
                  </option>
                ))}
              </select>
            </label>

            <label className="block">
              <span className="mb-2 block text-xs font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
                Mission
              </span>
              <select
                className="w-full rounded-[20px] border border-white/8 bg-[rgba(11,19,29,0.96)] px-4 py-3 text-sm text-white outline-none transition-colors focus:border-[#f5aa31]"
                onChange={(event) => setSelectedMissionId(event.currentTarget.value)}
                value={selectedMissionId}
              >
                <option value="none">None (space view)</option>
                {availableMissions.map((mission) => (
                  <option key={mission.id} value={mission.id}>
                    {mission.label}
                  </option>
                ))}
              </select>
            </label>
          </div>

          <div className="rounded-[20px] border border-white/8 bg-white/4 px-4 py-3 text-sm text-[rgba(224,231,239,0.76)]">
            Content chains are treated as mission-scoped for now. Effect scope is intentionally omitted and tracked as a backlog item.
          </div>

          <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_auto]">
            <label className="block">
              <span className="mb-2 block text-xs font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
                Priority
              </span>
              <input
                className="w-full rounded-[20px] border border-white/8 bg-white/4 px-4 py-3 text-sm text-white outline-none transition-colors focus:border-[#f5aa31]"
                onChange={(event) =>
                  updateSelectedChain({
                    priority: Number(event.currentTarget.value) || 0,
                  })
                }
                onBlur={() => pushToast(`Updated priority for ${selectedChain?.name ?? 'Untitled chain'}.`)}
                type="number"
                value={selectedChain?.priority ?? 0}
              />
            </label>

            <button
              className={`mt-7 rounded-[20px] px-4 py-3 text-sm font-medium transition-colors ${
                selectedChain?.enabled
                  ? 'border border-[#22c55e]/30 bg-[rgba(34,197,94,0.14)] text-[#c7ffd5]'
                  : 'border border-white/8 bg-white/4 text-[rgba(224,231,239,0.76)]'
              }`}
              onClick={() => {
                updateSelectedChain({ enabled: !selectedChain?.enabled });
                pushToast(
                  `${selectedChain?.enabled ? 'Disabled' : 'Enabled'} ${selectedChain?.name ?? 'Untitled chain'}.`,
                );
              }}
              type="button"
            >
              {selectedChain?.enabled ? 'Enabled' : 'Disabled'}
            </button>
          </div>

          <div className="rounded-[24px] border border-dashed border-white/12 bg-[rgba(255,255,255,0.03)] p-4">
            <p className="text-xs font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
              Chain frame tint
            </p>
            <div className="mt-3 flex items-center gap-3">
              <div
                className="h-4 w-20 rounded-full"
                style={{ backgroundColor: selectedChain?.color ?? '#94a3b8' }}
              />
              <p className="text-sm text-[rgba(224,231,239,0.76)]">
                Use semantic tints to make space chains, mission hand-offs, and hidden controllers readable on the canvas.
              </p>
            </div>
            <div className="mt-4 grid gap-2 sm:grid-cols-2">
              {sequencePalette.map((style) => (
                <button
                  className="rounded-[18px] border border-white/8 bg-white/4 px-3 py-3 text-left transition-colors hover:border-[rgba(245,170,49,0.28)] hover:bg-[rgba(245,170,49,0.08)]"
                  key={`chain-style-${style.id}`}
                  onClick={() => applyChainTint(style)}
                  type="button"
                >
                  <div className="flex items-center justify-between gap-3">
                    <span className="text-sm font-medium text-white">{style.label}</span>
                    <span
                      className="h-3.5 w-3.5 rounded-full"
                      style={{ backgroundColor: style.color }}
                    />
                  </div>
                </button>
              ))}
            </div>
          </div>

          <div className="rounded-[24px] border border-white/8 bg-[rgba(255,255,255,0.03)] p-4">
            <p className="text-xs font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
              Chain inventory
            </p>
            <div className="mt-3 space-y-2">
              {visibleChains.map((chain) => (
                <button
                  className={`flex w-full items-center justify-between rounded-[18px] px-3 py-3 text-left transition-colors ${
                    chain.nodeId === selectedChain?.nodeId ? 'bg-white/8' : 'bg-transparent hover:bg-white/4'
                  }`}
                  key={chain.nodeId}
                  onClick={() => setSelectedChainId(chain.nodeId)}
                  type="button"
                >
                  <div className="flex items-center gap-3">
                    <span className="h-3 w-3 rounded-full" style={{ backgroundColor: chain.color }} />
                    <div>
                      <p className="text-sm font-medium text-white">{chain.name}</p>
                      <p className="text-xs uppercase tracking-[0.18em] text-[rgba(160,174,192,0.72)]">
                        {chain.scopeType} / {chain.scopeId}
                      </p>
                    </div>
                  </div>
                  <span className="text-xs text-[rgba(224,231,239,0.72)]">
                    {chain.sequenceCount} yarn{chain.sequenceCount === 1 ? '' : 's'}
                  </span>
                </button>
              ))}
            </div>
          </div>
        </>
      ),
    },
    sequence: {
      eyebrow: 'Selected Sequence',
      title: selectedSequence?.label ?? 'No sequence selected',
      badge: (
        <span
          className="rounded-full px-3 py-1 text-xs font-medium"
          style={{
            backgroundColor: `${selectedSequence?.color ?? '#94a3b8'}22`,
            color: selectedSequence?.color ?? '#cbd5e1',
          }}
        >
          {selectedSequence?.category ?? 'Unstyled'}
        </span>
      ),
      content: (
        <>
          <label className="block">
            <span className="mb-2 block text-xs font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
              Sequence name
            </span>
            <input
              className="w-full rounded-[20px] border border-white/8 bg-white/4 px-4 py-3 text-sm text-white outline-none transition-colors focus:border-[#f5aa31]"
              onChange={(event) => updateSelectedSequence({ label: event.currentTarget.value })}
              onBlur={() => pushToast(`Updated sequence name to ${selectedSequence?.label ?? 'Untitled sequence'}.`)}
              value={selectedSequence?.label ?? ''}
            />
          </label>

          <div className="rounded-[24px] border border-dashed border-white/12 bg-[rgba(255,255,255,0.03)] p-4">
            <p className="text-xs font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
              Sequence semantic style
            </p>
            <div
              className="mt-3 flex items-center gap-3"
              onDragOver={(event) => {
                event.preventDefault();
              }}
              onDrop={(event) => {
                event.preventDefault();
                const styleId = event.dataTransfer.getData('text/sequence-style');
                const style = sequencePalette.find((item) => item.id === styleId);
                if (style) {
                  applySequenceStyle(style);
                }
              }}
            >
              <div
                className="h-4 w-20 rounded-full"
                style={{ backgroundColor: selectedSequence?.color ?? '#94a3b8' }}
              />
              <p className="text-sm text-[rgba(224,231,239,0.76)]">
                Drag a semantic chip here, or click a chip below to apply it to the whole yarn.
              </p>
            </div>
          </div>

          <div className="grid gap-3 sm:grid-cols-2">
            {sequencePalette.map((style) => (
              <button
                className="rounded-[22px] border border-white/8 bg-[rgba(255,255,255,0.03)] p-4 text-left transition-colors hover:border-[rgba(245,170,49,0.28)] hover:bg-[rgba(245,170,49,0.08)]"
                draggable
                key={style.id}
                onClick={() => applySequenceStyle(style)}
                onDragStart={(event) => {
                  event.dataTransfer.setData('text/sequence-style', style.id);
                  event.dataTransfer.effectAllowed = 'copy';
                }}
                type="button"
              >
                <div className="flex items-center justify-between gap-3">
                  <span className="text-sm font-medium text-white">{style.label}</span>
                  <span
                    className="h-4 w-4 rounded-full shadow-[0_0_14px_rgba(255,255,255,0.12)]"
                    style={{ backgroundColor: style.color }}
                  />
                </div>
                <p className="mt-2 text-xs uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
                  {style.category}
                </p>
                <p className="mt-2 text-sm leading-6 text-[rgba(224,231,239,0.76)]">
                  {style.description}
                </p>
              </button>
            ))}
          </div>

          <div className="flex gap-3">
            <button
              className="flex-1 rounded-[20px] border border-white/8 bg-white/4 px-4 py-3 text-sm font-medium text-white transition-colors hover:bg-white/8"
              onClick={beginDraftSequence}
              type="button"
            >
              New sequence yarn
            </button>
            <button
              className="flex-1 rounded-[20px] border border-[rgba(255,94,91,0.28)] bg-[rgba(255,94,91,0.12)] px-4 py-3 text-sm font-medium text-[#ffd0cf] transition-colors hover:bg-[rgba(255,94,91,0.18)]"
              onClick={deleteSelectedNode}
              type="button"
            >
              Delete selected card
            </button>
          </div>

          <div className="rounded-[24px] border border-white/8 bg-[rgba(255,255,255,0.03)] p-4">
            <p className="text-xs font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
              Sequence inventory
            </p>
            <div className="mt-3 space-y-2">
              {sequences.map((sequence) => (
                <button
                  className={`flex w-full items-center justify-between rounded-[18px] px-3 py-3 text-left transition-colors ${
                    sequence.sequenceId === selectedSequence?.sequenceId
                      ? 'bg-white/8'
                      : 'bg-transparent hover:bg-white/4'
                  }`}
                  key={sequence.sequenceId}
                  onClick={() => {
                    setSelectedSequenceId(sequence.sequenceId);
                    if (draftSequence && draftSequence.sequenceId !== sequence.sequenceId) {
                      setDraftSequence(null);
                    }
                  }}
                  type="button"
                >
                  <div className="flex items-center gap-3">
                    <span
                      className="h-3 w-3 rounded-full"
                      style={{ backgroundColor: sequence.color }}
                    />
                    <div>
                      <p className="text-sm font-medium text-white">{sequence.label}</p>
                      <p className="text-xs uppercase tracking-[0.18em] text-[rgba(160,174,192,0.72)]">
                        {sequence.category}
                      </p>
                    </div>
                  </div>
                  <span className="text-xs text-[rgba(224,231,239,0.72)]">
                    {sequence.edgeCount} link{sequence.edgeCount === 1 ? '' : 's'}
                  </span>
                </button>
              ))}
            </div>
          </div>
        </>
      ),
    },
    node: {
      eyebrow: 'Selected Card',
      title: selectedNode?.data.title ?? 'No selection',
      badge: (
        <span
          className="rounded-full px-3 py-1 text-xs font-medium"
          style={{
            backgroundColor: selectedNode ? familyTint[selectedNode.data.family].chip : 'rgba(148,163,184,0.12)',
            color: selectedNode ? familyTint[selectedNode.data.family].text : '#cbd5e1',
          }}
        >
          {selectedNode ? familyLabel[selectedNode.data.family] : 'Card'}
        </span>
      ),
      content: (
        <>
          <label className="block">
            <span className="mb-2 block text-xs font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
              Card title
            </span>
            <input
              className="w-full rounded-[20px] border border-white/8 bg-white/4 px-4 py-3 text-sm text-white outline-none ring-0 transition-colors focus:border-[#f5aa31]"
              onChange={(event) => updateSelectedNode({ title: event.currentTarget.value })}
              onBlur={() => pushToast(`Updated card title to ${selectedNode?.data.title ?? 'Untitled card'} in ${selectedChainName}.`)}
              value={selectedNode?.data.title ?? ''}
            />
          </label>

          <label className="block">
            <span className="mb-2 block text-xs font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
              Scenario
            </span>
            <input
              className="w-full rounded-[20px] border border-white/8 bg-white/4 px-4 py-3 text-sm text-white outline-none ring-0 transition-colors focus:border-[#f5aa31]"
              onChange={(event) => updateSelectedNode({ scenario: event.currentTarget.value })}
              onBlur={() => pushToast(`Updated scenario for ${selectedCardTitle} in ${selectedChainName}.`)}
              value={selectedNode?.data.scenario ?? ''}
            />
          </label>

          <div className="grid gap-3 md:grid-cols-2">
            <label className="block">
              <span className="mb-2 block text-xs font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
                Inputs
              </span>
              <input
                className="w-full rounded-[20px] border border-white/8 bg-white/4 px-4 py-3 text-sm text-white outline-none ring-0 transition-colors focus:border-[#f5aa31]"
                onChange={(event) =>
                  updateSelectedNode({
                    inputs: event.currentTarget.value
                      .split(',')
                      .map((item) => item.trim())
                      .filter(Boolean),
                  })
                }
                onBlur={() => pushToast(`Updated inputs for ${selectedCardTitle} in ${selectedChainName}.`)}
                value={selectedNodePorts.inputs.join(', ')}
              />
            </label>

            <label className="block">
              <span className="mb-2 block text-xs font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
                Outputs
              </span>
              <input
                className="w-full rounded-[20px] border border-white/8 bg-white/4 px-4 py-3 text-sm text-white outline-none ring-0 transition-colors focus:border-[#f5aa31]"
                onChange={(event) =>
                  updateSelectedNode({
                    outputs: event.currentTarget.value
                      .split(',')
                      .map((item) => item.trim())
                      .filter(Boolean),
                  })
                }
                onBlur={() => pushToast(`Updated outputs for ${selectedCardTitle} in ${selectedChainName}.`)}
                value={selectedNodePorts.outputs.join(', ')}
              />
            </label>
          </div>

          <div className="flex gap-3">
            <button
              className="flex-1 rounded-[18px] border border-white/8 bg-white/4 px-4 py-3 text-sm font-medium text-white transition-colors hover:bg-white/8"
              onClick={() => {
                updateSelectedNode({
                  inputs: [...selectedNodePorts.inputs, `Input ${selectedNodePorts.inputs.length + 1}`],
                });
                pushToast(`Added input port to ${selectedCardTitle} in ${selectedChainName}.`);
              }}
              type="button"
            >
              Add input
            </button>
            <button
              className="flex-1 rounded-[18px] border border-white/8 bg-white/4 px-4 py-3 text-sm font-medium text-white transition-colors hover:bg-white/8"
              onClick={() => {
                updateSelectedNode({
                  outputs: [...selectedNodePorts.outputs, `Output ${selectedNodePorts.outputs.length + 1}`],
                  outputConditions: [...selectedNodeOutputConditions, ''],
                });
                pushToast(`Added output port to ${selectedCardTitle} in ${selectedChainName}.`);
              }}
              type="button"
            >
              Add output
            </button>
          </div>

          <div className="rounded-[24px] border border-white/8 bg-[rgba(255,255,255,0.03)] p-4">
            <p className="text-xs font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
              Branch Rules
            </p>
            <div className="mt-4 space-y-3">
              {selectedNodePorts.outputs.map((outputLabel, index) => (
                <div
                  className="rounded-[20px] border border-white/8 bg-white/4 p-3"
                  key={`branch-rule-${index}`}
                >
                  <div className="flex items-center justify-between gap-3">
                    <span className="text-xs font-semibold uppercase tracking-[0.18em] text-[rgba(160,174,192,0.72)]">
                      Output {index + 1}
                    </span>
                    <button
                      className="rounded-full border border-[rgba(255,94,91,0.28)] bg-[rgba(255,94,91,0.12)] px-3 py-1 text-[11px] font-medium text-[#ffd0cf] transition-colors hover:bg-[rgba(255,94,91,0.18)]"
                      onClick={() => {
                        updateSelectedNode({
                          outputs: selectedNodePorts.outputs.filter((_, itemIndex) => itemIndex !== index),
                          outputConditions: selectedNodeOutputConditions.filter(
                            (_, itemIndex) => itemIndex !== index,
                          ),
                        });
                        pushToast(`Removed output ${index + 1} from ${selectedCardTitle} in ${selectedChainName}.`);
                      }}
                      type="button"
                    >
                      Remove
                    </button>
                  </div>

                  <div className="mt-3 grid gap-3 md:grid-cols-2">
                    <label className="block">
                      <span className="mb-2 block text-xs font-semibold uppercase tracking-[0.16em] text-[rgba(160,174,192,0.68)]">
                        Branch label
                      </span>
                      <input
                        className="w-full rounded-[16px] border border-white/8 bg-[rgba(11,19,29,0.96)] px-3 py-2 text-sm text-white outline-none transition-colors focus:border-[#f5aa31]"
                        onChange={(event) =>
                          updateSelectedNode({
                            outputs: selectedNodePorts.outputs.map((item, itemIndex) =>
                              itemIndex === index ? event.currentTarget.value : item,
                            ),
                          })
                        }
                        onBlur={() => pushToast(`Updated branch label on ${selectedCardTitle} in ${selectedChainName}.`)}
                        value={outputLabel}
                      />
                    </label>

                    <label className="block">
                      <span className="mb-2 block text-xs font-semibold uppercase tracking-[0.16em] text-[rgba(160,174,192,0.68)]">
                        Condition
                      </span>
                      <input
                        className="w-full rounded-[16px] border border-white/8 bg-[rgba(11,19,29,0.96)] px-3 py-2 text-sm text-white outline-none transition-colors focus:border-[#f5aa31]"
                        onChange={(event) =>
                          updateSelectedNode({
                            outputConditions: selectedNodePorts.outputs.map((_, itemIndex) =>
                              itemIndex === index
                                ? event.currentTarget.value
                                : selectedNodeOutputConditions[itemIndex] ?? '',
                              ),
                            })
                          }
                        onBlur={() => pushToast(`Updated branch rule on ${selectedCardTitle} in ${selectedChainName}.`)}
                        placeholder="archetype >= 8"
                        value={selectedNodeOutputConditions[index] ?? ''}
                      />
                    </label>
                  </div>
                </div>
              ))}
            </div>
          </div>

          <div className="rounded-[24px] border border-white/8 bg-[rgba(255,255,255,0.03)] p-4">
            <p className="text-xs font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
              Input Ports
            </p>
            <div className="mt-4 space-y-3">
              {selectedNodePorts.inputs.map((inputLabel, index) => (
                <div
                  className="flex items-center gap-3 rounded-[18px] border border-white/8 bg-white/4 p-3"
                  key={`input-port-${index}`}
                >
                  <input
                    className="flex-1 rounded-[16px] border border-white/8 bg-[rgba(11,19,29,0.96)] px-3 py-2 text-sm text-white outline-none transition-colors focus:border-[#f5aa31]"
                    onChange={(event) =>
                      updateSelectedNode({
                        inputs: selectedNodePorts.inputs.map((item, itemIndex) =>
                          itemIndex === index ? event.currentTarget.value : item,
                        ),
                      })
                    }
                    onBlur={() => pushToast(`Updated input port on ${selectedCardTitle} in ${selectedChainName}.`)}
                    value={inputLabel}
                  />
                  <button
                    className="rounded-full border border-[rgba(255,94,91,0.28)] bg-[rgba(255,94,91,0.12)] px-3 py-1 text-[11px] font-medium text-[#ffd0cf] transition-colors hover:bg-[rgba(255,94,91,0.18)]"
                    onClick={() => {
                      updateSelectedNode({
                        inputs: selectedNodePorts.inputs.filter((_, itemIndex) => itemIndex !== index),
                      });
                      pushToast(`Removed input ${index + 1} from ${selectedCardTitle} in ${selectedChainName}.`);
                    }}
                    type="button"
                  >
                    Remove
                  </button>
                </div>
              ))}
            </div>
          </div>

          <label className="block">
            <span className="mb-2 block text-xs font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
              Detail
            </span>
            <textarea
              className="min-h-32 w-full rounded-[24px] border border-white/8 bg-white/4 px-4 py-3 text-sm leading-6 text-white outline-none transition-colors focus:border-[#f5aa31]"
              onChange={(event) => updateSelectedNode({ detail: event.currentTarget.value })}
              onBlur={() => pushToast(`Updated detail for ${selectedCardTitle} in ${selectedChainName}.`)}
              value={selectedNode?.data.detail ?? ''}
            />
          </label>
        </>
      ),
    },
  };

  return (
    <div className="space-y-4">
      <ToastStack toasts={toasts} />
      <div className="grid items-stretch gap-4 xl:grid-cols-[minmax(0,1.9fr)_420px]">
        <section className="flex h-full min-h-0 flex-col overflow-hidden rounded-[32px] border border-white/8 bg-[rgba(9,18,28,0.8)] shadow-[0_24px_80px_rgba(0,0,0,0.22)]">
          <div className="flex flex-wrap items-center justify-between gap-3 border-b border-white/6 px-6 py-5">
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.24em] text-[rgba(160,174,192,0.72)]">
                Content Chain Workbench
              </p>
              <h2 className="mt-2 text-xl font-semibold tracking-[-0.04em] text-white">
                Castle CellBlock mission graph
              </h2>
            </div>
            <div className="flex flex-wrap gap-2 text-xs text-[rgba(224,231,239,0.76)]">
              <span className="rounded-full border border-white/8 bg-white/4 px-3 py-2">
                {activeDraftState?.storage === 'database'
                  ? 'PostgreSQL draft'
                  : activeDraftState?.storage === 'browser'
                    ? 'Browser draft'
                    : 'Seed content'}
              </span>
              <span
                className={`rounded-full border px-3 py-2 ${
                  isDraftDirty
                    ? 'border-[rgba(245,170,49,0.28)] bg-[rgba(245,170,49,0.12)] text-[#ffd38a]'
                    : 'border-[rgba(34,197,94,0.28)] bg-[rgba(34,197,94,0.12)] text-[#c7ffd5]'
                }`}
              >
                {isDraftDirty ? 'Unsaved changes' : 'Saved'}
              </span>
              <span className="rounded-full border border-white/8 bg-white/4 px-3 py-2">
                Last saved {formatTimestampLabel(activeDraftState?.lastSavedAt)}
              </span>
              {activeDraftState?.recoveredFromAutosave ? (
                <span className="rounded-full border border-[rgba(94,184,179,0.28)] bg-[rgba(94,184,179,0.12)] px-3 py-2 text-[#a7f0eb]">
                  Recovered from autosave
                </span>
              ) : null}
              {activeAutosaveRecovery ? (
                <span className="rounded-full border border-[rgba(94,184,179,0.28)] bg-[rgba(94,184,179,0.12)] px-3 py-2 text-[#a7f0eb]">
                  Autosave ready
                </span>
              ) : null}
              <span className="rounded-full border border-white/8 bg-white/4 px-3 py-2">
                {visibleChains.length} chains
              </span>
              <span className="rounded-full border border-white/8 bg-white/4 px-3 py-2">
                {visibleNodes.filter((node) => isMissionData(node.data)).length} cards
              </span>
              <span className="rounded-full border border-white/8 bg-white/4 px-3 py-2">
                {visibleEdges.length} links
              </span>
              <button
                className={`rounded-full px-4 py-2 text-sm font-medium transition-colors ${
                  savingDraft
                    ? 'border border-white/8 bg-white/4 text-[rgba(224,231,239,0.6)]'
                    : isDraftDirty
                      ? 'border border-[#22c55e]/30 bg-[rgba(34,197,94,0.14)] text-[#c7ffd5]'
                      : 'border border-white/8 bg-white/4 text-[rgba(224,231,239,0.82)]'
                }`}
                disabled={savingDraft}
                onClick={() => {
                  void handleSaveDraft();
                }}
                type="button"
              >
                {savingDraft ? 'Saving...' : 'Save draft'}
              </button>
              <button
                className="rounded-full border border-white/8 bg-white/4 px-4 py-2 text-sm font-medium text-[rgba(224,231,239,0.82)] transition-colors hover:bg-white/8 disabled:text-[rgba(224,231,239,0.45)]"
                disabled={!activeDraftState?.persistedDraft || !isDraftDirty}
                onClick={handleRevertDraft}
                type="button"
              >
                Revert draft
              </button>
              {activeAutosaveRecovery ? (
                <>
                  <button
                    className="rounded-full border border-[rgba(94,184,179,0.28)] bg-[rgba(94,184,179,0.12)] px-4 py-2 text-sm font-medium text-[#a7f0eb] transition-colors hover:bg-[rgba(94,184,179,0.18)]"
                    onClick={handleRecoverAutosave}
                    type="button"
                  >
                    Recover autosave
                  </button>
                  <button
                    className="rounded-full border border-white/8 bg-white/4 px-4 py-2 text-sm font-medium text-[rgba(224,231,239,0.82)] transition-colors hover:bg-white/8"
                    onClick={dismissAutosaveRecovery}
                    type="button"
                  >
                    Dismiss recovery
                  </button>
                </>
              ) : null}
              <button
                className={`rounded-full px-3 py-2 transition-colors ${
                  edgeMode === 'sequenceThread'
                    ? 'border border-[#ff5e5b]/40 bg-[rgba(255,94,91,0.14)] text-[#ffd0cf]'
                    : 'border border-white/8 bg-white/4 text-[rgba(224,231,239,0.76)]'
                }`}
                onClick={() => setEdgeMode('sequenceThread')}
                type="button"
              >
                Sequence yarn
              </button>
              <button
                className={`rounded-full px-3 py-2 transition-colors ${
                  edgeMode === 'default'
                    ? 'border border-[#13a2a4]/40 bg-[rgba(19,162,164,0.14)] text-[#8ae5e2]'
                    : 'border border-white/8 bg-white/4 text-[rgba(224,231,239,0.76)]'
                }`}
                onClick={() => setEdgeMode('default')}
                type="button"
              >
                Cross-chain link
              </button>
            </div>
          </div>

          <div className="min-h-[980px] flex-1">
            <ReactFlow<EditorNodeData, SequenceEdgeData>
              className="h-full"
              connectionLineStyle={
                edgeMode === 'sequenceThread'
                  ? { stroke: selectedSequence?.color ?? '#ff5e5b', strokeWidth: 4 }
                  : { stroke: '#5eb8b3', strokeDasharray: '8 8', strokeWidth: 3 }
              }
              defaultEdgeOptions={{
                style: { stroke: '#5eb8b3', strokeDasharray: '8 8', strokeWidth: 2 },
                type: 'smoothstep',
              }}
              edgeTypes={edgeTypes}
              edges={visibleEdges}
              fitView
              fitViewOptions={{ padding: 0.18 }}
              nodeTypes={nodeTypes}
              nodes={visibleNodes}
              onConnect={handleConnect}
              onEdgeClick={(_, edge) => {
                if (edge.type === 'sequenceThread' && edge.data?.sequenceId) {
                  setSelectedSequenceId(edge.data.sequenceId);
                }

                const sourceNode = nodesById.get(edge.source);
                if (sourceNode?.parentId) {
                  setSelectedChainId(sourceNode.parentId);
                }
              }}
              onEdgesChange={handleEdgesChange}
              onNodeClick={(_, node) => {
                if (isChainData(node.data)) {
                  setSelectedChainId(node.id);
                  return;
                }

                setSelectedNodeId(node.id);
                if (node.parentId) {
                  setSelectedChainId(node.parentId);
                }
              }}
              onNodeDragStop={handleNodeDragStop}
              onNodeDragStart={(_event, node) => {
                if (!isChainData(node.data)) {
                  setDraggingNodeId(node.id);
                }
              }}
              onNodesChange={handleNodesChange}
              proOptions={{ hideAttribution: true }}
            >
              <Background
                color="rgba(255,255,255,0.08)"
                gap={24}
                size={1}
                variant={BackgroundVariant.Dots}
              />
              <Controls
                className="!border !border-white/10 !bg-[rgba(9,18,28,0.82)] !text-white"
                showInteractive={false}
              />
              <Panel
                className="!max-w-[620px] !rounded-[20px] !border !border-white/8 !bg-[rgba(9,18,28,0.8)] !px-4 !py-3 !text-xs !text-[rgba(224,231,239,0.74)]"
                position="top-left"
              >
                The canvas now mirrors the content-engine docs: each tinted region is a real content chain, with explicit sequence, trigger, condition, action, and counter lanes. Use the Space and Mission filters to swap between space view and single-mission view, then pull a sequence yarn from red <strong>OUT</strong> to red <strong>IN</strong> or expose extra named ports for branching cards.
              </Panel>
            </ReactFlow>
          </div>
        </section>

        <aside className="space-y-4">
          {panelOrder.map((panelId) => {
            const panel = inspectorPanels[panelId];

            return (
              <InspectorPanelShell
                badge={panel.badge}
                collapsed={collapsedPanels[panelId]}
                dragging={draggingPanelId === panelId}
                eyebrow={panel.eyebrow}
                key={panelId}
                onDragEnd={handlePanelDragEnd}
                onDragOver={handlePanelDragOver}
                onDragStart={handlePanelDragStart(panelId)}
                onDrop={handlePanelDrop(panelId)}
                onToggle={() => togglePanelCollapsed(panelId)}
                title={panel.title}
              >
                {panel.content}
              </InspectorPanelShell>
            );
          })}
        </aside>
      </div>

      <MissionCardLibrary
        onAdd={addScenarioNode}
        selectedChainName={selectedChain?.name ?? 'No chain selected'}
      />
    </div>
  );
}

export default function ChainFlowWorkbench() {
  return (
    <ReactFlowProvider>
      <FlowContent />
    </ReactFlowProvider>
  );
}
