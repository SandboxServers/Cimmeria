import { useState, useCallback } from 'react';
import { ChevronDown, ChevronRight } from 'lucide-react';
import type { EntityTemplate } from '../editors/dataTypes';

interface EntityTemplateFormProps {
  data: EntityTemplate;
  onChange: (updated: EntityTemplate) => void;
}

export default function EntityTemplateForm({ data, onChange }: EntityTemplateFormProps) {
  const set = useCallback(
    <K extends keyof EntityTemplate>(key: K, value: EntityTemplate[K]) => {
      onChange({ ...data, [key]: value });
    },
    [data, onChange],
  );

  const setNum = useCallback(
    (key: keyof EntityTemplate, raw: string) => {
      const v = raw.trim() === '' ? null : Number(raw);
      set(key, (v != null && isNaN(v) ? null : v) as EntityTemplate[keyof EntityTemplate]);
    },
    [set],
  );

  return (
    <div className="subtle-scrollbar flex flex-col gap-1 overflow-y-auto p-3">
      <Section title="Identity" defaultOpen>
        <Field label="Template ID">
          <NumberInput value={data.template_id} disabled />
        </Field>
        <Field label="Name">
          <TextInput value={data.name} onChange={(v) => set('name', v || null)} />
        </Field>
        <Field label="Template Name">
          <TextInput value={data.template_name} onChange={(v) => set('template_name', v || null)} />
        </Field>
        <Field label="Class">
          <TextInput value={data.class} onChange={(v) => set('class', v || null)} />
        </Field>
        <Field label="Name ID">
          <NumberInput value={data.name_id} onChange={(v) => setNum('name_id', v)} />
        </Field>
      </Section>

      <Section title="Stats" defaultOpen>
        <Field label="Level">
          <NumberInput value={data.level} onChange={(v) => setNum('level', v)} />
        </Field>
        <Field label="Alignment">
          <NumberInput value={data.alignment} onChange={(v) => setNum('alignment', v)} />
        </Field>
        <Field label="Faction">
          <NumberInput value={data.faction} onChange={(v) => setNum('faction', v)} />
        </Field>
        <Field label="Event Set ID">
          <NumberInput value={data.event_set_id} onChange={(v) => setNum('event_set_id', v)} />
        </Field>
      </Section>

      <Section title="Visual">
        <Field label="Body Set">
          <TextInput value={data.body_set} onChange={(v) => set('body_set', v || null)} />
        </Field>
        <Field label="Static Mesh">
          <TextInput value={data.static_mesh} onChange={(v) => set('static_mesh', v || null)} />
        </Field>
        <Field label="Primary Color ID">
          <NumberInput value={data.primary_color_id} onChange={(v) => setNum('primary_color_id', v)} />
        </Field>
        <Field label="Secondary Color ID">
          <NumberInput value={data.secondary_color_id} onChange={(v) => setNum('secondary_color_id', v)} />
        </Field>
        <Field label="Skin Tint">
          <NumberInput value={data.skin_tint} onChange={(v) => setNum('skin_tint', v)} />
        </Field>
      </Section>

      <Section title="Combat">
        <Field label="Ability Set ID">
          <NumberInput value={data.ability_set_id} onChange={(v) => setNum('ability_set_id', v)} />
        </Field>
        <Field label="Weapon Item ID">
          <NumberInput value={data.weapon_item_id} onChange={(v) => setNum('weapon_item_id', v)} />
        </Field>
        <Field label="Ammo Type">
          <NumberInput value={data.ammo_type} onChange={(v) => setNum('ammo_type', v)} />
        </Field>
        <Field label="Loot Table ID">
          <NumberInput value={data.loot_table_id} onChange={(v) => setNum('loot_table_id', v)} />
        </Field>
      </Section>

      <Section title="Commerce">
        <Field label="Buy Item List">
          <NumberInput value={data.buy_item_list} onChange={(v) => setNum('buy_item_list', v)} />
        </Field>
        <Field label="Sell Item List">
          <NumberInput value={data.sell_item_list} onChange={(v) => setNum('sell_item_list', v)} />
        </Field>
        <Field label="Repair Item List">
          <NumberInput value={data.repair_item_list} onChange={(v) => setNum('repair_item_list', v)} />
        </Field>
        <Field label="Recharge Item List">
          <NumberInput value={data.recharge_item_list} onChange={(v) => setNum('recharge_item_list', v)} />
        </Field>
      </Section>

      <Section title="Interaction">
        <Field label="Interaction Type">
          <NumberInput value={data.interaction_type} onChange={(v) => setNum('interaction_type', v)} />
        </Field>
        <Field label="Interaction Set ID">
          <NumberInput value={data.interaction_set_id} onChange={(v) => setNum('interaction_set_id', v)} />
        </Field>
        <Field label="Speaker ID">
          <NumberInput value={data.speaker_id} onChange={(v) => setNum('speaker_id', v)} />
        </Field>
        <Field label="Trainer Ability List ID">
          <NumberInput
            value={data.trainer_ability_list_id}
            onChange={(v) => setNum('trainer_ability_list_id', v)}
          />
        </Field>
        <Field label="Static Interaction Sets">
          <ArrayInput
            value={data.static_interaction_sets}
            onChange={(v) => set('static_interaction_sets', v)}
          />
        </Field>
      </Section>

      <Section title="Behavior">
        <Field label="Patrol Path ID">
          <NumberInput value={data.patrol_path_id} onChange={(v) => setNum('patrol_path_id', v)} />
        </Field>
        <Field label="Patrol Point Delay">
          <NumberInput value={data.patrol_point_delay} onChange={(v) => setNum('patrol_point_delay', v)} />
        </Field>
        <Field label="Has Dynamic Properties">
          <Checkbox
            checked={data.has_dynamic_properties}
            onChange={(v) => set('has_dynamic_properties', v)}
          />
        </Field>
        <Field label="Flags">
          <NumberInput value={data.flags} onChange={(v) => setNum('flags', v)} />
        </Field>
        <Field label="Components">
          <StringArrayInput
            value={data.components}
            onChange={(v) => set('components', v)}
          />
        </Field>
      </Section>
    </div>
  );
}

/* ---- Shared sub-components ---- */

function Section({
  title,
  defaultOpen = false,
  children,
}: {
  title: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <div className="rounded border border-border/50">
      <button
        type="button"
        className="flex w-full items-center gap-1.5 px-2 py-1.5 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground hover:text-foreground"
        onClick={() => setOpen((o) => !o)}
      >
        {open ? (
          <ChevronDown className="h-3 w-3" />
        ) : (
          <ChevronRight className="h-3 w-3" />
        )}
        {title}
      </button>
      {open && (
        <div className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 border-t border-border/30 px-2 pb-2 pt-1">
          {children}
        </div>
      )}
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <>
      <label className="self-center text-[11px] text-muted-foreground">{label}</label>
      <div>{children}</div>
    </>
  );
}

function TextInput({
  value,
  onChange,
  disabled,
}: {
  value: string | null;
  onChange?: (v: string) => void;
  disabled?: boolean;
}) {
  return (
    <input
      type="text"
      value={value ?? ''}
      onChange={onChange ? (e) => onChange(e.target.value) : undefined}
      disabled={disabled}
      className="w-full rounded border border-border bg-input px-1.5 py-0.5 text-xs text-foreground placeholder:text-muted-foreground focus:border-primary focus:outline-none disabled:opacity-50"
    />
  );
}

function NumberInput({
  value,
  onChange,
  disabled,
}: {
  value: number | null;
  onChange?: (v: string) => void;
  disabled?: boolean;
}) {
  return (
    <input
      type="number"
      value={value ?? ''}
      onChange={onChange ? (e) => onChange(e.target.value) : undefined}
      disabled={disabled}
      className="w-full rounded border border-border bg-input px-1.5 py-0.5 text-xs text-foreground placeholder:text-muted-foreground focus:border-primary focus:outline-none disabled:opacity-50"
    />
  );
}

function Checkbox({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <input
      type="checkbox"
      checked={checked}
      onChange={(e) => onChange(e.target.checked)}
      className="h-3.5 w-3.5 rounded border-border bg-input accent-primary"
    />
  );
}

function ArrayInput({
  value,
  onChange,
}: {
  value: number[];
  onChange: (v: number[]) => void;
}) {
  const raw = value.join(', ');
  return (
    <input
      type="text"
      value={raw}
      onChange={(e) => {
        const parts = e.target.value
          .split(',')
          .map((s) => s.trim())
          .filter((s) => s !== '')
          .map(Number)
          .filter((n) => !isNaN(n));
        onChange(parts);
      }}
      placeholder="Comma-separated IDs"
      className="w-full rounded border border-border bg-input px-1.5 py-0.5 text-xs text-foreground placeholder:text-muted-foreground focus:border-primary focus:outline-none"
    />
  );
}

function StringArrayInput({
  value,
  onChange,
}: {
  value: string[];
  onChange: (v: string[]) => void;
}) {
  const raw = value.join(', ');
  return (
    <input
      type="text"
      value={raw}
      onChange={(e) => {
        const parts = e.target.value
          .split(',')
          .map((s) => s.trim())
          .filter((s) => s !== '');
        onChange(parts);
      }}
      placeholder="Comma-separated values"
      className="w-full rounded border border-border bg-input px-1.5 py-0.5 text-xs text-foreground placeholder:text-muted-foreground focus:border-primary focus:outline-none"
    />
  );
}

export { Section, Field, TextInput, NumberInput, Checkbox, ArrayInput, StringArrayInput };
