---
name: npc-vs-npc-1009
description: NPC-vs-NPC combat (#1009, 2026-09-28) - who fights whom, the witness-gated grid scan, dormant seekers, NPC-only kills pay nothing, Castle standoff placement facts
metadata:
  type: project
---

NPC-vs-NPC landed in #1009 (branch feat/1009-npc-vs-npc, 2026-09-28).

- **Rule.** `combat::npc_may_target_npc(viewer, target)` = both SGWMob, not pet, not player, and
  `REACTION[viewer.faction][target.faction]` HOSTILE. The aggression override only *narrows* it
  (non-HOSTILE override disarms vs NPCs; HOSTILE override does not widen). Faction 1 (World
  Object) is FRIENDLY in every row: a faction-1 NPC never fights anything. Factions 3 and 10 are
  mutually HOSTILE; the seed had only templates 10 (Marsh, a being, excluded) and 17 (Prisoner
  329) on faction 3 before #1009, and no hostile NPC pair within aggro range anywhere.
- **Scan.** Idle NPCs with `seeks_npc_targets` are admitted; the NPC half of
  `npc_ai_idle_auto_aggro` runs only while a player witnesses the scanner, queries
  `SpaceManager::npc_ids_near` (WorldGrid, 2x aggro radius) and picks the closest of the player
  and NPC picks. Unwitnessed seek-only NPCs skip their whole turn in `dispatch` but stay in
  `admitted` (so `npc_ai_idle_unticked` does not count them). Assist stays player-only.
- **Death.** `purge_dead_target_from_threat` (was `purge_dead_player_from_threat`) runs for NPC
  deaths too. `death::npc_only_kill` = killer is a live non-pet NPC -> no loot roll; XP and
  `EntityDeath` were already refused for plain NPCs. Credit is killing-blow only (owner question
  open: per-contributor credit).
- **Adjacent fix.** `npc_ai_idle_auto_aggro` now returns "NPC is Fighting"; it used to return
  "player just entered combat", so a patroller engaging an already-in-combat player was flipped
  back to Patrol by the dispatcher.
- **Castle standoff (D-CP11).** Templates 187-189 (faction 3, level 4, aggro 30). Courtyard rows
  sit on the slope at x 532-538 z 612-627 vs `Castle_PRU4`; Alpha Jaffa at the ramp foot x 894-897
  vs `Castle_NidGuard7`. The Alpha room is 25-30 u above every field hostile, so nothing there
  can engage without moving down the ramp. Placement was searched with the real scan on
  castle.nav + castle.occ (a temporary grid-search test); the content guard is
  `service::tests::npc_ai::castle_standoff`.
- **CPU.** 10v10 steady fighting ~105 us per 2 s AI tick (release), same as 20 guards fighting a
  player; idle overhead ~1 us per faction-10 NPC per tick. Pre-existing: a 400-NPC space makes a
  20-NPC fight ~4x dearer on main too (something per-shot scales with the space's entity count).

Related: [[faction-derived-aggro-na13]], [[assist-aggro-na14]], [[faction-10-gates-everything]].
