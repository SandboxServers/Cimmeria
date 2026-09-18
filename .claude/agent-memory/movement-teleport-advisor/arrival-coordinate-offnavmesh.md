---
name: arrival-coordinate-offnavmesh
description: Block on sight — authored entity coordinates are model origins, not standing positions; gate/ring arrivals land inside prefab navmesh carve-outs and strand the player as CorrectionSuppressed
metadata:
  type: project
---

**Add to the block-on-sight list: an arrival coordinate taken from a content/entity row is a *model origin*, not a position a player can stand at. Validate it against the destination world's navmesh before wiring any arrival.**

**Why:** measured directly from `data/spaces/harset.nav` (2026-09-17, read-only header + quantized-vertex decode). Gate travel arrives at the destination gate's own `resources.stargates` row — `cell/gate_travel.rs:49` then `:104` `position: [gate.x, gate.y, gate.z]`. For Harset (stargate id 3) that is `(-0.076, -67.274, 38.011)`. The navmesh has a **~5-unit radius hole centred on the gate** (the `GLB-Stargate_Prefab_Seq` footprint carve-out): nearest walkable vertex is **5.05 u away in XZ**, and the walkable floor there sits at **Y ≈ -68.8 … -69.2**, i.e. the gate row's `y_pos` is ~1.5 u *above* the floor because it is the prefab's origin.

That is not a near miss — it is unreachable by the validator's own search box. `NavMesh::is_point_valid` (`crates/entity/src/navigation/mod.rs:436-448`) searches `DEST_EXTENTS = [3.0, 3.0, 3.0]` (`mod.rs:43`), then retries with `JUMP_SEARCH_EXTENTS = [3.0, 5.0, 3.0]` (`mod.rs:78-82`) re-centred 4 u below. Both are ±3.0 horizontally, so **neither phase can reach mesh 5 u away** → `is_point_valid` = false.

The termination chain then dead-ends, because `resolve_recovery_position` (`cell/space_manager/client_move.rs:491-551`) has nothing to offer:

1. `nav.get_nearest_point` → `find_nearest_poly` (same `DEST_EXTENTS`) fails and `unwrap_or(*pos)` returns the **input unchanged** (`navigation/mod.rs:466-468`), so the candidate fails its own `is_point_valid` re-check.
2. Nearest respawner — **world 57 has none.** `db/resources/Worlds/Seed/respawners.sql` holds only 8 rows total, for worlds 8, 12 and 23, and 6 of the 8 are literally `(0,0,0)`.
3. AABB clamp — arrival is well inside `harset.nav`'s AABB, so `clamped == from` and the function returns `None` early.

→ `CorrectionSuppressed` from the first packet onward. Not the #644 rubber-band (that needs a *sound* target); the failure shape here is **silent**: the server never writes the player's position, so they move normally on their own client, never move for witnesses, and can't interact. Distinguish the two in triage by the outcome counter, not by the player's description.

**Why nobody has hit this yet:** the navmesh layer *fails open* when `space.navmesh` is `None` (`cell/space_manager/spatial.rs:54-67`, three separate `return true`s). Castle (world 8), the main gate destination, has **no** `data/spaces/castle.nav` — only `castle_cellblock.nav` exists. The defect is latent in every gate arrival and only fires in a navmesh-backed destination. `data/spaces/` currently holds `agnos.nav`, `castle_cellblock.nav`, `harset.nav`, `harset_storagerm.nav`, `sgc_w1.nav` — so **Harset is the first navmesh-backed gate destination**, and any Harset campaign is what surfaces this.

**How to apply:**

- Never reuse an entity/prefab row's `x/y/z` as an arrival position. Gate rows, ring-pad rows and point-set corners are all authored geometry, not stand points.
- The house precedent is already correct and should be cited: chain 1109's `cross_world_teleport` coordinate carries the comment *"Coords pinned in-game to the visible Castle ring platform (map debug HUD readout)"* (`db/resources/Content/Seed/castle_cellblock_chains.sql:1710-1719`), and work packet C09 makes "verify the arrival coordinate lands within interaction range" a **manual UAT step**. Budget that step; there is no automated navmesh gate on any transfer path.
- Coordinate validation on transfer is **finiteness only** — `TransferRejected::NonFinitePosition` (`space_transfer/mod.rs:121`) and the loader guard at `content-engine/src/loader/action.rs:241`. Nothing checks walkability.
- Seeding a respawner row per world is cheap insurance: it is recovery candidate 2 and is validated before use, so it converts a silent `CorrectionSuppressed` into a visible `Recovered`.
- The same measurement method works for any world: the `.nav` header is fixed-offset LE (`agent_height, agent_climb, agent_radius` f32; `nverts, npolys, nvp, border_size` u32; `cs, ch` f32; `bmin[3], bmax[3]` f32 = 60 bytes), then `nverts` × 3 × u16 quantized verts with `world = bmin + v * cs` for X/Z and `bmin.y + v * ch` for Y. Decoding it is read-only and needs no build.

**Axis note that keeps biting:** in SGW **Y is the vertical axis**. `within_containment_tolerance` (`navigation/mod.rs:453-463`) gates X/Z on `agent_radius * 2.0` (= 1.2 u at the shipped `agent_radius = 0.6`) and Y on `+JUMP_HEIGHT_TOLERANCE` / `-agent_radius*2`. Generic "validate Z" guidance maps to **Y** here. `bounds.rs` prose and `docs/architecture/movement-validation.md:41` call the floor-clip axis "Z", which contradicts the navmesh code — harmless (all three axes are tested) but misleading in review.

See [[snap-back-termination]] for the sound-target loop this is the mirror image of, and [[authorized-teleport-paths]] for why every arrival write is unchecked.
