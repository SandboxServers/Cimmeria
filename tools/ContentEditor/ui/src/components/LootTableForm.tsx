import { useCallback } from 'react';
import { Plus, Trash2, AlertTriangle } from 'lucide-react';
import type { LootTable, LootEntry } from '../editors/dataTypes';
import { Section, Field, NumberInput } from './EntityTemplateForm';

interface LootTableFormProps {
  data: LootTable;
  onChange: (updated: LootTable) => void;
}

export default function LootTableForm({ data, onChange }: LootTableFormProps) {
  const set = useCallback(
    <K extends keyof LootTable>(key: K, value: LootTable[K]) => {
      onChange({ ...data, [key]: value });
    },
    [data, onChange],
  );

  const updateEntry = useCallback(
    (idx: number, entry: LootEntry) => {
      const entries = [...data.entries];
      entries[idx] = entry;
      onChange({ ...data, entries });
    },
    [data, onChange],
  );

  const addEntry = useCallback(() => {
    const nextId = data.entries.length > 0
      ? Math.max(...data.entries.map((e) => e.loot_id)) + 1
      : 1;
    onChange({
      ...data,
      entries: [
        ...data.entries,
        {
          loot_id: nextId,
          loot_table_id: data.loot_table_id,
          design_id: 0,
          min_quantity: 1,
          max_quantity: 1,
          probability: 0,
        },
      ],
    });
  }, [data, onChange]);

  const removeEntry = useCallback(
    (idx: number) => {
      onChange({ ...data, entries: data.entries.filter((_, i) => i !== idx) });
    },
    [data, onChange],
  );

  const probSum = data.entries.reduce((sum, e) => sum + e.probability, 0);
  const probValid = Math.abs(probSum - 1.0) < 0.001 || data.entries.length === 0;

  return (
    <div className="subtle-scrollbar flex flex-col gap-1 overflow-y-auto p-3">
      <Section title="Loot Table" defaultOpen>
        <Field label="Table ID">
          <NumberInput value={data.loot_table_id} disabled />
        </Field>
        <Field label="Description">
          <textarea
            value={data.description ?? ''}
            onChange={(e) => set('description', (e.target.value || null) as LootTable['description'])}
            rows={2}
            className="w-full rounded border border-border bg-input px-1.5 py-0.5 text-xs text-foreground focus:border-primary focus:outline-none"
          />
        </Field>
      </Section>

      {/* Entries */}
      <div className="rounded border border-border/50">
        <div className="flex items-center justify-between px-2 py-1.5">
          <span className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
            Entries ({data.entries.length})
          </span>
          <button
            type="button"
            onClick={addEntry}
            className="flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] text-primary hover:bg-primary/10"
          >
            <Plus className="h-3 w-3" /> Add Entry
          </button>
        </div>

        {/* Probability summary */}
        <div
          className={`flex items-center gap-1.5 border-t border-border/30 px-2 py-1 text-[10px] ${
            probValid ? 'text-muted-foreground' : 'text-destructive'
          }`}
        >
          {!probValid && <AlertTriangle className="h-3 w-3" />}
          <span>
            Total probability: {probSum.toFixed(4)}
            {!probValid && ' (should be ~1.0)'}
          </span>
        </div>

        {/* Entry table */}
        {data.entries.length > 0 && (
          <div className="border-t border-border/30">
            <table className="w-full text-xs">
              <thead>
                <tr className="text-left text-[10px] font-medium text-muted-foreground">
                  <th className="px-2 py-1">ID</th>
                  <th className="px-2 py-1">Design ID</th>
                  <th className="px-2 py-1">Min Qty</th>
                  <th className="px-2 py-1">Max Qty</th>
                  <th className="px-2 py-1">Probability</th>
                  <th className="w-8 px-1 py-1"></th>
                </tr>
              </thead>
              <tbody>
                {data.entries.map((entry, ei) => (
                  <LootEntryRow
                    key={entry.loot_id}
                    entry={entry}
                    onChange={(e) => updateEntry(ei, e)}
                    onRemove={() => removeEntry(ei)}
                  />
                ))}
              </tbody>
            </table>
          </div>
        )}

        {data.entries.length === 0 && (
          <div className="border-t border-border/30 px-2 py-4 text-center text-[11px] text-muted-foreground">
            No loot entries defined
          </div>
        )}
      </div>
    </div>
  );
}

function LootEntryRow({
  entry,
  onChange,
  onRemove,
}: {
  entry: LootEntry;
  onChange: (e: LootEntry) => void;
  onRemove: () => void;
}) {
  return (
    <tr className="border-t border-border/20">
      <td className="px-2 py-0.5 text-[10px] text-muted-foreground">{entry.loot_id}</td>
      <td className="px-1 py-0.5">
        <input
          type="number"
          value={entry.design_id}
          onChange={(e) => onChange({ ...entry, design_id: Number(e.target.value) || 0 })}
          className="w-full rounded border border-border bg-input px-1 py-0 text-[10px] text-foreground focus:border-primary focus:outline-none"
        />
      </td>
      <td className="px-1 py-0.5">
        <input
          type="number"
          value={entry.min_quantity}
          onChange={(e) => onChange({ ...entry, min_quantity: Number(e.target.value) || 0 })}
          className="w-full rounded border border-border bg-input px-1 py-0 text-[10px] text-foreground focus:border-primary focus:outline-none"
        />
      </td>
      <td className="px-1 py-0.5">
        <input
          type="number"
          value={entry.max_quantity}
          onChange={(e) => onChange({ ...entry, max_quantity: Number(e.target.value) || 0 })}
          className="w-full rounded border border-border bg-input px-1 py-0 text-[10px] text-foreground focus:border-primary focus:outline-none"
        />
      </td>
      <td className="px-1 py-0.5">
        <input
          type="number"
          value={entry.probability}
          step="0.01"
          onChange={(e) => onChange({ ...entry, probability: Number(e.target.value) || 0 })}
          className="w-full rounded border border-border bg-input px-1 py-0 text-[10px] text-foreground focus:border-primary focus:outline-none"
        />
      </td>
      <td className="px-1 py-0.5">
        <button
          type="button"
          onClick={onRemove}
          className="rounded p-0.5 text-destructive hover:bg-destructive/10"
        >
          <Trash2 className="h-3 w-3" />
        </button>
      </td>
    </tr>
  );
}
