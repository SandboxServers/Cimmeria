---
name: telemetry-last-valid-is-mostly-synthetic
description: A raw last_valid_* histogram from movement.validation_reject is dominated by default/test positions like (0,0,0) and (50,2,50) - use only a cleaned list, and treat a region ceiling as a real discriminator on multi-storey rooms
metadata:
  type: project
---

**Read before using `movement.validation_reject` `last_valid_*` positions as "this is walkable" evidence.**

**Why:** the raw histogram for Harset (space 65544) is **(0, 0, 0) x100,649** and (1,1,1) x1,256 — about 77% of all rejects. For `Harset_StorageRm` (65558) it is (50, 2, 50) x7,958, (2, 30, 50) x2,765, (2, 2, 50) x1,744, (2, 2, 100) x1,617. Those are entities parked at a default or test position, refused on every packet. Ranking by count puts a "walkable anchor" at (50, 2, 50), which is nowhere. After cleaning, StorageRm has **20** distinct real positions, not thousands.

**How to apply:**

- Treat a point as real-player evidence only if it survives an explicit synthetic filter: origin, unit values, round numbers, and any position whose reject count is orders of magnitude above the rest. The cleaned Harset lists live at `$O\harset\harset_storagerm_last_valid_probes.txt` with the exclusions explained in `harset_suspicious_points.txt`.
- `last_valid_*` is where the player **was** when a move was refused, so the server had already accepted it — that is the walkable claim. `client_x/y/z` is where they *tried* to go, so those mark holes, not floors.
- **Telemetry is HIGH for "walkable" and says nothing about purpose.** It cannot tell you an NPC belongs there.
- **Sort the surviving points by height band before using them.** In StorageRm the 13 points inside one room's footprint sit in four bands: pen floor (y 1.25-1.58), the under-layer (-1.68), the upper arrival deck where it overhangs (7.06), and gantries/catwalks (7.7-17.7). Averaging or bounding-boxing them all together produces a volume that means nothing.
- **A region ceiling is load-bearing on a multi-storey room.** `is_point_in_region` widens the AABB by `GENERIC_REGION_CHECK_THRESHOLD` (1.5 m) on **every** axis including Y, so the real admitted band is `[floor - 1.5, ceiling + 1.5]`. Pick the ceiling from the telemetry bands: for `Harset_StorageRm.Storage` a 4.00 ceiling admits all 7 floor positions with 3.9 m to spare and none of the other 13, where the 10.30 first draft admitted 5 gantry/deck positions. Admitting the deck a player arrives on destroys the `enter_region` edge for them (see [player-loaded-edge-trigger-race.md](player-loaded-edge-trigger-race.md)).
- Telemetry makes an excellent live-DB guard: assert the seeded region contains every real accepted position on the floor and excludes every one that is not. Both halves revert-verify cleanly (raise the ceiling, shrink the footprint).

Worlds 68 (Harset_CmdCenter) and 69 (Harset_Market) have **no telemetry and no navmesh** — nothing placed in them has ever been confirmed walkable. See [map-data-placement-toolkit.md](map-data-placement-toolkit.md) and [navmesh-onmesh-assertions-are-weak.md](navmesh-onmesh-assertions-are-weak.md).
