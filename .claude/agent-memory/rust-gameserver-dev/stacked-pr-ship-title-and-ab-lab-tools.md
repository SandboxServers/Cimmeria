---
name: stacked-pr-ship-title-and-ab-lab-tools
description: ship.py on a stacked branch titles the PR after the base packet's commit; where the AB-T5 snapshot, AB-L1 lab query and AB-L2 dummy/cooldown tools live and their non-obvious rules
metadata:
  type: project
---

**ship.py on a stacked branch.** `ship.py pr` titles a new PR after the
branch's *first* commit not on main and uses that commit's body. On a branch
stacked on another open packet that is the base packet's commit, so fix the
title and body with `gh pr edit` straight after (2026-10-04, #1174). The PR
also opens against `main`, so its diff includes the base packet; say so in
the body.

**Ability state, one builder (AB-T5/L1/L2, 2026-10-04).**
- `CellEntity::ability_state(now)` in `cimmeria-entity` (`cell_entity/ability_state.rs`)
  is the only builder; `abilities.snapshot` row (`cell-world` `effects::ability_snapshot`),
  `LabQuery::AbilityState` and `.effects` all read it. `AbilityManager` exposes
  `ability_cooldowns()` / `moniker_cooldowns()` read iterators for it.
- `start_ability_cooldown` stamps its own `Instant::now()`: a test that builds a
  snapshot at a precomputed `now` must start cooldowns *before* taking `now`,
  and assert their remaining time within a tolerance.
- `entity` and `cell-combat` have no `serde_json` dev-dep; JSON round-trip tests
  live in `cell-world` / `lab-mcp`.

**Lab dummy (D-AU6).** A dummy is a template NPC carrying a `LabDummy`
extension (`cell-world` `space_manager/lab_dummy.rs`). The *only* thing that
stops it fighting back is `ai_driven_npc_entity_ids` skipping the mark: no
AI turn means no fight, chase, leash or movement (the retry sweep is fed only
by the fight pass). Expiry is a 1 Hz sweep in the cell message loop; the
owner's `DisconnectEntity` sweeps theirs. Gate travel does not.

**Cooldown clear bytes.** The client's hotbar sweep stops on
`onTimerUpdate(ability_id, type 2 TIMER_ABILITY_COOLDOWN, caster, 0, 0.0, 0.0)`
— the same zero timer the warmup interrupt sends when it refunds a cooldown.
Note `TIMER_ABILITY_COOLDOWN` is 2, not 0.

See [[witness-entity-method-dual-fn]] for the timer routing (owner only).
