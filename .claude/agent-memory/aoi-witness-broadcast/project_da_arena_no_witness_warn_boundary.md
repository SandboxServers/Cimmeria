---
name: project-da-arena-no-witness-warn-boundary
description: 2026-10-05 colo no_witnesses WARN on Debug Area arena was a false positive from player_present's counterpart-in-AoI branch; player sat at the 150 m AoI edge
metadata:
  type: project
---

Colo v2026-10-05.2, world DebugArea (1300), space 65552: abilities.sequence `no_witnesses` WARN, shooter Op-CORE Soldier (template 1370) to target NID Guard (1372).
A single idle player stood about 145 m (dy 17 m) from the NID guards, so the guards were inside the 150 m enter radius (`PLAYER_AOI_RADIUS`) and the Soldier, 24 m further out, was not. The Soldier left that witness's view at 19:57:52Z and was never re-introduced.

**Why:** `SpaceManager::player_present` (cell-world/space_manager/player_presence.rs) counts a player in AoI of the shooter OR the counterpart. Counterpart-in-AoI makes the WARN fire although nobody could have seen the shooter. Not a witness bug.

**How to apply:** for the `onSequence` no-witness WARN the AoI branch should test the shooter only (the thing the witness list is of); keep is_player_side for either party. Arena at x 354-378, z -744 sits on the AoI boundary of the player's spot, which also produces enter/leave churn.

**Outcome:** fixed on `fix/no-witness-presence-shooter-only`: `player_present` tests AoI for the shooter only; counterpart still counts via `is_player_side`. Only the sequence caller passes a counterpart (the wire caller passes None), so no other site changed. Guards: `a_player_in_range_of_only_the_counterpart_is_not_present` and `an_npc_shooting_a_target_a_player_sees_from_beyond_its_own_aoi_writes_nothing`; both fail with the counterpart AoI branch restored.
