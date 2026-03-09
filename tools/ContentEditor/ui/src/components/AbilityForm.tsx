import { useState, useCallback } from 'react';
import { ChevronDown, ChevronRight, Plus, Trash2 } from 'lucide-react';
import type { Ability, Effect, EffectNvp } from '../editors/dataTypes';
import {
  Section,
  Field,
  TextInput,
  NumberInput,
  Checkbox,
  ArrayInput,
} from './EntityTemplateForm';

interface AbilityFormProps {
  data: Ability;
  onChange: (updated: Ability) => void;
}

export default function AbilityForm({ data, onChange }: AbilityFormProps) {
  const set = useCallback(
    <K extends keyof Ability>(key: K, value: Ability[K]) => {
      onChange({ ...data, [key]: value });
    },
    [data, onChange],
  );

  const setNum = useCallback(
    (key: keyof Ability, raw: string) => {
      const v = raw.trim() === '' ? null : Number(raw);
      set(key, (v != null && isNaN(v) ? null : v) as Ability[keyof Ability]);
    },
    [set],
  );

  const updateEffect = useCallback(
    (idx: number, effect: Effect) => {
      const effects = [...data.effects];
      effects[idx] = effect;
      onChange({ ...data, effects });
    },
    [data, onChange],
  );

  const addEffect = useCallback(() => {
    const nextId = data.effects.length > 0
      ? Math.max(...data.effects.map((e) => e.effect_id)) + 1
      : 1;
    onChange({
      ...data,
      effects: [
        ...data.effects,
        {
          effect_id: nextId,
          ability_id: data.ability_id,
          name: null,
          effect_desc: null,
          delay: null,
          effect_sequence: null,
          flags: null,
          icon_location: null,
          pulse_count: null,
          pulse_duration: null,
          is_channeled: false,
          script_name: null,
          target_collection_method: null,
          target_collection_id: null,
          event_set_id: null,
          nvps: [],
        },
      ],
    });
  }, [data, onChange]);

  const removeEffect = useCallback(
    (idx: number) => {
      onChange({ ...data, effects: data.effects.filter((_, i) => i !== idx) });
    },
    [data, onChange],
  );

  return (
    <div className="subtle-scrollbar flex flex-col gap-1 overflow-y-auto p-3">
      <Section title="Identity" defaultOpen>
        <Field label="Ability ID">
          <NumberInput value={data.ability_id} disabled />
        </Field>
        <Field label="Name">
          <TextInput value={data.name} onChange={(v) => set('name', v || null)} />
        </Field>
        <Field label="Description">
          <textarea
            value={data.description ?? ''}
            onChange={(e) => set('description', e.target.value || null)}
            rows={2}
            className="w-full rounded border border-border bg-input px-1.5 py-0.5 text-xs text-foreground focus:border-primary focus:outline-none"
          />
        </Field>
        <Field label="Icon">
          <TextInput value={data.icon} onChange={(v) => set('icon', v || null)} />
        </Field>
      </Section>

      <Section title="Mechanics" defaultOpen>
        <Field label="Type ID">
          <NumberInput value={data.type_id} onChange={(v) => setNum('type_id', v)} />
        </Field>
        <Field label="Cooldown">
          <NumberInput value={data.cooldown} onChange={(v) => setNum('cooldown', v)} />
        </Field>
        <Field label="Warmup">
          <NumberInput value={data.warmup} onChange={(v) => setNum('warmup', v)} />
        </Field>
        <Field label="Flags">
          <NumberInput value={data.flags} onChange={(v) => setNum('flags', v)} />
        </Field>
        <Field label="Passive">
          <Checkbox checked={data.passive_yn} onChange={(v) => set('passive_yn', v)} />
        </Field>
        <Field label="Event Set ID">
          <NumberInput value={data.event_set_id} onChange={(v) => setNum('event_set_id', v)} />
        </Field>
      </Section>

      <Section title="Targeting">
        <Field label="Target Type ID">
          <NumberInput value={data.target_type_id} onChange={(v) => setNum('target_type_id', v)} />
        </Field>
        <Field label="Target Collection">
          <NumberInput
            value={data.target_collection_method}
            onChange={(v) => setNum('target_collection_method', v)}
          />
        </Field>
        <Field label="Is Ranged">
          <Checkbox checked={data.is_ranged} onChange={(v) => set('is_ranged', v)} />
        </Field>
        <Field label="Min Range">
          <NumberInput value={data.min_range} onChange={(v) => setNum('min_range', v)} />
        </Field>
        <Field label="Max Range">
          <NumberInput value={data.max_range} onChange={(v) => setNum('max_range', v)} />
        </Field>
      </Section>

      <Section title="Combat">
        <Field label="Threat Level ID">
          <NumberInput value={data.threat_level_id} onChange={(v) => setNum('threat_level_id', v)} />
        </Field>
        <Field label="Taunt Adjustment">
          <NumberInput value={data.taunt_adjustment} onChange={(v) => setNum('taunt_adjustment', v)} />
        </Field>
        <Field label="Velocity">
          <NumberInput value={data.velocity} onChange={(v) => setNum('velocity', v)} />
        </Field>
      </Section>

      <Section title="Training">
        <Field label="Training Cost">
          <NumberInput value={data.training_cost} onChange={(v) => setNum('training_cost', v)} />
        </Field>
        <Field label="Required Ammo">
          <NumberInput value={data.required_ammo} onChange={(v) => setNum('required_ammo', v)} />
        </Field>
      </Section>

      <Section title="References">
        <Field label="Effect IDs">
          <ArrayInput value={data.effect_ids} onChange={(v) => set('effect_ids', v)} />
        </Field>
        <Field label="Moniker IDs">
          <ArrayInput value={data.moniker_ids} onChange={(v) => set('moniker_ids', v)} />
        </Field>
      </Section>

      {/* Effects */}
      <div className="rounded border border-border/50">
        <div className="flex items-center justify-between px-2 py-1.5">
          <span className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
            Effects ({data.effects.length})
          </span>
          <button
            type="button"
            onClick={addEffect}
            className="flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] text-primary hover:bg-primary/10"
          >
            <Plus className="h-3 w-3" /> Add Effect
          </button>
        </div>
        <div className="flex flex-col gap-1 border-t border-border/30 px-1 pb-1 pt-1">
          {data.effects.map((effect, ei) => (
            <EffectEditor
              key={effect.effect_id}
              effect={effect}
              onChange={(e) => updateEffect(ei, e)}
              onRemove={() => removeEffect(ei)}
            />
          ))}
          {data.effects.length === 0 && (
            <div className="px-2 py-2 text-center text-[11px] text-muted-foreground">
              No effects defined
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

/* ---- Effect Editor ---- */

function EffectEditor({
  effect,
  onChange,
  onRemove,
}: {
  effect: Effect;
  onChange: (e: Effect) => void;
  onRemove: () => void;
}) {
  const [expanded, setExpanded] = useState(false);

  const setNum = (key: keyof Effect, raw: string) => {
    const v = raw.trim() === '' ? null : Number(raw);
    onChange({ ...effect, [key]: v != null && isNaN(v) ? null : v });
  };

  const updateNvp = (idx: number, nvp: EffectNvp) => {
    const nvps = [...effect.nvps];
    nvps[idx] = nvp;
    onChange({ ...effect, nvps });
  };

  const addNvp = () => {
    const nextId = effect.nvps.length > 0
      ? Math.max(...effect.nvps.map((n) => n.nvp_id)) + 1
      : 1;
    onChange({
      ...effect,
      nvps: [
        ...effect.nvps,
        { nvp_id: nextId, effect_id: effect.effect_id, name: '', value: '' },
      ],
    });
  };

  const removeNvp = (idx: number) => {
    onChange({ ...effect, nvps: effect.nvps.filter((_, i) => i !== idx) });
  };

  return (
    <div className="rounded border border-border/40 bg-card/50">
      <div className="flex items-center gap-1 px-1.5 py-1">
        <button type="button" onClick={() => setExpanded((e) => !e)} className="text-muted-foreground">
          {expanded ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
        </button>
        <span className="flex-1 text-[11px] font-medium text-foreground">
          Effect {effect.effect_id}: {effect.name ?? 'Unnamed'}
        </span>
        <button
          type="button"
          onClick={onRemove}
          className="rounded p-0.5 text-destructive hover:bg-destructive/10"
        >
          <Trash2 className="h-3 w-3" />
        </button>
      </div>
      {expanded && (
        <div className="border-t border-border/30 px-2 pb-2 pt-1">
          <div className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1">
            <Field label="Name">
              <TextInput value={effect.name} onChange={(v) => onChange({ ...effect, name: v || null })} />
            </Field>
            <Field label="Description">
              <TextInput value={effect.effect_desc} onChange={(v) => onChange({ ...effect, effect_desc: v || null })} />
            </Field>
            <Field label="Script Name">
              <TextInput value={effect.script_name} onChange={(v) => onChange({ ...effect, script_name: v || null })} />
            </Field>
            <Field label="Icon Location">
              <TextInput value={effect.icon_location} onChange={(v) => onChange({ ...effect, icon_location: v || null })} />
            </Field>
            <Field label="Delay">
              <NumberInput value={effect.delay} onChange={(v) => setNum('delay', v)} />
            </Field>
            <Field label="Sequence">
              <NumberInput value={effect.effect_sequence} onChange={(v) => setNum('effect_sequence', v)} />
            </Field>
            <Field label="Flags">
              <NumberInput value={effect.flags} onChange={(v) => setNum('flags', v)} />
            </Field>
            <Field label="Pulse Count">
              <NumberInput value={effect.pulse_count} onChange={(v) => setNum('pulse_count', v)} />
            </Field>
            <Field label="Pulse Duration">
              <NumberInput value={effect.pulse_duration} onChange={(v) => setNum('pulse_duration', v)} />
            </Field>
            <Field label="Channeled">
              <Checkbox checked={effect.is_channeled} onChange={(v) => onChange({ ...effect, is_channeled: v })} />
            </Field>
            <Field label="Target Method">
              <NumberInput
                value={effect.target_collection_method}
                onChange={(v) => setNum('target_collection_method', v)}
              />
            </Field>
            <Field label="Target ID">
              <NumberInput
                value={effect.target_collection_id}
                onChange={(v) => setNum('target_collection_id', v)}
              />
            </Field>
            <Field label="Event Set ID">
              <NumberInput value={effect.event_set_id} onChange={(v) => setNum('event_set_id', v)} />
            </Field>
          </div>

          {/* NVPs */}
          <div className="mt-2 rounded border border-border/30">
            <div className="flex items-center justify-between px-2 py-1">
              <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                Name-Value Pairs ({effect.nvps.length})
              </span>
              <button
                type="button"
                onClick={addNvp}
                className="flex items-center gap-0.5 rounded px-1 py-0.5 text-[10px] text-primary hover:bg-primary/10"
              >
                <Plus className="h-2.5 w-2.5" /> Add
              </button>
            </div>
            {effect.nvps.map((nvp, ni) => (
              <div
                key={nvp.nvp_id}
                className="flex items-center gap-2 border-t border-border/20 px-2 py-0.5"
              >
                <input
                  type="text"
                  value={nvp.name}
                  onChange={(e) => updateNvp(ni, { ...nvp, name: e.target.value })}
                  placeholder="Name"
                  className="w-1/3 rounded border border-border bg-input px-1 py-0 text-[10px] text-foreground placeholder:text-muted-foreground focus:border-primary focus:outline-none"
                />
                <input
                  type="text"
                  value={nvp.value}
                  onChange={(e) => updateNvp(ni, { ...nvp, value: e.target.value })}
                  placeholder="Value"
                  className="flex-1 rounded border border-border bg-input px-1 py-0 text-[10px] text-foreground placeholder:text-muted-foreground focus:border-primary focus:outline-none"
                />
                <button
                  type="button"
                  onClick={() => removeNvp(ni)}
                  className="rounded p-0.5 text-destructive hover:bg-destructive/10"
                >
                  <Trash2 className="h-2.5 w-2.5" />
                </button>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
