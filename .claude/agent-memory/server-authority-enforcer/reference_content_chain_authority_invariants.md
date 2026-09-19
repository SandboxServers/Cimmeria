---
name: reference-content-chain-authority-invariants
description: Server-authority invariants and footguns for content-engine chains — step-gate single-grant, delay_ms breaking ordering guards, vacuous all_required_complete, entity_dead_tag authority
metadata:
  type: reference
---

# Content-chain server-authority invariants (verified 2026-09-18)

Established while reviewing the Castle 708 kill/dialog chains. These are
cross-cutting, not mission-708 specific.

## The step gate IS a valid single-grant guard — conditionally

Pattern: `trigger X` + `condition step_status M S eq active` + actions
`[add_item, complete_objective, advance_step M S+1]`. A second trigger
finds S no longer active and resolves nothing. This holds because:

- The cell service is a **single task owning `SpaceManager` by value**
  (`cell/service/message_loop.rs:23`); every `fire_*` dispatcher holds
  `&mut SpaceManager` across its whole await chain, so no packet, tick or
  disconnect can interleave between two actions of one chain.
- `advance_step` mutates `entity.missions` in place *synchronously*
  (`cell/missions/progression.rs:76`) before the cell→base persistence
  send. The in-memory step, not the DB, is what the next event's
  `populate_mission_context` reads.
- Each `fire_*` call builds a **fresh** `ExecutionContext`; `resolve_event`
  (`content-engine/src/chain/mod.rs:288-325`) evaluates conditions
  against that snapshot, so back-to-back events cannot both see S active.

**The guard breaks if `content_actions.delay_ms > 0` on the
`advance_step` row.** `executor/mod.rs:96-120` does not execute a delayed
action — it queues it to `schedule_content_action` for a later
`deferred_content_action_tick`. Step S stays active across that window and
every repeat trigger re-grants. Always assert `delay_ms = 0` on the
advancing action of a single-grant chain.

Note: action *order within a chain* is irrelevant to the guard — all
actions of a matched chain are resolved as one batch after conditions are
evaluated. Only **delay** matters.

## `advance_step` does NOT trip mission auto-complete

`missions/progression.rs:58-66` force-completes the old step's objectives
by calling `MissionInstance::complete_objective` (`entity/src/missions.rs:101`)
directly on the struct, bypassing the module-level `complete_objective`
that carries the `all_required_complete` check. Confirmed safe.

## Vacuous-truth footgun in `complete_objective`

`missions/progression.rs:176-180`: `all_required_complete` is `.all()`
over `active_objectives.filter(|o| !o.optional)`. **A step whose
`active_objectives` contains only optional objectives auto-completes the
whole mission on the first `complete_objective`.** Partially mitigated by
the early return at line 155-157 when the objective isn't in
`active_objectives`, but the invariant "every step has >=1 required
objective" is load-bearing and unasserted anywhere.

Corollary: a step with several *mutually exclusive* branch objectives all
marked `is_optional = false` can NEVER be completed by `complete_objective`
— the untaken branch stays active forever. Such steps need
`complete_mission` / `advance_step`, not `complete_objective`.

`mission_objectives.sql` is loaded with **no `is_enabled` filter**
(`cell/spawner/missions.rs:86-88, 128-131`), so `is_enabled = false` rows
still land in `active_objectives`.

## `entity_dead_tag` is genuinely server-authoritative

`entity_tag` is read from the victim's server-side `CellEntity::tag`, never
off the wire. Only client inputs on the kill path are `ability_id` and
`target_id`. Alive->dead is computed by re-reading HEALTH from server state
after damage resolution. Five call sites, all behind a server-computed
kill: `kill_credit.rs:102`/`:116`, `player/combat/mod.rs:94`,
`interaction/interact.rs:133`, the auto-cycle + pending-attack ticks.

Corpse re-fire is blocked by `was_alive_before`
(`abilities/use_ability/kill_credit.rs:56-69`) and, on the AoE path, by
the `alive_before` snapshot at `abilities/dispatch.rs:183-194`.

`npc_respawn_tick` (`cell/service/ticks/npc_respawn/mod.rs:116-165`)
resets the **same entity in place** — tag preserved, HP restored to max.
So a respawned NPC re-kill *does* re-fire `fire_entity_death`; the step
gate is the only thing stopping a second grant.

## Grant/advance durability asymmetry (not a dupe — the opposite)

`add_item` is fire-and-forget to base (`executor/inventory.rs:112-129`,
`tracing::error!` only on send failure) while `advance_step` commits
cell-side state immediately. If the base grant is lost, the player is past
the step with no item and the death chain can never re-fire — the mission
is unrecoverable. Affects every `add_item` + `advance_step` chain in the
repo.

Links: [[reference-dialog-choice-exploit-shape]] [[reference-authority-sources]]
