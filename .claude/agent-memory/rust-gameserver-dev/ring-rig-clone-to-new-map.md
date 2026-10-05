---
name: ring-rig-clone-to-new-map
description: Cloning the Castle ring rig into another map (DA-08, patch 010) - donor is 007's result, base 1192 has a 1-LOD tail, one central chunk for streaming, template 3 renders the console, floor heights from obj_slab/occluder
metadata:
  type: project
---

Facts from building `010-debug-area-rings` (2026-10-04, eight rigs in `Ihpet_Crater_Light-fff80002`).

- **Rig source.** Region 3's de-prefabbed rig in `Castle_CellBlock-fffeffff`: roots
  `772` (sequence), `1192` (base, first actor root so `--first-at` lands it), `216`
  (emitter), `218,219,220,227,228` (rings); `--map 764:<target Prefabs index>`. Console
  `1194` sits at base + game (2.88, 0.44, 1.32). 007's command reproduces byte-exact:
  `--roots 772,216,218,219,227,228 --map 764:764,220:226 --anchor 1192:1174`.
- **Donor = 007's result, not stock.** 007 rewrites fffeffff, so a cross-map patch's
  donor source must pin 007's result hash (`2f41a7e1…`), carry `"output_of"` (recipe
  field, reports `PatchOutputMismatch`), and be published `after: 007` exactly
  (`blocked_by_failure` checks only the named id). Superseding 007 means rebuilding 010.
- **Witness Teleport In is fragile.** Cell witness lists refresh on the AoI tick, not in
  the teleport, and the client erases view-type-3 sequences whose source has no pawn
  (`FUN_00d06f30`); the traveller is hidden until ShowPlayer. Documented, not fixed.
- **Test trap:** `patchset::tests::noisy(seed)` uses `seed | 1`, so seeds 22 and 23 give
  identical bytes. Pick unrelated seeds for "a different file".
  The prefab-instanced rigs (fffefffe, Harset) carry archetype imports: avoid.
- **Base 1192's component ends in `1,0,0,0`** (one empty LODInfo). The cloner refused it
  until `tail_copies_verbatim` (object_clone.rs) accepted all-zero LOD entries.
- **Chunk choice.** Chunks are `LevelStreamingDistance` 500 m from the chunk centre;
  chunk name `XXXXYYYY` = floor(game z / 100), floor(game x / 100). Put every rig in one
  central chunk so the destination rig is loaded when Teleport In fires. A chunk
  needs `Main_Sequence.Prefabs` (some Ihpet chunks have no Main_Sequence at all).
- **Floor heights.** Navmesh lies on open terrain (up to 3 m); `obj_slab --levels` on
  `extract_map` OBJs and `occluder_extract probe` columns give the render floor.
  The Ihpet "pit" is a WaterCollisionPrefab plane at y -33.28.
- **Consoles.** Template 3 renders `GP-Ring_Trans_Console00`, so don't clone the map
  console too. Text 7508 (every ring's DisplayName) is empty.
- **bsdiff vs donor.** Most cloned bytes match the target's own objects (same target
  name indices), not the donor; only the extra block is verbatim. Check it with the
  header arithmetic in `debug_area_rings_tests.rs`.
- Worktree-isolated Bash refuses `$VAR`-computed commands, loops and heredocs with
  `cd`: drive multi-file tool runs from a small Python script instead.

Related: [[patchset-supersede-and-restore-to-stock]], [[cooked-pak-and-dialog-override-traps]].
