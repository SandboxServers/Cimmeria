---
name: ring-rig-stacking-patches
description: Adding a ring rig to a chunk an earlier rig patch already wrote (DA-11 / patch 014): one-source delta, instance numbering, footprint survey numbers for the Debug Area pads
metadata:
  type: project
---

Facts from DA-11 (2026-10-05, Lineup station, region 43, patch `014-debug-area-lineup-ring`).

- **Stack, don't rebuild.** A new rig goes on top of the previous rig patch's output
  (`output_of` that patch, no `alternatives`): the earlier patch already takes stock and
  broken states to its output. Cloning from Castle's rig into 011's chunk needs the Castle
  map only for `upk_patch`; the bsdiff delta built from 011's chunk alone was smaller
  (607 B, extra 14) than with the Armory map as a second source (629, extra 39), so 014
  does not pin 007. Publish `after` the patch it builds on and keep it terminal.
- **Instance numbers continue.** 011's chunk holds `_Pf0_Seq`, `_Seq_0`..`_Seq_6`; a clone
  into it takes `_Seq_7`. `--map 764:104` still points at `Main_Sequence.Prefabs` (export
  indices are append-only). The cloner added 0 names: 011's table already had the
  client-loadable second `Dynamic`, and `ensure_name_with_flags` picks it.
- **Footprint survey (occluder, solid span hi > base+0.3 and lo < base+3.5):** clear radius
  per pad 35: 7.5, 36: 5.0, 37: 6.4, 38: 4.4, 39: 6.3, 40: 5.0, 41: 5.0, 42: 4.4, 43: 7.6.
  The live-DB guard now enforces 4.0 m on every pad (`RIG_FOOTPRINT`).
- **Walk distances lie.** "20 m on foot" from the Lineup pad to the lineup doorway was the
  crow-flies distance; Detour's route is ~39 m round a 0.3-0.9 m wall along z -905. Check a
  walking claim with `find_path` and sum the legs.
- **Console offset is rig-relative.** Every station's console is base + (2.88, 0.44, 1.32),
  heading -2.0617, because the rig is cloned unrotated; don't move it to a "nicer" spot.

Related: [[ring-fsm-departure-hooks]], [[arrival-coordinate-offnavmesh]].
