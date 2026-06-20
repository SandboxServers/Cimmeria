---
name: content-engine-once-semantics
description: The content_triggers.once column is dead code — loaded but never enforced. One-shot guards must come from conditions, not `once`.
metadata:
  type: project
---

# `once` is NOT enforced — confirmed dead code (2026-06)

The `once` boolean on `content_triggers` is loaded into `DbTriggerRow.once`
(`crates/content-engine/src/loader/mod.rs:58`, SELECTed in
`crates/services/src/cell/content/engine_loader.rs:68`) but **dropped on the
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
