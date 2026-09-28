---
name: respawner-zero-and-false-pull-reports
description: callForAid respawner_id 0 used to land Castle players at Stasis Chamber (first row of an unordered SELECT; now nearest-to-death), and "pulled through walls" reports often turn out to be the player walking past the NPC on the way back from a respawn
metadata:
  type: project
---

Found diagnosing the 2026-09-28 colo playtest (build 5f9730c6, Hallway01_Guard npc 100222).

- **Respawner 0:** the Defeat Window offers `[8, 5]` for Castle_CellBlock (log `Sent onBeginAidWait`,
  `respawner_ids`). If the client answers `callForAid` with 0, `resolve_respawn_target` step 2
  (`cell-interactions/src/cell/respawn/mod.rs`) used to take the FIRST world row. `load_respawners` had no
  ORDER BY, and 8 (Stasis Chamber, -334/73/-228, same as CASTLE_DEFAULT_POS) came first.
  **Fixed the same day** (Decision (@Cadacious, 2026-09-28)): id 0, the auto-respawn -1 and an unusable id
  now resolve to the authored respawner nearest the death position through
  `respawner_fallback::nearest_valid_respawner_def`, logged at INFO on target `player.respawn` with
  `reason=respawner_id_zero|respawner_id_unset|respawner_unusable`, `respawner_id` (chosen),
  `requested_respawner_id` and `distance_m`. `load_respawners` now orders by `respawner_id`.
- **"Pulled through walls":** before blaming assist or threat carry-over, find the NPC's
  `npc_ai.aggro event=acquired` row. Here it was plain proximity aggro at 8.4 u with `has_los=clear`: the
  player respawned at the start area and walked back past the hallway guard, which then followed a
  valid navmesh route (the same line the player ran) to the Mess Hall.
- **Stuck warn false positive:** `detectors/sweep.rs::check_stuck` looks only at NPC-to-target distance, so
  it fires while a target outruns a chasing NPC that is moving along its route.
- A 43 u chase does not leash: default radius 50 + 5 u band, horizontal, vertical cap 20.

Related: [[leash-reset-na12]], [[assist-aggro-na14]]
