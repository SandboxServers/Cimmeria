---
name: technique-pull-map-probe
description: For content-placement reviews (Debug Area, arenas), sweep a navmesh grid with the production Idle scan to find every pull point; fixed spectator-spot guards miss gaps in walls
metadata:
  type: project
---

Fixed-point placement guards (spectator spots, pairwise spacing pins) miss pull zones that leak through wall gaps. In the PR #1244 review (2026-10-04), a probe found the NID squad pulling a player in the fight-2 room's SW corner (x 328-336, z -730..-734) past the long wall's end. The PR's guards all passed.

**How:** in `crates/cell/src/cell/service/tests/npc_ai/debug_area_combat/`:
- build `scene(&records)` with any masking squads filtered out (the scan picks the closest hostile, so an NPC enemy hides a player pull);
- `add_player` once, then step the player over a 2 u grid with `navmesh().get_height_near(x, hint, z)` at several height hints (`update_position_preserving_facing` + `compute_aoi_changes`);
- for each point: `reset_idle` the squad, then `scan()` each guard;
- bucket the hits by zone.

A run of about 6 s covered 60x55 points × 4 hints.

**Related semantics:** the NPC Idle scan's 2x "consider" radius (`NPC_CONSIDER_RADIUS_FACTOR`) only writes DEBUG `npc_candidate_rejected` rows. Engagement stays at 1x aggro radius. `debug_area::reach::no_npc_fighter_reaches_a_da03_target` uses the 2x radius as its margin. So "fails the reach guard" means log noise, not a real engagement. Weigh any aggro-radius cut made to satisfy that guard against the fight's own margin.

**Why:** content guards that check only the authored spots read as complete coverage, but they are not. A pull map is cheap and runs on the real mesh and occluder (both are in git).

**How to apply:** use it on any review that moves NPC squads or claims "spectators are never pulled". Suggest it as a guard: every engaged point stays inside the intended join zone. See also [[finding-self-skipping-asset-tests]].

**Landed as a guard (#1244, 2026-10-05):** `crates/cell/src/cell/service/tests/npc_ai/debug_area_combat/arena_pull_map.rs::the_nid_squad_pulls_a_player_only_inside_the_fight_strip`. DA-F2 fixed the leak by closing fight 1's rows to z -738/-741/-744 (NID3 at -746 saw past the long wall's west end).
