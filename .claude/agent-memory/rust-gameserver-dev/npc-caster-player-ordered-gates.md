---
name: npc-caster-player-ordered-gates
description: An NPC entity casting on a player's order (pets, any future summon/command) skips the #444 target rule, fire_los and the warmup re-check unless the caller adds them; also the player useAbility launch has no same-space check.
metadata:
  type: project
---

`handle_use_ability` applies the #444 hostile-target rule to **player** casters only,
`fire_los::refuse_without_line_of_sight` returns `NotChecked("npc_shooter")` for every NPC,
and `warmup/tick.rs` `fire_time_refusal` re-checked the target rule for players only. All three
assume an NPC caster is AI-driven and that the fight tick already chose a legal, visible target.

**Why:** PT-04 (pet commands, 2026-09-27) let an owner direct an NPC-class pet through
`handle_use_ability_with_kill_credit`; the server-authority review found it could hit through
walls and hit targets that turned friendly during a warmup.

**How to apply:** any new path where a player's packet makes an NPC cast must pre-check
same space, hostility (incl. `is_pet`), range and `SpaceManager::attack_line_of_sight`, and
send refusals to the **player** (an `EntityMethodCall` on an NPC id reaches no client).
PT-04 added pet-only checks to the warmup tick (`warmup/pet_order.rs`: PT-05's
`npc_ai::pet::fight_refusal` against the live owner, plus a pet LOS check), and an owner order
engages through `engage_pet_target(.., PetEngagement::OwnerOrder)` only once the cast fires.
An `onErrorCode` alone is invisible (no Lua consumer, AT-E1): pair it with a `CHAN_FEEDBACK` line. Reviewer also flagged (unverified, not fixed): the player launch path has no
same-space target check, and `fire_los` lets other-space targets through, so an instanced
NPC at matching coordinates may be hittable. See [[interact-range-and-logcapture-traps]].
