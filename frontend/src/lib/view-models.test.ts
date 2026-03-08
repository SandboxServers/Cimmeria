import { describe, expect, it } from 'vitest';
import { clampProgressValue, getPlayerStatusVariant } from './view-models';

describe('view-models', () => {
  it('clamps progress values into the supported range', () => {
    expect(clampProgressValue(-10)).toBe(0);
    expect(clampProgressValue(42)).toBe(42);
    expect(clampProgressValue(120)).toBe(100);
  });

  it('maps player statuses into badge variants', () => {
    expect(getPlayerStatusVariant('Combat')).toBe('destructive');
    expect(getPlayerStatusVariant('In mission')).toBe('default');
    expect(getPlayerStatusVariant('Crafting')).toBe('secondary');
    expect(getPlayerStatusVariant('Idle')).toBe('success');
  });
});
