---
name: content-engine-once-semantics
description: content_triggers.once fires once per player per cell-entity life since #802 (resets on relog and world change); it is not persisted. Persistent one-shots still need a condition.
metadata:
  type: project
---

# `once` is enforced per cell-entity life since #802 (corrected 2026-10-06)

`crates/cell-content/src/cell/content/executor/once_gate.rs`: a chain loaded
with `once` fires once per `(entity the actions run for, chain id)` and then
disarms until that cell entity is destroyed, so it re-arms on relog and on
every world change. A once-chain whose conditions fail is not recorded and
stays armed. Deferred actions do not re-enter the gate.

It is still **not persisted**. For "once per character" use a state that
survives: `mission_status` / `step_status`, the `tutorial_shown` condition
(`sgw_player_tutorials`, CS-03), `open_loot`'s `once_per_character`, or
`send_system_mail`'s persisted `cooldown_secs` (which answers a repeat firing
with a refusal chat line).

The section below is the pre-#802 analysis, kept for the history of why seed
comments that lean on `once` for re-loot protection are wrong.

## Before #802: `once` was dead code (2026-06)

The `once` boolean on `content_triggers` is loaded into `DbTriggerRow.once`
(`crates/content-engine/src/loader/mod.rs:58`, SELECTed in
`crates/cell-content/src/cell/content/engine_loader.rs:68`) but **dropped on the
floor**:

- `convert_trigger` (`crates/content-engine/src/loader/trigger.rs:8-106`) never
  reads `row.once` when building the `Trigger` enum.
- `Chain` (`crates/content-engine/src/chain.rs:25-48`) has no `once` field.
- `resolve_event` (`chain.rs:247-282`) has no fired-set, no per-player
  bookkeeping, no dedup. It re-fires every matching enabled chain whose
  conditions pass, on every event, every time.

**Implication:** `once=true` provides ZERO re-trigger / re-loot protection.
It is neither per-player, per-session, nor persisted. Any chain that must fire
"exactly once" MUST gate on a state change that flips a condition false —
typically `step_status` flipping to `completed` after an `advance_step`, or
`mission_status` flipping to `active`/`completed`.

Existing seed comments that say "re-loot guard: `once`..." are wrong if they
rely on the trigger column. The real guards in castle_cellblock_chains.sql are
all condition-based (step_status gate flips false post-advance).

If a chain has no advance_step to flip its own gate (e.g. a grant-only chain),
the re-fire guard must be an explicit state mutation that a *condition* keys on
— e.g. `set_interaction_type ~mask` to clear the body's search bit AND a
condition... but conditions can't read interaction_type. So the durable guard
is: route the one-shot through a step advance, OR add a counter/objective the
condition can read. Bit-clear alone does NOT stop the chain re-firing (it only
stops the *client* re-opening the dialog, which is usually enough in practice
since the dialog_open event won't fire if the body isn't clickable).
