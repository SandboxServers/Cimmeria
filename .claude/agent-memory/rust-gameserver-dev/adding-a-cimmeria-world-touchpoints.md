---
name: adding-a-cimmeria-world-touchpoints
description: Adding a new server world id (DA-01, 2026-10-04) - ADDED_WORLDS table, seed row, spaces/cell_spaces, category-12 push, nav/occ client-map fallback, and the traps (spaces.xml bounds unused, respawners have no heading, world_id_for_name defaulted to CombatSim)
metadata:
  type: project
---

A Cimmeria-added world (id the shipped `CookedWorldInfo.pak` lacks) is one row in
`cimmeria_wire::mercury::world_data::added_worlds::ADDED_WORLDS`. That table feeds
`known_world_id` / `world_id_for_name` / `client_map_for_world` (onClientMapLoad,
setupWorldParameters), `WORLD_INFO_OVERRIDES` (category-12 push; `world_info_flags` =
the shipped entry's Flags of the map it plays on) and the fail-closed
`resolve_space_id_fallback`. Data still needed by hand: `worlds.sql` row,
`entities/spaces.xml` (+ `cell_spaces.xml` if shared), respawners, advisory list test in
`cell-catalog/.../worlds.rs`.

- `world_id_for_name` used to send world id 1 (CombatSim) for 8 shipped spaces.xml worlds
  (50, 61, 62, 69, 70, 72, 73, 78). `every_declared_space_resolves_to_its_seed_world_id_and_client_map`
  (wire tests) now checks every spaces.xml world against the seed. The id the client keeps is
  `setupWorldParameters.worldId`; it discards onClientMapLoad's WorldID
  ([[client-world-id-and-same-map-load]] in bigworld-engine-advisor memory).
- `.nav`/`.occ` resolve via `cell-world/.../space_manager/space_files.rs` AS A PAIR: a world
  shipping either file of its own uses only its own (SandBox: sandbox.nav, no occluder);
  otherwise both come from the client map (D-DA5). Occluder cache keyed by FILE
  (`occluder_files` maps world key -> file key) so two worlds on one map share pages.
  `SpaceManager.space_data_dir` lets a test point at the repo `data/spaces`.
- Same-map travel (1300 <-> 73, SandBox <-> Harset_CmdCenter, `.gotospace` between instances)
  never reloads the client level; the base finishes the entry from onClientReady
  (`handle_on_client_ready` synthesises mapLoaded). Live behaviour is a DA-06 check.
- spaces.xml MinX/MaxX/MinY/MaxY are parsed but read by nothing (movement bounds come from
  the navmesh). `resources.respawners` has no heading column; respawn rotation is [0;3].
- `.gotolocation <world>` on a world with no char start / ring pad / stargate lands on the
  lowest-id authored respawner. A stargate row for the world would take precedence.
- A world stargate row makes the gate's arrival the `.gotolocation <world>` entry point;
  D-DA4 was amended 2026-10-04 so world 1300 gets an outbound-only gate (da07).
- The lane log-dir race (an empty per-worktree log dir pruned while a job waits for a slot)
  was fixed by #1216, not by DA-01.

Related: [[cover-seed-ids-and-orient-convention]].
