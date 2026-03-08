export type PrimitiveFamily = 'anchor' | 'trigger' | 'condition' | 'action' | 'counter';

export type CardRelevance = 'mission' | 'space';

export type ScenarioTemplate = {
  id: string;
  family: PrimitiveFamily;
  stage: string;
  title: string;
  detail: string;
  status: string;
  scenario: string;
  accent: string;
  relevance: CardRelevance[];
  tags: string[];
  properties: Array<{ label: string; value: string }>;
};

export type LibrarySection = {
  id: string;
  title: string;
  templates: ScenarioTemplate[];
};

export type MissionCardLibraryQuery = {
  family: PrimitiveFamily | 'all';
  favoriteIds: string[];
  recentIds: string[];
  relevance: CardRelevance | 'all';
  search: string;
  showFavoritesOnly: boolean;
  showRecentsOnly: boolean;
};

const sequenceAnchorCatalog: ScenarioTemplate[] = [
  {
    id: 'sequence-start',
    family: 'anchor',
    stage: 'Start',
    title: 'Sequence Start',
    detail:
      'Place this at the first readable beat in a chain frame, then pull a sequence yarn into the first trigger or action.',
    status: 'Marker',
    scenario: 'Sequence anchor',
    accent: '#ff5e5b',
    relevance: ['mission', 'space'],
    tags: ['sequence', 'start', 'anchor', 'thread'],
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
    detail:
      'Place this at the end of a beat so designers can see exactly where a mission branch resolves.',
    status: 'Marker',
    scenario: 'Sequence anchor',
    accent: '#ff5e5b',
    relevance: ['mission', 'space'],
    tags: ['sequence', 'end', 'anchor', 'thread'],
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
    detail:
      'Player enters a named spatial region such as Castle_CellBlock.Region8 or Region4.',
    status: 'ref_event_types',
    scenario: 'space event',
    accent: '#06b6d4',
    relevance: ['space'],
    tags: ['trigger', 'region', 'enter', 'space'],
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
    relevance: ['mission', 'space'],
    tags: ['trigger', 'interact', 'tag', 'entity'],
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
    detail:
      'Player selects a dialog option, keyed by dialog id and often used to fork mission progression.',
    status: 'ref_event_types',
    scenario: 'ui event',
    accent: '#8b5cf6',
    relevance: ['mission'],
    tags: ['trigger', 'dialog', 'choice', 'branch'],
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
    detail:
      'Check whether a mission is active, completed, failed, or not active before letting the chain continue.',
    status: 'ref_condition_types',
    scenario: 'mission gate',
    accent: '#22c55e',
    relevance: ['mission', 'space'],
    tags: ['condition', 'mission', 'status', 'gate'],
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
    detail:
      'Gate the flow on whether a specific mission step is active, completed, or not yet started.',
    status: 'ref_condition_types',
    scenario: 'mission gate',
    accent: '#f59e0b',
    relevance: ['mission'],
    tags: ['condition', 'step', 'mission', 'gate'],
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
    detail:
      'Split the chain on player archetype so Jaffa and Human branches stay explicit in the editor.',
    status: 'ref_condition_types',
    scenario: 'player gate',
    accent: '#3b82f6',
    relevance: ['mission'],
    tags: ['condition', 'archetype', 'branch', 'player'],
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
    detail:
      'Player accepts a mission immediately. Useful for space-script style orchestration chains.',
    status: 'ref_action_types',
    scenario: 'mission action',
    accent: '#22c55e',
    relevance: ['mission', 'space'],
    tags: ['action', 'accept', 'mission'],
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
    detail:
      'Advance to a specified mission step inside a chain after the condition lane passes.',
    status: 'ref_action_types',
    scenario: 'mission action',
    accent: '#06b6d4',
    relevance: ['mission'],
    tags: ['action', 'advance', 'step', 'mission'],
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
    detail:
      'Open a dialog immediately, often as a bridge between mission beats and player-facing story.',
    status: 'ref_action_types',
    scenario: 'ui action',
    accent: '#8b5cf6',
    relevance: ['mission', 'space'],
    tags: ['action', 'dialog', 'ui', 'story'],
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
    detail:
      'Open the reward selection surface at the end of a sequence or a mission completion handoff.',
    status: 'ref_action_types',
    scenario: 'mission action',
    accent: '#eab308',
    relevance: ['mission'],
    tags: ['action', 'rewards', 'completion', 'mission'],
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
    detail:
      'Track hidden hallway controllers, kill counts, or collection beats before an action fires.',
    status: 'content_counters',
    scenario: 'runtime counter',
    accent: '#94a3b8',
    relevance: ['mission', 'space'],
    tags: ['counter', 'threshold', 'progress', 'runtime'],
    properties: [
      { label: 'counter_key', value: 'hallway_01_progress' },
      { label: 'operator', value: 'gte' },
      { label: 'value', value: '1' },
    ],
  },
];

export const missionCardSections: LibrarySection[] = [
  { id: 'favorites', title: 'Favorites', templates: [] },
  { id: 'recents', title: 'Recent Cards', templates: [] },
  { id: 'anchors', title: 'Sequence Anchors', templates: sequenceAnchorCatalog },
  { id: 'triggers', title: 'Trigger Cards', templates: triggerReferenceCatalog },
  { id: 'conditions', title: 'Condition Cards', templates: conditionReferenceCatalog },
  { id: 'actions', title: 'Action Cards', templates: actionReferenceCatalog },
  { id: 'counters', title: 'Counter Cards', templates: counterReferenceCatalog },
];

export const allMissionCardTemplates = missionCardSections
  .filter((section) => section.id !== 'favorites' && section.id !== 'recents')
  .flatMap((section) => section.templates);

export function matchesMissionCardSearch(template: ScenarioTemplate, rawQuery: string) {
  const query = rawQuery.trim().toLowerCase();
  if (!query) {
    return true;
  }

  const haystack = [
    template.title,
    template.detail,
    template.status,
    template.scenario,
    template.stage,
    template.family,
    ...template.tags,
    ...template.properties.flatMap((property) => [property.label, property.value]),
  ]
    .join(' ')
    .toLowerCase();

  return haystack.includes(query);
}

export function buildMissionCardLibrarySections(query: MissionCardLibraryQuery): LibrarySection[] {
  const favoriteIds = new Set(query.favoriteIds);
  const recentRank = new Map(query.recentIds.map((id, index) => [id, index] as const));

  const filtered = allMissionCardTemplates.filter((template) => {
    if (query.family !== 'all' && template.family !== query.family) {
      return false;
    }

    if (query.relevance !== 'all' && !template.relevance.includes(query.relevance)) {
      return false;
    }

    if (query.showFavoritesOnly && !favoriteIds.has(template.id)) {
      return false;
    }

    if (query.showRecentsOnly && !recentRank.has(template.id)) {
      return false;
    }

    return matchesMissionCardSearch(template, query.search);
  });

  const filteredIds = new Set(filtered.map((template) => template.id));
  const favorites = allMissionCardTemplates.filter(
    (template) => favoriteIds.has(template.id) && filteredIds.has(template.id),
  );
  const recents = [...filtered]
    .filter((template) => recentRank.has(template.id))
    .sort((left, right) => (recentRank.get(left.id) ?? 999) - (recentRank.get(right.id) ?? 999));

  const standardSections = missionCardSections
    .filter((section) => section.id !== 'favorites' && section.id !== 'recents')
    .map((section) => ({
      ...section,
      templates: section.templates.filter((template) => filteredIds.has(template.id)),
    }))
    .filter((section) => section.templates.length > 0);

  const sections: LibrarySection[] = [];
  if (favorites.length > 0) {
    sections.push({ id: 'favorites', title: 'Favorites', templates: favorites });
  }
  if (recents.length > 0) {
    sections.push({ id: 'recents', title: 'Recent Cards', templates: recents });
  }

  if (query.showFavoritesOnly) {
    return sections.filter((section) => section.id === 'favorites');
  }

  if (query.showRecentsOnly) {
    return sections.filter((section) => section.id === 'recents');
  }

  return [...sections, ...standardSections];
}
