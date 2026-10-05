---
name: rule6-sweep-scanner-and-naming-traps
description: NT-03 unpaired-ID scan blind spots ($level! macro rows), how to list every unpaired site, and naming traps (reused entity slots, entity ids logged as player_id) found by the NT-20 combat sweep
metadata:
  type: project
---

Facts from the NT-20 combat/effects Rule 6 sweep (2026-10-04).

**Listing sites.** `unpaired_id_report` prints only counts. To get every `file:line key`,
set the files' rows in `crates/server/src/logging/unpaired_id_baseline.txt` to 0, fix the
`# total` line to the new row sum (the guard checks it first), run
`unpaired_id_fields_only_shrink --no-capture`, and restore the file. The "rose" message
lists every site. The baseline is read at runtime, so no rebuild.

**Scanner blind spot.** Rows emitted through `macro_rules! row { ($level:ident) =>
tracing::$level!(...) }` (gate rows in `use_ability/gate_rows.rs`, `log_nothing_removed`
in `stat_buff/ledger.rs`) are not event calls to the scan, so they never show as unpaired.
Pair them by hand; the launch-refusal row is one of the most-read combat rows.

**Naming traps.**
- A deferred effect row (pulse, expiry, removal, script run under `enter_effect_scope`)
  must not name its invoker with a live `entity_label(invoker_id)`: the invoker may have
  left and the id been reused. Use `SpaceManager::caster_label` or the entry's
  `invoker_identity.player_name`.
- Pet rows: `owner_name` from the summon-time identity, never the live `owner_id` (#889).
- Rows logged an entity id under `player_id` (`threat` enter/exit_combat, and npc_ai rows
  NT-25 owned). Rename the key, then pair.
- A logged item *type* is `item_type_id` (+ `item_name`); `item_id` is reserved for instances.
- The NameBook has `loot_table`, `event_set`, `sequence` and `chain` lookups (NT-21): don't
  mark those IDs unnameable.
- The wire `onEffectResults` EffectID is the cast_id, not an `effects` row: don't name it.
- Rows naming an ability `"unknown"` violated Rule 6; pass `Option<&str>`.

**Hot-path rule in practice.** tracing evaluates field expressions only when the event is
enabled, but `OTEL_FILTER` exports every cimmeria crate and `npc_ai`/`abilities` at DEBUG
and the file layers write some at TRACE, so in practice debug rows are on: a lookup in a
per-tick row (`npc_ai` `decision`, `npc_ai.tick`, `warmup_pending`, per-witness rows) is
paid every tick. Name them from fields already in hand (`identity()`, `log_names`), or mark
the ID `// nt:id-only` with the reason; don't leave it silently in the baseline. Where a
row logs while the target is `&mut`-borrowed (NVP damage, absorb), resolve once per hit
into the ids struct (`HitIds::entity_name`) or use `EntityNames::of(entity)`.

Related: [[ability-row-event-guard-and-cast-join]], [[tracing-span-fields-not-on-log-records]].
