import type { PrimitiveFamily } from './missionCardCatalog';

export type NodeProperty = {
  label: string;
  value: string;
};

export type ChainPickerOption = {
  value: string;
  label: string;
  group?: string;
};

export type ChainEditorPickers = {
  spaces: ChainPickerOption[];
  missions: Array<ChainPickerOption & { spaceId: string }>;
  dialogs: ChainPickerOption[];
  items: ChainPickerOption[];
  regions: Array<ChainPickerOption & { spaceId: string }>;
  steps: Array<ChainPickerOption & { missionId: string }>;
};

export type ChainFieldSchema = {
  key: string;
  label: string;
  kind: 'text' | 'textarea' | 'number' | 'select' | 'picker';
  placeholder?: string;
  helperText?: string;
  options?: ChainPickerOption[];
  pickerSource?: keyof ChainEditorPickers;
  allowEmpty?: boolean;
  filterByMissionProperty?: string;
  filterBySpaceSelection?: boolean;
};

export type ChainTypeSchema = {
  description: string;
  fields: ChainFieldSchema[];
};

export type ChainFamilySchema = {
  typeKey: string;
  typeLabel: string;
  typeOptions: ChainPickerOption[];
  types: Record<string, ChainTypeSchema>;
};

const scopeOptions: ChainPickerOption[] = [
  { value: 'player', label: 'Player' },
  { value: 'mission', label: 'Mission' },
  { value: 'space', label: 'Space' },
];

const missionStatusOptions: ChainPickerOption[] = [
  { value: 'active', label: 'Active' },
  { value: 'completed', label: 'Completed' },
  { value: 'failed', label: 'Failed' },
  { value: 'not_active', label: 'Not Active' },
];

const stepStatusOptions: ChainPickerOption[] = [
  { value: 'active', label: 'Active' },
  { value: 'completed', label: 'Completed' },
  { value: 'not_started', label: 'Not Started' },
];

const operatorOptions: ChainPickerOption[] = [
  { value: 'eq', label: 'Equals' },
  { value: 'neq', label: 'Does Not Equal' },
  { value: 'gte', label: 'Greater Than or Equal' },
  { value: 'gt', label: 'Greater Than' },
  { value: 'lte', label: 'Less Than or Equal' },
  { value: 'lt', label: 'Less Than' },
];

const rewardCategoryOptions: ChainPickerOption[] = [
  { value: 'mission', label: 'Mission Reward' },
  { value: 'bonus', label: 'Bonus Reward' },
  { value: 'loot', label: 'Loot Payout' },
];

const triggerSchema: ChainFamilySchema = {
  typeKey: 'event_type',
  typeLabel: 'Trigger type',
  typeOptions: [
    { value: 'enter_region', label: 'Enter Region' },
    { value: 'interact_tag', label: 'Interact Tag' },
    { value: 'dialog_choice', label: 'Dialog Choice' },
    { value: 'dialog_open', label: 'Dialog Open' },
  ],
  types: {
    enter_region: {
      description: 'Subscribe to a region enter event in the selected space.',
      fields: [
        {
          key: 'event_key',
          label: 'Region',
          kind: 'picker',
          pickerSource: 'regions',
          placeholder: 'Castle_CellBlock.Region8',
          filterBySpaceSelection: true,
        },
        {
          key: 'scope',
          label: 'Scope',
          kind: 'select',
          options: scopeOptions,
        },
      ],
    },
    interact_tag: {
      description: 'Listen for interaction events on a mission prop or entity tag.',
      fields: [
        {
          key: 'event_key',
          label: 'Entity Tag',
          kind: 'text',
          placeholder: 'Cellblock_WoodenCrate',
        },
        {
          key: 'scope',
          label: 'Scope',
          kind: 'select',
          options: scopeOptions,
        },
      ],
    },
    dialog_choice: {
      description: 'Respond to a specific dialog choice and branch the mission accordingly.',
      fields: [
        {
          key: 'event_key',
          label: 'Dialog',
          kind: 'picker',
          pickerSource: 'dialogs',
          placeholder: '5020',
        },
        {
          key: 'scope',
          label: 'Scope',
          kind: 'select',
          options: [{ value: 'player', label: 'Player' }],
        },
      ],
    },
    dialog_open: {
      description: 'React when a dialog opens for the player.',
      fields: [
        {
          key: 'event_key',
          label: 'Dialog',
          kind: 'picker',
          pickerSource: 'dialogs',
          placeholder: 'Dialog ID',
        },
        {
          key: 'scope',
          label: 'Scope',
          kind: 'select',
          options: [{ value: 'player', label: 'Player' }],
        },
      ],
    },
  },
};

const conditionSchema: ChainFamilySchema = {
  typeKey: 'condition_type',
  typeLabel: 'Condition type',
  typeOptions: [
    { value: 'mission_status', label: 'Mission Status' },
    { value: 'step_status', label: 'Mission Step Status' },
    { value: 'archetype', label: 'Archetype' },
  ],
  types: {
    mission_status: {
      description: 'Check the current mission state before continuing the chain.',
      fields: [
        {
          key: 'target_id',
          label: 'Mission',
          kind: 'picker',
          pickerSource: 'missions',
          placeholder: 'Mission ID',
        },
        {
          key: 'operator',
          label: 'Operator',
          kind: 'select',
          options: operatorOptions.filter((option) => option.value === 'eq' || option.value === 'neq'),
        },
        {
          key: 'value',
          label: 'Status',
          kind: 'select',
          options: missionStatusOptions,
        },
      ],
    },
    step_status: {
      description: 'Gate progression on a specific mission step state.',
      fields: [
        {
          key: 'target_id',
          label: 'Mission',
          kind: 'picker',
          pickerSource: 'missions',
          placeholder: 'Mission ID',
        },
        {
          key: 'target_key',
          label: 'Step',
          kind: 'picker',
          pickerSource: 'steps',
          placeholder: 'Step ID',
          filterByMissionProperty: 'target_id',
        },
        {
          key: 'operator',
          label: 'Operator',
          kind: 'select',
          options: operatorOptions.filter((option) => option.value === 'eq' || option.value === 'neq'),
        },
        {
          key: 'value',
          label: 'Step state',
          kind: 'select',
          options: stepStatusOptions,
        },
      ],
    },
    archetype: {
      description: 'Split by player archetype or similar numeric classifications.',
      fields: [
        {
          key: 'operator',
          label: 'Operator',
          kind: 'select',
          options: operatorOptions,
        },
        {
          key: 'value',
          label: 'Archetype value',
          kind: 'number',
          placeholder: '8',
        },
      ],
    },
  },
};

const actionSchema: ChainFamilySchema = {
  typeKey: 'action_type',
  typeLabel: 'Action type',
  typeOptions: [
    { value: 'accept_mission', label: 'Accept Mission' },
    { value: 'advance_step', label: 'Advance Step' },
    { value: 'complete_mission', label: 'Complete Mission' },
    { value: 'display_dialog', label: 'Display Dialog' },
    { value: 'display_rewards', label: 'Display Rewards' },
    { value: 'add_item', label: 'Grant Item' },
  ],
  types: {
    accept_mission: {
      description: 'Accept a mission immediately from the chain.',
      fields: [
        {
          key: 'target_id',
          label: 'Mission',
          kind: 'picker',
          pickerSource: 'missions',
          placeholder: 'Mission ID',
        },
        {
          key: 'delay_ms',
          label: 'Delay (ms)',
          kind: 'number',
          placeholder: '0',
        },
      ],
    },
    advance_step: {
      description: 'Advance the active mission to a specific step.',
      fields: [
        {
          key: 'target_id',
          label: 'Mission',
          kind: 'picker',
          pickerSource: 'missions',
          placeholder: 'Mission ID',
        },
        {
          key: 'target_key',
          label: 'Step',
          kind: 'picker',
          pickerSource: 'steps',
          placeholder: 'Step ID',
          filterByMissionProperty: 'target_id',
        },
        {
          key: 'delay_ms',
          label: 'Delay (ms)',
          kind: 'number',
          placeholder: '0',
        },
      ],
    },
    complete_mission: {
      description: 'Complete the selected mission and hand the player forward.',
      fields: [
        {
          key: 'target_id',
          label: 'Mission',
          kind: 'picker',
          pickerSource: 'missions',
          placeholder: 'Mission ID',
        },
        {
          key: 'delay_ms',
          label: 'Delay (ms)',
          kind: 'number',
          placeholder: '0',
        },
      ],
    },
    display_dialog: {
      description: 'Open a dialog from the chain using a real dialog lookup.',
      fields: [
        {
          key: 'target_id',
          label: 'Dialog',
          kind: 'picker',
          pickerSource: 'dialogs',
          placeholder: 'Dialog ID',
        },
        {
          key: 'params',
          label: 'Params',
          kind: 'textarea',
          placeholder: '{ "npc": null }',
        },
      ],
    },
    display_rewards: {
      description: 'Open the mission rewards or payout surface.',
      fields: [
        {
          key: 'category',
          label: 'Reward category',
          kind: 'select',
          options: rewardCategoryOptions,
        },
        {
          key: 'delay_ms',
          label: 'Delay (ms)',
          kind: 'number',
          placeholder: '0',
        },
      ],
    },
    add_item: {
      description: 'Grant an item directly to the player.',
      fields: [
        {
          key: 'target_id',
          label: 'Item',
          kind: 'picker',
          pickerSource: 'items',
          placeholder: 'Item ID',
        },
        {
          key: 'quantity',
          label: 'Quantity',
          kind: 'number',
          placeholder: '1',
        },
      ],
    },
  },
};

const counterSchema: ChainFamilySchema = {
  typeKey: 'counter_type',
  typeLabel: 'Counter type',
  typeOptions: [{ value: 'progress_counter', label: 'Progress Counter' }],
  types: {
    progress_counter: {
      description: 'Track a hidden counter or threshold alongside the chain.',
      fields: [
        {
          key: 'counter_key',
          label: 'Counter key',
          kind: 'text',
          placeholder: 'prisoner_branch_progress',
        },
        {
          key: 'threshold',
          label: 'Threshold',
          kind: 'number',
          placeholder: '1',
        },
      ],
    },
  },
};

const anchorSchema: ChainFamilySchema = {
  typeKey: 'marker',
  typeLabel: 'Anchor marker',
  typeOptions: [
    { value: 'START', label: 'Start Marker' },
    { value: 'END', label: 'End Marker' },
  ],
  types: {
    START: {
      description: 'Visible start marker for a sequence yarn.',
      fields: [
        {
          key: 'behavior',
          label: 'Behavior',
          kind: 'text',
          placeholder: 'Visual grouping',
        },
      ],
    },
    END: {
      description: 'Visible end marker for a sequence yarn.',
      fields: [
        {
          key: 'behavior',
          label: 'Behavior',
          kind: 'text',
          placeholder: 'Visual grouping',
        },
      ],
    },
  },
};

export const chainFamilySchemas: Partial<Record<PrimitiveFamily, ChainFamilySchema>> = {
  anchor: anchorSchema,
  trigger: triggerSchema,
  condition: conditionSchema,
  action: actionSchema,
  counter: counterSchema,
};

export function getPropertyMap(properties: NodeProperty[]) {
  return Object.fromEntries(properties.map((property) => [property.label, property.value]));
}

export function setPropertyValue(properties: NodeProperty[], key: string, value: string) {
  const nextValue = value.trim();

  if (!nextValue) {
    return properties.filter((property) => property.label !== key);
  }

  const existingIndex = properties.findIndex((property) => property.label === key);
  if (existingIndex === -1) {
    return [...properties, { label: key, value }];
  }

  return properties.map((property, index) =>
    index === existingIndex ? { ...property, value } : property,
  );
}

export function renamePropertyKey(
  properties: NodeProperty[],
  previousKey: string,
  nextKey: string,
) {
  const normalizedNextKey = nextKey.trim();
  const existing = properties.find((property) => property.label === previousKey);

  if (!existing) {
    return properties;
  }

  if (!normalizedNextKey) {
    return properties.filter((property) => property.label !== previousKey);
  }

  const withoutPrevious = properties.filter((property) => property.label !== previousKey);
  return setPropertyValue(withoutPrevious, normalizedNextKey, existing.value);
}

export function getNodeSchema(family: PrimitiveFamily) {
  return chainFamilySchemas[family] ?? null;
}

export function getNodeTypeValue(family: PrimitiveFamily, properties: NodeProperty[]) {
  const familySchema = getNodeSchema(family);
  if (!familySchema) {
    return '';
  }

  const propertyMap = getPropertyMap(properties);
  return propertyMap[familySchema.typeKey] ?? '';
}

export function getDefaultFieldValue(field: ChainFieldSchema) {
  if (field.options?.length) {
    return field.options[0]?.value ?? '';
  }

  return field.kind === 'number' ? '0' : '';
}

export function getTypeSchema(
  family: PrimitiveFamily,
  properties: NodeProperty[],
) {
  const familySchema = getNodeSchema(family);
  if (!familySchema) {
    return null;
  }

  const typeValue = getNodeTypeValue(family, properties);
  if (!typeValue) {
    return null;
  }

  return familySchema.types[typeValue] ?? null;
}

export function applyNodeTypeSelection(
  family: PrimitiveFamily,
  properties: NodeProperty[],
  typeValue: string,
) {
  const familySchema = getNodeSchema(family);
  if (!familySchema) {
    return properties;
  }

  const existingMap = getPropertyMap(properties);
  const customProperties = getAdditionalProperties(family, properties);
  const nextTypeSchema = familySchema.types[typeValue];

  let nextProperties = setPropertyValue(customProperties, familySchema.typeKey, typeValue);

  if (!nextTypeSchema) {
    return nextProperties;
  }

  for (const field of nextTypeSchema.fields) {
    const nextValue = existingMap[field.key] || getDefaultFieldValue(field);
    nextProperties = setPropertyValue(nextProperties, field.key, nextValue);
  }

  return nextProperties;
}

export function getKnownPropertyKeys(family: PrimitiveFamily, properties: NodeProperty[]) {
  const familySchema = getNodeSchema(family);
  if (!familySchema) {
    return [];
  }

  const typeSchema = getTypeSchema(family, properties);
  const keys = [familySchema.typeKey];

  if (typeSchema) {
    keys.push(...typeSchema.fields.map((field) => field.key));
  }

  return keys;
}

export function getAdditionalProperties(family: PrimitiveFamily, properties: NodeProperty[]) {
  const knownKeys = new Set(getKnownPropertyKeys(family, properties));
  return properties.filter((property) => !knownKeys.has(property.label));
}

export function getFieldOptions(
  field: ChainFieldSchema,
  pickers: ChainEditorPickers,
  properties: NodeProperty[],
  selectedSpaceId?: string,
) {
  if (field.options) {
    return field.options;
  }

  if (!field.pickerSource) {
    return [];
  }

  const sourceOptions = pickers[field.pickerSource];
  if (!Array.isArray(sourceOptions)) {
    return [];
  }

  const propertyMap = getPropertyMap(properties);

  return sourceOptions.filter((option) => {
    if (field.filterBySpaceSelection && selectedSpaceId && 'spaceId' in option) {
      return option.spaceId === selectedSpaceId;
    }

    if (field.filterByMissionProperty && 'missionId' in option) {
      return option.missionId === propertyMap[field.filterByMissionProperty];
    }

    return true;
  });
}

export function buildFallbackEditorPickers(
  spaces: Array<{ id: string; label: string }>,
  missions: Array<{ id: string; label: string; spaceId: string }>,
): ChainEditorPickers {
  return {
    spaces: spaces.map((space) => ({ value: space.id, label: space.label })),
    missions: missions.map((mission) => ({
      value: mission.id,
      label: mission.label,
      spaceId: mission.spaceId,
    })),
    dialogs: [],
    items: [],
    regions: [],
    steps: [],
  };
}
