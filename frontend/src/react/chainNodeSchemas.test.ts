import { describe, expect, it } from 'vitest';
import {
  applyNodeTypeSelection,
  buildFallbackEditorPickers,
  getFieldOptions,
  getNodeSchema,
  getPropertyMap,
  type ChainEditorPickers,
} from './chainNodeSchemas';

describe('chain node schemas', () => {
  const pickers: ChainEditorPickers = {
    ...buildFallbackEditorPickers(
      [
        { id: 'Castle_CellBlock', label: 'Castle CellBlock' },
        { id: 'Castle_Hallway', label: 'Castle Hallway' },
      ],
      [{ id: '622', label: '622 - Arm Yourself', spaceId: 'Castle_CellBlock' }],
    ),
    dialogs: [
      { value: '2982', label: '2982 - Corpse Dialog' },
      { value: '5020', label: '5020 - Prisoner Choice' },
    ],
    items: [{ value: '9001', label: '9001 - Pistol' }],
    regions: [
      {
        value: 'Castle_CellBlock.Region8',
        label: 'Castle_CellBlock.Region8',
        spaceId: 'Castle_CellBlock',
      },
      {
        value: 'Castle_Hallway.Region1',
        label: 'Castle_Hallway.Region1',
        spaceId: 'Castle_Hallway',
      },
    ],
    steps: [
      { value: '214', label: '214 - Step 214', missionId: '622' },
      { value: '315', label: '315 - Step 315', missionId: '638' },
    ],
  };

  it('filters space pickers to the selected space', () => {
    const field = getNodeSchema('trigger')?.types.enter_region.fields[0];
    expect(field).toBeDefined();

    expect(
      getFieldOptions(field!, pickers, [{ label: 'event_type', value: 'enter_region' }], 'Castle_CellBlock'),
    ).toEqual([
      {
        value: 'Castle_CellBlock.Region8',
        label: 'Castle_CellBlock.Region8',
        spaceId: 'Castle_CellBlock',
      },
    ]);
  });

  it('filters step pickers by the selected mission property', () => {
    const field = getNodeSchema('condition')?.types.step_status.fields[1];
    expect(field).toBeDefined();

    expect(
      getFieldOptions(
        field!,
        pickers,
        [
          { label: 'condition_type', value: 'step_status' },
          { label: 'target_id', value: '622' },
        ],
        'Castle_CellBlock',
      ),
    ).toEqual([{ value: '214', label: '214 - Step 214', missionId: '622' }]);
  });

  it('applies typed defaults while preserving custom properties', () => {
    const properties = applyNodeTypeSelection(
      'action',
      [
        { label: 'action_type', value: 'advance_step' },
        { label: 'target_id', value: '622' },
        { label: 'target_key', value: '214' },
        { label: 'custom_note', value: 'keep me' },
      ],
      'display_dialog',
    );

    expect(getPropertyMap(properties)).toEqual({
      action_type: 'display_dialog',
      target_id: '622',
      custom_note: 'keep me',
    });
  });

  it('retains shared typed values when switching between compatible action types', () => {
    const properties = applyNodeTypeSelection(
      'action',
      [
        { label: 'action_type', value: 'accept_mission' },
        { label: 'target_id', value: '680' },
        { label: 'delay_ms', value: '250' },
      ],
      'complete_mission',
    );

    expect(getPropertyMap(properties)).toEqual({
      action_type: 'complete_mission',
      target_id: '680',
      delay_ms: '250',
    });
  });
});
