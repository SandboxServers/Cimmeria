---
name: multi-chain-dispatch-semantics
description: resolve_event aggregates EVERY matching chain for one event with conditions frozen at resolve time — cross-chain mutual exclusion must be authored into conditions; priority does not stop later chains.
metadata:
  type: project
---

# One event fires EVERY matching chain (verified 2026-09-18)

`ChainEngine::resolve_event` (`crates/content-engine/src/chain/mod.rs`, the
`impl` block after `ResolvedActions`) loops **all** chains registered for the
event's `TriggerType`, evaluates each chain's conditions, and **appends** the
actions of every chain that passes into one flat `ResolvedActions.actions`.
There is no first-match break.

Two consequences that bite chain authors:

1. **`priority` only orders; it never excludes.** `register_chain` sorts the
   per-trigger bucket by `Reverse(priority)`. A higher-priority chain matching
   does not suppress a lower-priority sibling.

2. **Conditions are frozen at resolve time.** All chains' conditions are
   evaluated in `resolve_event`, *before* the executor runs a single action
   (`executor::execute_actions` is called afterwards, with the resolved list).
   So a `complete_mission` / `advance_step` in chain A **cannot** gate chain B
   in the same event. There are no per-action conditions: each chain's full
   condition set is evaluated during `resolve_event`, before any resolved
   action executes. Nested lifecycle dispatch can still observe mutations
   mid-list.

**Therefore:** when N chains share a trigger key (e.g. three chains on
`interact_tag 'CmdCenter_Marsh'`), their condition sets must be **pairwise
disjoint by construction**. Enumerate the reachable state cross-product and add
explicit negative gates (`step_status <other mission>/<step> neq active`,
`mission_status X neq active`) until no two can be simultaneously true.

Symptom when you get it wrong: two `display_dialog` actions in one right-click.
`send_dialog_display` re-pins `open_dialog_id` each time, so the **last** dialog
wins the #479 gate and the earlier one is unreadable — while its chain's
side effects (remove_item, complete_mission) still ran.

## Shared-hub corollary

This is why a shared world (Harset 57 / Harset_CmdCenter 68) with several
concurrent missions on one quest-giver NPC is far more dangerous than an
instanced zone: every mission that can be simultaneously live on that NPC is a
row in the cross-product.
