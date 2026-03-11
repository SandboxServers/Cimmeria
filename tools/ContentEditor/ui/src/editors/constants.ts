import type {
  MissionOption,
  PrimitiveFamily,
  ScenarioTemplate,
  SequenceStyle,
  SpaceOption,
} from './types';

export const familyLabel: Record<PrimitiveFamily, string> = {
  anchor: 'Sequence',
  trigger: 'Trigger',
  condition: 'Condition',
  action: 'Action',
  counter: 'Counter',
};

export const familyTint: Record<PrimitiveFamily, { chip: string; text: string; glow: string }> = {
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

export const laneDescriptors: Array<{ family: PrimitiveFamily; title: string; blurb: string }> = [
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

export const orderedFamilies: PrimitiveFamily[] = [
  'anchor',
  'trigger',
  'condition',
  'action',
  'counter',
];

export const AUTO_LAYOUT = {
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

export const spaceCatalog: SpaceOption[] = [
  { id: 'Castle_CellBlock', label: 'Castle CellBlock' },
  { id: 'Castle', label: 'Castle' },
  { id: 'Harset', label: 'Harset' },
  { id: 'SGC_W1', label: 'SGC W1' },
];

export const missionCatalog: MissionOption[] = [
  { id: '622', label: '622 - Arm Yourself!', spaceId: 'Castle_CellBlock' },
  { id: '638', label: '638 - Speak to Prisoner 329', spaceId: 'Castle_CellBlock' },
  { id: '639', label: '639 - Find Ambernol', spaceId: 'Castle_CellBlock' },
  { id: '680', label: '680 - Escape the Cellblock', spaceId: 'Castle_CellBlock' },
  { id: '681', label: '681 - Mess Hall Controller', spaceId: 'Castle_CellBlock' },
  { id: '684', label: '684 - Hallway03 Controller', spaceId: 'Castle_CellBlock' },
  { id: '687', label: '687 - Aftermath', spaceId: 'Castle_CellBlock' },
];

export const sequencePalette: SequenceStyle[] = [
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

export const sequenceAnchorCatalog: ScenarioTemplate[] = [
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

export const triggerReferenceCatalog: ScenarioTemplate[] = [
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

export const conditionReferenceCatalog: ScenarioTemplate[] = [
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

export const actionReferenceCatalog: ScenarioTemplate[] = [
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

export const counterReferenceCatalog: ScenarioTemplate[] = [
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
