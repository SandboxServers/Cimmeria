---
name: submit-state-semantics
description: AiState::Submit is enum-recovered but behavior-invented; it is not terminal because multiple writers can exit it, and `aggression = 0` is the default value, not a non-hostile marker
metadata:
  type: project
---

## Submit is half-recovered

`AI_STATE_Submit = 10` is real, recovered data: `deprecated/python/Atrea/enums.py:238`,
and every Rust `AiState` discriminant matches the python enum 1:1
(`crates/entity/src/cell_entity/mod.rs:165-178`). **The behavior is entirely
invented.** Nothing in `deprecated/python/` ever writes `AI_STATE_Submit`;
`SGWMob.doAiAction` (`deprecated/python/cell/SGWMob.py:308-322`) dispatches only
`AI_STATE_Fighting` and `AI_STATE_Spawning`, with a literal
`# TODO: Add other states here!`. So Submit has the same evidence status as the
H04 30% threshold: name recovered, semantics designed.

## Submit is not terminal

Multiple non-test writers can move an NPC out of Submit; none guards on current state:

- `content/executor/world/mod.rs:224` (`set_npc_ai_state`) — intended exit
- `content/executor/world/mod.rs:120` (`set_npc_poi`) → Investigating (**preemptable**)
- `content/executor/world/mod.rs:189/191` (`set_follow_target`) → Follow/Idle (**preemptable**)
- `combat/state.rs:109` (`mark_npc_dead`) → Dead; nothing in the damage path reads `ai_state`
- `console/patrol.rs:296/350`, `console/net.rs:424/428`, `console/spawn/mod.rs:275` — GM
- `service/ticks/npc_respawn/mod.rs:228` → Idle, but only reachable via Dead

`AiState::Submit` is read in the NPC-AI snapshot filter and match arm.
No targeting or interaction code consults it; `generate_threat` consults it only
to prevent the transition to `Fighting`.

## generate_threat does NOT fully ignore a submitted NPC

`cell/combat/threat/aggro.rs:69-76` excludes Submit from `preemptable`, so the NPC
stays in Submit — but line 91 (`threat_list.entry(...) += amount`) is **outside**
that guard, and line 98 still calls `enter_player_combat`. So shooting a submitted
NPC re-arms the attacker's `BSF_InCombat` + `threatened_mobs` with an NPC that
will not enter Fighting or leash; player combat remains stale until a separate
cleanup path, such as NPC death or player respawn, removes it. `regen.rs:62` gates
regen on `threatened_mobs.is_empty()`.
Any Submit handler must re-run its player-side scrub whenever `threat_list` is
non-empty, not only on first entry.

## `aggression = 0` is the identity value, not a pacify marker

Rust `aggression: i32` is 0 = passive / >=1 = hostile-on-sight
(`crates/entity/src/cell_entity/entity_struct.rs:452-463`), defaults to 0 at
construction (`cell_entity/construction.rs:88`), is never seeded from
`entity_templates` or `spawnlist` (only from the `spawn_entity` chain param,
`space_manager/spawn.rs:147-157`), and is never broadcast on the wire.

This is a **different scale** from the recovered `EMobAggressionLevel`
(`entities/defs/enumerations.xml:437-446`): 1=HOSTILE, 2=SUSPICIOUS, 3=NEUTRAL,
4=FRIENDLY, 5=DEFAULT, with `SGWMob.def:88` `Aggression` CELL_PUBLIC default 3.
Python's attack-vs-interact decision is `aggression < AGGRESSION_NEUTRAL`
(`SGWPlayer.py:1164-1166`). `SGWMob.setAggression` broadcasts
`onEntityProperty(GENERICPROPERTY_MobAggression=6)`; `createOnClient` sends
`onAggressionOverrideUpdate` when an override already exists. Rust broadcasts
nothing. Rust `aggression = 4` still means hostile-on-sight; it has no graduated
semantics, whereas Python 4 means FRIENDLY.

Consequence: setting `aggression = 0` records nothing distinguishable from a
normal defensive mob. `ai_state == Submit` is the only durable pacify marker
today; the recovered client-visible one is the `MobAggression` property.

## Cover leaks on Submit

`release_for_entity` (`cell/cover/reservation.rs:93`, wrapper `cover/types.rs:205`)
is idempotent — returns `None` for an entity that never reserved
(`cover/tests.rs:220-222`). It is called on death (`abilities/death.rs:136-138`),
on threat-empty Idle reset (`npc_ai/fight.rs:136-138`), and on leash
(`npc_ai/fight.rs:195-197`). It is **not** called by `npc_ai_submit`, so a
submitted NPC leaks its slot for the life of the instance — same shape as the
dead-NPC leak that motivated the death-path call.

Related: [[npc-follow-state-gaps]], [[harset-zone-evidence]]
