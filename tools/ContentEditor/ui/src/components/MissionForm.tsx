import { useState, useCallback } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Plus,
  Trash2,
} from 'lucide-react';
import type {
  Mission,
  MissionStep,
  MissionObjective,
  MissionTask,
  MissionRewardGroup,
  MissionReward,
} from '../editors/dataTypes';
import { Section, Field, TextInput, NumberInput, Checkbox } from './EntityTemplateForm';

interface MissionFormProps {
  data: Mission;
  onChange: (updated: Mission) => void;
}

export default function MissionForm({ data, onChange }: MissionFormProps) {
  const set = useCallback(
    <K extends keyof Mission>(key: K, value: Mission[K]) => {
      onChange({ ...data, [key]: value });
    },
    [data, onChange],
  );

  const setNum = useCallback(
    (key: keyof Mission, raw: string) => {
      const v = raw.trim() === '' ? null : Number(raw);
      set(key, (v != null && isNaN(v) ? null : v) as Mission[keyof Mission]);
    },
    [set],
  );

  const updateStep = useCallback(
    (idx: number, step: MissionStep) => {
      const steps = [...data.steps];
      steps[idx] = step;
      onChange({ ...data, steps });
    },
    [data, onChange],
  );

  const addStep = useCallback(() => {
    const nextId = data.steps.length > 0
      ? Math.max(...data.steps.map((s) => s.step_id)) + 1
      : 1;
    const nextIndex = data.steps.length > 0
      ? Math.max(...data.steps.map((s) => s.index)) + 1
      : 0;
    onChange({
      ...data,
      steps: [
        ...data.steps,
        {
          step_id: nextId,
          mission_id: data.mission_id,
          index: nextIndex,
          award_xp: null,
          difficulty: null,
          step_enabled: true,
          step_display_log_text: null,
          objectives: [],
        },
      ],
    });
  }, [data, onChange]);

  const removeStep = useCallback(
    (idx: number) => {
      onChange({ ...data, steps: data.steps.filter((_, i) => i !== idx) });
    },
    [data, onChange],
  );

  const updateRewardGroup = useCallback(
    (idx: number, group: MissionRewardGroup) => {
      const groups = [...data.reward_groups];
      groups[idx] = group;
      onChange({ ...data, reward_groups: groups });
    },
    [data, onChange],
  );

  const addRewardGroup = useCallback(() => {
    const nextId = data.reward_groups.length > 0
      ? Math.max(...data.reward_groups.map((g) => g.group_id)) + 1
      : 1;
    onChange({
      ...data,
      reward_groups: [
        ...data.reward_groups,
        {
          group_id: nextId,
          mission_id: data.mission_id,
          choices: 1,
          rewards: [],
        },
      ],
    });
  }, [data, onChange]);

  const removeRewardGroup = useCallback(
    (idx: number) => {
      onChange({ ...data, reward_groups: data.reward_groups.filter((_, i) => i !== idx) });
    },
    [data, onChange],
  );

  return (
    <div className="subtle-scrollbar flex flex-col gap-1 overflow-y-auto p-3">
      <Section title="Mission Header" defaultOpen>
        <Field label="Mission ID">
          <NumberInput value={data.mission_id} disabled />
        </Field>
        <Field label="Label">
          <TextInput value={data.mission_label} onChange={(v) => set('mission_label', v || null)} />
        </Field>
        <Field label="Definition">
          <textarea
            value={data.mission_defn ?? ''}
            onChange={(e) => set('mission_defn', e.target.value || null)}
            rows={2}
            className="w-full rounded border border-border bg-input px-1.5 py-0.5 text-xs text-foreground focus:border-primary focus:outline-none"
          />
        </Field>
        <Field label="History Text">
          <textarea
            value={data.history_text ?? ''}
            onChange={(e) => set('history_text', e.target.value || null)}
            rows={2}
            className="w-full rounded border border-border bg-input px-1.5 py-0.5 text-xs text-foreground focus:border-primary focus:outline-none"
          />
        </Field>
        <Field label="Level">
          <NumberInput value={data.level} onChange={(v) => setNum('level', v)} />
        </Field>
        <Field label="Difficulty">
          <NumberInput value={data.difficulty} onChange={(v) => setNum('difficulty', v)} />
        </Field>
        <Field label="Script Name">
          <TextInput value={data.script_name} onChange={(v) => set('script_name', v || null)} />
        </Field>
        <Field label="Script Spaces">
          <TextInput value={data.script_spaces} onChange={(v) => set('script_spaces', v || null)} />
        </Field>
        <Field label="Num Repeats">
          <NumberInput value={data.num_repeats} onChange={(v) => setNum('num_repeats', v)} />
        </Field>
      </Section>

      <Section title="Flags" defaultOpen>
        <Field label="Enabled">
          <Checkbox checked={data.is_enabled} onChange={(v) => set('is_enabled', v)} />
        </Field>
        <Field label="Story">
          <Checkbox checked={data.is_a_story} onChange={(v) => set('is_a_story', v)} />
        </Field>
        <Field label="Hidden">
          <Checkbox checked={data.is_hidden} onChange={(v) => set('is_hidden', v)} />
        </Field>
        <Field label="Shareable">
          <Checkbox checked={data.is_shareable} onChange={(v) => set('is_shareable', v)} />
        </Field>
        <Field label="Can Abandon">
          <Checkbox checked={data.can_abandon} onChange={(v) => set('can_abandon', v)} />
        </Field>
        <Field label="Can Fail">
          <Checkbox checked={data.can_fail} onChange={(v) => set('can_fail', v)} />
        </Field>
        <Field label="Can Repeat on Fail">
          <Checkbox checked={data.can_repeat_on_fail} onChange={(v) => set('can_repeat_on_fail', v)} />
        </Field>
        <Field label="Override Mission">
          <Checkbox checked={data.is_override_mission} onChange={(v) => set('is_override_mission', v)} />
        </Field>
        <Field label="Faction Change Icon">
          <Checkbox checked={data.show_faction_change_icon} onChange={(v) => set('show_faction_change_icon', v)} />
        </Field>
        <Field label="Instance Icon">
          <Checkbox checked={data.show_instance_icon} onChange={(v) => set('show_instance_icon', v)} />
        </Field>
        <Field label="PvP Icon">
          <Checkbox checked={data.show_pvp_icon} onChange={(v) => set('show_pvp_icon', v)} />
        </Field>
      </Section>

      <Section title="Rewards">
        <Field label="Award XP">
          <NumberInput value={data.award_xp} onChange={(v) => setNum('award_xp', v)} />
        </Field>
        <Field label="Reward XP">
          <NumberInput value={data.reward_xp} onChange={(v) => setNum('reward_xp', v)} />
        </Field>
        <Field label="Reward NAQ">
          <NumberInput value={data.reward_naq} onChange={(v) => setNum('reward_naq', v)} />
        </Field>
      </Section>

      {/* Steps */}
      <div className="rounded border border-border/50">
        <div className="flex items-center justify-between px-2 py-1.5">
          <span className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
            Steps ({data.steps.length})
          </span>
          <button
            type="button"
            onClick={addStep}
            className="flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] text-primary hover:bg-primary/10"
          >
            <Plus className="h-3 w-3" /> Add Step
          </button>
        </div>
        <div className="flex flex-col gap-1 border-t border-border/30 px-1 pb-1 pt-1">
          {data.steps.map((step, si) => (
            <StepEditor
              key={step.step_id}
              step={step}
              index={si}
              onChange={(s) => updateStep(si, s)}
              onRemove={() => removeStep(si)}
            />
          ))}
          {data.steps.length === 0 && (
            <div className="px-2 py-2 text-center text-[11px] text-muted-foreground">
              No steps defined
            </div>
          )}
        </div>
      </div>

      {/* Reward Groups */}
      <div className="rounded border border-border/50">
        <div className="flex items-center justify-between px-2 py-1.5">
          <span className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
            Reward Groups ({data.reward_groups.length})
          </span>
          <button
            type="button"
            onClick={addRewardGroup}
            className="flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] text-primary hover:bg-primary/10"
          >
            <Plus className="h-3 w-3" /> Add Group
          </button>
        </div>
        <div className="flex flex-col gap-1 border-t border-border/30 px-1 pb-1 pt-1">
          {data.reward_groups.map((group, gi) => (
            <RewardGroupEditor
              key={group.group_id}
              group={group}
              onChange={(g) => updateRewardGroup(gi, g)}
              onRemove={() => removeRewardGroup(gi)}
            />
          ))}
          {data.reward_groups.length === 0 && (
            <div className="px-2 py-2 text-center text-[11px] text-muted-foreground">
              No reward groups defined
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

/* ---- Step Editor ---- */

function StepEditor({
  step,
  index,
  onChange,
  onRemove,
}: {
  step: MissionStep;
  index: number;
  onChange: (s: MissionStep) => void;
  onRemove: () => void;
}) {
  const [expanded, setExpanded] = useState(false);

  const setNum = (key: keyof MissionStep, raw: string) => {
    const v = raw.trim() === '' ? null : Number(raw);
    onChange({ ...step, [key]: v != null && isNaN(v) ? null : v });
  };

  const updateObjective = (idx: number, obj: MissionObjective) => {
    const objectives = [...step.objectives];
    objectives[idx] = obj;
    onChange({ ...step, objectives });
  };

  const addObjective = () => {
    const nextId = step.objectives.length > 0
      ? Math.max(...step.objectives.map((o) => o.objective_id)) + 1
      : 1;
    onChange({
      ...step,
      objectives: [
        ...step.objectives,
        {
          objective_id: nextId,
          step_id: step.step_id,
          award_xp: null,
          difficulty: null,
          is_enabled: true,
          is_hidden: false,
          is_optional: false,
          display_log_text: null,
          tasks: [],
        },
      ],
    });
  };

  const removeObjective = (idx: number) => {
    onChange({ ...step, objectives: step.objectives.filter((_, i) => i !== idx) });
  };

  return (
    <div className="rounded border border-border/40 bg-card/50">
      <div className="flex items-center gap-1 px-1.5 py-1">
        <button type="button" onClick={() => setExpanded((e) => !e)} className="text-muted-foreground">
          {expanded ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
        </button>
        <span className="flex-1 text-[11px] font-medium text-foreground">
          Step {index} (ID: {step.step_id})
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
            <Field label="Index">
              <NumberInput value={step.index} onChange={(v) => setNum('index', v)} />
            </Field>
            <Field label="Award XP">
              <NumberInput value={step.award_xp} onChange={(v) => setNum('award_xp', v)} />
            </Field>
            <Field label="Difficulty">
              <NumberInput value={step.difficulty} onChange={(v) => setNum('difficulty', v)} />
            </Field>
            <Field label="Enabled">
              <Checkbox checked={step.step_enabled} onChange={(v) => onChange({ ...step, step_enabled: v })} />
            </Field>
            <Field label="Log Text">
              <TextInput
                value={step.step_display_log_text}
                onChange={(v) => onChange({ ...step, step_display_log_text: v || null })}
              />
            </Field>
          </div>

          {/* Objectives */}
          <div className="mt-2 rounded border border-border/30">
            <div className="flex items-center justify-between px-2 py-1">
              <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                Objectives ({step.objectives.length})
              </span>
              <button
                type="button"
                onClick={addObjective}
                className="flex items-center gap-0.5 text-[10px] text-primary hover:bg-primary/10 rounded px-1 py-0.5"
              >
                <Plus className="h-2.5 w-2.5" /> Add
              </button>
            </div>
            {step.objectives.map((obj, oi) => (
              <ObjectiveEditor
                key={obj.objective_id}
                objective={obj}
                onChange={(o) => updateObjective(oi, o)}
                onRemove={() => removeObjective(oi)}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

/* ---- Objective Editor ---- */

function ObjectiveEditor({
  objective,
  onChange,
  onRemove,
}: {
  objective: MissionObjective;
  onChange: (o: MissionObjective) => void;
  onRemove: () => void;
}) {
  const [expanded, setExpanded] = useState(false);

  const setNum = (key: keyof MissionObjective, raw: string) => {
    const v = raw.trim() === '' ? null : Number(raw);
    onChange({ ...objective, [key]: v != null && isNaN(v) ? null : v });
  };

  const updateTask = (idx: number, task: MissionTask) => {
    const tasks = [...objective.tasks];
    tasks[idx] = task;
    onChange({ ...objective, tasks });
  };

  const addTask = () => {
    const nextId = objective.tasks.length > 0
      ? Math.max(...objective.tasks.map((t) => t.task_id)) + 1
      : 1;
    onChange({
      ...objective,
      tasks: [
        ...objective.tasks,
        {
          task_id: nextId,
          objective_id: objective.objective_id,
          award_xp: null,
          difficulty: null,
          is_enabled: true,
          task_type: null,
        },
      ],
    });
  };

  const removeTask = (idx: number) => {
    onChange({ ...objective, tasks: objective.tasks.filter((_, i) => i !== idx) });
  };

  return (
    <div className="mx-1 mb-1 rounded border border-border/30 bg-background/30">
      <div className="flex items-center gap-1 px-1.5 py-0.5">
        <button type="button" onClick={() => setExpanded((e) => !e)} className="text-muted-foreground">
          {expanded ? <ChevronDown className="h-2.5 w-2.5" /> : <ChevronRight className="h-2.5 w-2.5" />}
        </button>
        <span className="flex-1 text-[10px] text-foreground">
          Objective {objective.objective_id}
        </span>
        <button
          type="button"
          onClick={onRemove}
          className="rounded p-0.5 text-destructive hover:bg-destructive/10"
        >
          <Trash2 className="h-2.5 w-2.5" />
        </button>
      </div>
      {expanded && (
        <div className="border-t border-border/20 px-2 pb-1.5 pt-1">
          <div className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1">
            <Field label="Award XP">
              <NumberInput value={objective.award_xp} onChange={(v) => setNum('award_xp', v)} />
            </Field>
            <Field label="Difficulty">
              <NumberInput value={objective.difficulty} onChange={(v) => setNum('difficulty', v)} />
            </Field>
            <Field label="Enabled">
              <Checkbox checked={objective.is_enabled} onChange={(v) => onChange({ ...objective, is_enabled: v })} />
            </Field>
            <Field label="Hidden">
              <Checkbox checked={objective.is_hidden} onChange={(v) => onChange({ ...objective, is_hidden: v })} />
            </Field>
            <Field label="Optional">
              <Checkbox checked={objective.is_optional} onChange={(v) => onChange({ ...objective, is_optional: v })} />
            </Field>
            <Field label="Log Text">
              <TextInput
                value={objective.display_log_text}
                onChange={(v) => onChange({ ...objective, display_log_text: v || null })}
              />
            </Field>
          </div>

          {/* Tasks */}
          <div className="mt-1.5 rounded border border-border/20">
            <div className="flex items-center justify-between px-2 py-0.5">
              <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                Tasks ({objective.tasks.length})
              </span>
              <button
                type="button"
                onClick={addTask}
                className="flex items-center gap-0.5 text-[10px] text-primary hover:bg-primary/10 rounded px-1 py-0.5"
              >
                <Plus className="h-2.5 w-2.5" /> Add
              </button>
            </div>
            {objective.tasks.map((task, ti) => (
              <TaskEditor
                key={task.task_id}
                task={task}
                onChange={(t) => updateTask(ti, t)}
                onRemove={() => removeTask(ti)}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

/* ---- Task Editor ---- */

function TaskEditor({
  task,
  onChange,
  onRemove,
}: {
  task: MissionTask;
  onChange: (t: MissionTask) => void;
  onRemove: () => void;
}) {
  const setNum = (key: keyof MissionTask, raw: string) => {
    const v = raw.trim() === '' ? null : Number(raw);
    onChange({ ...task, [key]: v != null && isNaN(v) ? null : v });
  };

  return (
    <div className="mx-1 mb-0.5 flex flex-wrap items-center gap-2 rounded border border-border/20 bg-card/30 px-2 py-1">
      <span className="text-[10px] text-muted-foreground">Task {task.task_id}</span>
      <label className="flex items-center gap-1 text-[10px] text-muted-foreground">
        Type
        <input
          type="text"
          value={task.task_type ?? ''}
          onChange={(e) => onChange({ ...task, task_type: e.target.value || null })}
          className="w-20 rounded border border-border bg-input px-1 py-0 text-[10px] text-foreground focus:border-primary focus:outline-none"
        />
      </label>
      <label className="flex items-center gap-1 text-[10px] text-muted-foreground">
        XP
        <input
          type="number"
          value={task.award_xp ?? ''}
          onChange={(e) => setNum('award_xp', e.target.value)}
          className="w-14 rounded border border-border bg-input px-1 py-0 text-[10px] text-foreground focus:border-primary focus:outline-none"
        />
      </label>
      <label className="flex items-center gap-1 text-[10px] text-muted-foreground">
        Diff
        <input
          type="number"
          value={task.difficulty ?? ''}
          onChange={(e) => setNum('difficulty', e.target.value)}
          className="w-14 rounded border border-border bg-input px-1 py-0 text-[10px] text-foreground focus:border-primary focus:outline-none"
        />
      </label>
      <Checkbox checked={task.is_enabled} onChange={(v) => onChange({ ...task, is_enabled: v })} />
      <button
        type="button"
        onClick={onRemove}
        className="ml-auto rounded p-0.5 text-destructive hover:bg-destructive/10"
      >
        <Trash2 className="h-2.5 w-2.5" />
      </button>
    </div>
  );
}

/* ---- Reward Group Editor ---- */

function RewardGroupEditor({
  group,
  onChange,
  onRemove,
}: {
  group: MissionRewardGroup;
  onChange: (g: MissionRewardGroup) => void;
  onRemove: () => void;
}) {
  const [expanded, setExpanded] = useState(false);

  const addReward = () => {
    const nextId = group.rewards.length > 0
      ? Math.max(...group.rewards.map((r) => r.reward_id)) + 1
      : 1;
    onChange({
      ...group,
      rewards: [
        ...group.rewards,
        { reward_id: nextId, group_id: group.group_id, item_id: 0 },
      ],
    });
  };

  const removeReward = (idx: number) => {
    onChange({ ...group, rewards: group.rewards.filter((_, i) => i !== idx) });
  };

  const updateReward = (idx: number, reward: MissionReward) => {
    const rewards = [...group.rewards];
    rewards[idx] = reward;
    onChange({ ...group, rewards });
  };

  return (
    <div className="rounded border border-border/40 bg-card/50">
      <div className="flex items-center gap-1 px-1.5 py-1">
        <button type="button" onClick={() => setExpanded((e) => !e)} className="text-muted-foreground">
          {expanded ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
        </button>
        <span className="flex-1 text-[11px] font-medium text-foreground">
          Group {group.group_id} ({group.rewards.length} rewards, {group.choices} choice{group.choices !== 1 ? 's' : ''})
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
          <div className="mb-2 flex items-center gap-2">
            <label className="text-[11px] text-muted-foreground">Choices</label>
            <input
              type="number"
              value={group.choices}
              onChange={(e) => onChange({ ...group, choices: Number(e.target.value) || 0 })}
              className="w-16 rounded border border-border bg-input px-1.5 py-0.5 text-xs text-foreground focus:border-primary focus:outline-none"
            />
          </div>

          <div className="flex flex-col gap-0.5">
            {group.rewards.map((reward, ri) => (
              <div
                key={reward.reward_id}
                className="flex items-center gap-2 rounded border border-border/20 bg-background/30 px-2 py-1"
              >
                <span className="text-[10px] text-muted-foreground">#{reward.reward_id}</span>
                <label className="flex items-center gap-1 text-[10px] text-muted-foreground">
                  Item ID
                  <input
                    type="number"
                    value={reward.item_id}
                    onChange={(e) =>
                      updateReward(ri, { ...reward, item_id: Number(e.target.value) || 0 })
                    }
                    className="w-20 rounded border border-border bg-input px-1 py-0 text-[10px] text-foreground focus:border-primary focus:outline-none"
                  />
                </label>
                <button
                  type="button"
                  onClick={() => removeReward(ri)}
                  className="ml-auto rounded p-0.5 text-destructive hover:bg-destructive/10"
                >
                  <Trash2 className="h-2.5 w-2.5" />
                </button>
              </div>
            ))}
          </div>

          <button
            type="button"
            onClick={addReward}
            className="mt-1 flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] text-primary hover:bg-primary/10"
          >
            <Plus className="h-2.5 w-2.5" /> Add Reward
          </button>
        </div>
      )}
    </div>
  );
}
