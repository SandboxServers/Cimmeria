---
name: npc-vs-npc-zone-review
description: Checklist for reviewing seeded NPC-vs-NPC fight zones (#1009), learned on DA-04 #1224 (2026-10-04) - what the code guarantees vs what seed placement must guarantee
metadata:
  type: reference
---

What the #1009 code already guarantees (no need to re-prove per zone):
- The NPC scan runs only while some player witnesses the NPC (`idle_aggro.rs`, `seeks_npcs && witness_count > 0`). Witnessing uses the 150 u AoI, measured in 3D (`aoi.rs` `distance_squared_to`). A fight already under way continues with no witness.
- Assist (`assist.rs`) recruits only when the target is a player. NPC-on-NPC engagements never chain.
- Area and splash attacks from an NPC go through `npc_may_target_npc`. A non-HOSTILE override only narrows the NPC's view of other NPCs.
- A dead NPC is purged from every threat list at death (`death/mod.rs`, `purge_dead_target_from_threat`). The respawn tick reuses the same entity id, so respawn loops do not grow the entity count.
- Kill credit goes to the killing blow (`npc_only_kill.rs`). A test asserting "NPC kill pays no XP" can't fail, because `grant_kill_xp` never credits a plain NPC. Only the loot assertion guards anything, and only with a probability-1.0 loot row.

What seed placement must still guarantee (where the bugs are):
- A player-safe pair placed within the 30 u radius of a hostile-to-players squad is unsafe to visit. DA-04 put its Lucia pair 20-24 u from the NID guards. Measure every place a tester will stand, not just the rim.
- Cross-zone AoI: zones 120-150 u apart in 3D witness each other's fights.
- A seed-wide guard exclusion (for example `world_name == "DebugArea"` in `live_db_aggression`) must land in the PR that creates the world, or sibling content PRs turn main red depending on merge order. `load_spawns_from_db` inner-joins `worlds`, so sibling PRs pass on their own and fail only once rebased onto the world row.
- A spawn-held cover slot (NA22) that is also named as another NPC's seek target can be taken while its owner is dead. The owner then respawns out of cover.

Related: [[pvp-duel-readiness]], [[combat-exit-tail-parity]].
