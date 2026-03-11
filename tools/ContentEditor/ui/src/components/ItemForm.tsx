import { useCallback } from 'react';
import type { Item } from '../editors/dataTypes';
import {
  Section,
  Field,
  TextInput,
  NumberInput,
  Checkbox,
  ArrayInput,
} from './EntityTemplateForm';

interface ItemFormProps {
  data: Item;
  onChange: (updated: Item) => void;
}

export default function ItemForm({ data, onChange }: ItemFormProps) {
  const set = useCallback(
    <K extends keyof Item>(key: K, value: Item[K]) => {
      onChange({ ...data, [key]: value });
    },
    [data, onChange],
  );

  const setNum = useCallback(
    (key: keyof Item, raw: string) => {
      const v = raw.trim() === '' ? null : Number(raw);
      set(key, (v != null && isNaN(v) ? null : v) as Item[keyof Item]);
    },
    [set],
  );

  return (
    <div className="subtle-scrollbar flex flex-col gap-1 overflow-y-auto p-3">
      <Section title="Identity" defaultOpen>
        <Field label="Item ID">
          <NumberInput value={data.item_id} disabled />
        </Field>
        <Field label="Name">
          <TextInput value={data.name} onChange={(v) => set('name', v || null)} />
        </Field>
        <Field label="Description">
          <textarea
            value={data.description ?? ''}
            onChange={(e) => set('description', e.target.value || null)}
            rows={3}
            className="w-full rounded border border-border bg-input px-1.5 py-0.5 text-xs text-foreground placeholder:text-muted-foreground focus:border-primary focus:outline-none"
          />
        </Field>
        <Field label="Icon Location">
          <TextInput value={data.icon_location} onChange={(v) => set('icon_location', v || null)} />
        </Field>
      </Section>

      <Section title="Classification" defaultOpen>
        <Field label="Quality ID">
          <NumberInput value={data.quality_id} onChange={(v) => setNum('quality_id', v)} />
        </Field>
        <Field label="Tier">
          <NumberInput value={data.tier} onChange={(v) => setNum('tier', v)} />
        </Field>
        <Field label="Flags">
          <NumberInput value={data.flags} onChange={(v) => setNum('flags', v)} />
        </Field>
      </Section>

      <Section title="Stacking">
        <Field label="Max Stack Size">
          <NumberInput value={data.max_stack_size} onChange={(v) => setNum('max_stack_size', v)} />
        </Field>
        <Field label="Charges">
          <NumberInput value={data.charges} onChange={(v) => setNum('charges', v)} />
        </Field>
        <Field label="Clip Size">
          <NumberInput value={data.clip_size} onChange={(v) => setNum('clip_size', v)} />
        </Field>
        <Field label="Default Ammo Type">
          <NumberInput value={data.default_ammo_type} onChange={(v) => setNum('default_ammo_type', v)} />
        </Field>
      </Section>

      <Section title="Range">
        <Field label="Min Ranged Range">
          <NumberInput value={data.min_ranged_range} onChange={(v) => setNum('min_ranged_range', v)} />
        </Field>
        <Field label="Max Ranged Range">
          <NumberInput value={data.max_ranged_range} onChange={(v) => setNum('max_ranged_range', v)} />
        </Field>
        <Field label="Min Melee Range">
          <NumberInput value={data.min_melee_range} onChange={(v) => setNum('min_melee_range', v)} />
        </Field>
        <Field label="Max Melee Range">
          <NumberInput value={data.max_melee_range} onChange={(v) => setNum('max_melee_range', v)} />
        </Field>
      </Section>

      <Section title="Visuals">
        <Field label="Visual Component">
          <TextInput value={data.visual_component} onChange={(v) => set('visual_component', v || null)} />
        </Field>
      </Section>

      <Section title="Tech">
        <Field label="Applied Science ID">
          <NumberInput value={data.applied_science_id} onChange={(v) => setNum('applied_science_id', v)} />
        </Field>
        <Field label="Tech Comp">
          <NumberInput value={data.tech_comp} onChange={(v) => setNum('tech_comp', v)} />
        </Field>
      </Section>

      <Section title="Arrays">
        <Field label="Container Sets">
          <ArrayInput value={data.container_sets} onChange={(v) => set('container_sets', v)} />
        </Field>
        <Field label="Moniker IDs">
          <ArrayInput value={data.moniker_ids} onChange={(v) => set('moniker_ids', v)} />
        </Field>
        <Field label="Discipline IDs">
          <ArrayInput value={data.discipline_ids} onChange={(v) => set('discipline_ids', v)} />
        </Field>
        <Field label="Ammo Types">
          <ArrayInput value={data.ammo_types} onChange={(v) => set('ammo_types', v)} />
        </Field>
      </Section>
    </div>
  );
}
