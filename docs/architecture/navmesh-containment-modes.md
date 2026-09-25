# Navmesh Containment Modes (per-world `advisory` navmesh)

> **Last updated**: 2026-09-25 (NA26: every meshed world but Castle_CellBlock is advisory)
> **Audience**: Engineers touching movement validation, arrivals, ring transport, or world seeding
> **Type**: ADR (explanation) + reference for the one predicate
> **Owner**: Movement / space management
> **Status**: Accepted (shipped in work packet H53)
> **Confidence**: High — the Harset coverage holes are measured, not inferred; every decision below is backed by code and regression guards in the same change
> **Companions**: [movement-validation.md](movement-validation.md) (the four validation layers this gates layer 4 of), [observability.md](observability.md) (the `movement.navmesh` target catalog), [integration-test-infra.md](integration-test-infra.md)

## Context

### A `.nav` file does two unrelated jobs

A `data/spaces/<world>.nav` mesh gets consulted for two things that have
nothing to do with each other:

- **Information.** `find_path`, `line_of_sight` / `has_line_of_sight`,
  `get_navmesh_height`, NPC wander validity, and the `on_navmesh` field
  printed in `.bug` reports and the spawn log. A wrong answer here degrades
  one NPC or one diagnostic line.
- **Containment.** A hard gate on where a *player* is allowed to be — the
  navmesh layer of [movement validation](movement-validation.md), the arrival
  checks, and the snap-back recovery resolver. A wrong answer here snaps the
  player back on every inbound packet, which reads in-game as an invisible
  wall with no message attached to it.

### Partial meshes fail closed, and that is worse than no mesh

A world with **no** mesh fails open in both roles. NPCs path in straight
lines and nothing contains the player, which is survivable and already
logged (`reason = "navmesh_missing"`).

A world with a **partial** mesh fails open as information and **closed** as
containment. Every coverage hole becomes a wall. The mesh loads cleanly,
reports a plausible polygon count, and answers every query — it simply does
not describe most of the map it is named after, and the only symptom is a
player who cannot walk somewhere.

Two things hid this. First, nothing at boot distinguished "a good mesh" from
"a mesh for a different build of the map" — both are just a successful load.
Second, **every containment gate is warn-only for a GM** (see the
"GM off-navmesh allowance" section of
[movement-validation.md](movement-validation.md)), so only ordinary players
hit the walls, and every Harset tester so far has been a GM.

### Harset is the worked example

Measured 2026-09-19 with `NavMesh::is_point_valid` on a 2-unit grid against
the shipped `harset.nav` (19,345 polygons):

- The plaza floor at Y ≈ -68 is covered down to Z -198.
- There is **no coverage at all** from Z -200 to Z -228 across X -24..0 —
  which is the only walk from the gate plaza to the Command Center door.
- Ring pad 4 at `(-25.641, -67.828, 15.249)` *is* on-mesh, so the mesh does
  load and does answer. It just stops describing the world a few metres
  further in.

Before H53, an ordinary player could not cross that hole.

## Decision

Make the containment role a **per-world, explicitly seeded mode**, and route
every containment gate through one predicate. An `advisory` world behaves,
for every containment purpose, exactly like a world with no mesh — while
keeping every informational consumer intact.

### 1. The column

`resources.worlds.navmesh_mode`:

| Property | Value |
|---|---|
| Type | `character varying(16)` |
| Default | `'enforce'` |
| Nullability | `NOT NULL` |
| Constraint | `CHECK (navmesh_mode IN ('enforce', 'advisory'))` |

Table definition in
[`db/resources/Worlds/Tables/worlds.sql`](../../db/resources/Worlds/Tables/worlds.sql),
seeded in
[`db/resources/Worlds/Seed/worlds.sql`](../../db/resources/Worlds/Seed/worlds.sql).
23 rows name the column; every other world relies on the default:

- **Harset** (`world_id` 57) — the worked example below: measured holes on a
  route players must take. NA26 (2026-09-25) rebuilt `harset.nav` and the
  Command Center route is now covered, but the rebuild has not been walked,
  so the row stays advisory.
- **Castle** (`world_id` 8) — a different reason. Castle had no navmesh at
  all until `data/spaces/castle.nav` was rebuilt from the cooked client maps
  (2026-09-19), so there is no history of players walking it under
  containment. The mesh resolves every known-walked point and routes the
  mission-704 escort, but its exterior (gate room, Checkpoint Bravo) and
  interior (cells, Communications room, throne room) are still separate
  regions joined only by a chain of terrain shelves. It ships advisory so
  NPCs get pathing, line of sight and ground height immediately, and is
  promoted to `enforce` only after an in-client walk shows no coverage
  gaps.
- **The 21 NA26 worlds** (2026-09-25) — Castle's reason, applied to every
  other world in `entities/spaces.xml`. NA26 built or rebuilt a `.nav` for
  each from the cooked client maps, and a mesh nobody has walked under
  containment must not start snapping players back on day one:
  Agnos (10), Agnos_Library (20), Beta_Site_Evo_1 (23), Dakara_E1 (61),
  Dakara_E1_StoryRm (62), Harset_CmdCenter (68), Harset_Market (69),
  Harset_StorageRm (70), Ihpet_Crater_Dark (72), Ihpet_Crater_Light (73),
  Lucia (15), Menfa_Dark (77), Menfa_Light (78), Omega_Site (18),
  Omega_Site_CmdCenter (80), SGC (86), SGC_W1 (58), Sewer_Falls (50),
  Tollana (19), Tollana_Curia (88), and SandBox (2), which loads a copy of
  `harset_cmdcenter.nav`. Three of them had a 2012 mesh that was
  `enforce`: SGC_W1 and Agnos had no player telemetry to run the old-vs-new
  regression check against, and Harset_StorageRm failed it (the rebuild
  accepts 16 of the 24 real positions the 2012 mesh accepted). The seven big
  exteriors are tiled meshes (NA28), new at the whole-map extent, and no
  player has walked them under containment. Per-world evidence:
  [data/spaces/README.md](../../data/spaces/README.md).

**Castle_CellBlock** (`world_id` 12) is the one meshed world left on
`enforce`. Its mesh was rebuilt on 2026-09-19 and has been walked under
containment since; NA26's rebuild of it changed nothing measurable and was
not shipped.

A consequence worth knowing before promoting any of them: advisory worlds
emit no `movement.validation_reject` rows for the navmesh gate (the "Known
gap" in [movement-telemetry.md](movement-telemetry.md)), so after NA26 those
rows come from Castle_CellBlock alone. The evidence for promoting a world is
the TRACE-level `advisory_off_mesh_accepted` stream, which has to be switched
on for the session that gathers it.

There is deliberately **no migration script** — this repo edits the table
definition and the seed directly.

### 2. The enum

[`crates/services/src/cell/space_manager/navmesh_mode.rs`](../../crates/services/src/cell/space_manager/navmesh_mode.rs)
holds `pub enum NavmeshMode { Enforce, Advisory }`, with `Enforce` as the
`Default`. Alongside it:

- `NavmeshMode::as_db_str()` — the spelling written to the column.
- `TryFrom<&str>` — **case-sensitive and exact**. `"Advisory"`, `"ADVISORY"`
  and `"advisory "` all fail to parse. The `CHECK` constraint only admits the
  two lowercase spellings, so anything else reached the process by a route
  that bypassed the schema and should not be normalised into a weaker gate.
- `mode_from_db_value(world, raw)` — parses, and on failure logs a WARN on
  target `movement.navmesh` with `reason = "navmesh_mode_unrecognised"` and
  falls back to `Enforce`.

### 3. The predicate

One function is the whole decision:

```rust
pub fn SpaceManager::enforces_navmesh_containment(&self, space_id: u32) -> bool
```

It is `true` only when **both** hold:

1. a mesh is actually resident for that space, and
2. the space's world is `NavmeshMode::Enforce`.

An unknown `space_id` is `false` — nothing can be enforced against a space
that is not here.

Two thin wrappers expose the same decision as an `Option<&NavMesh>`, for call
sites that already branch that way rather than on a bool:

- `SpaceManager::containment_navmesh(space_id) -> Option<&NavMesh>`
- `SpaceManager::containment_navmesh_for_world(world_name) -> Option<&NavMesh>`

Plus the direct read, `SpaceManager::navmesh_mode(&self, world_name) -> NavmeshMode`,
which returns `Enforce` for a world this cell has no definition for.

The wrappers exist so a resolver that takes a mesh can pick up the mode
without growing a second parameter that a caller could pass inconsistently
with the first.

### 4. Loading and stamping

The mode travels with the world id, in one query, so the two can never
arrive by paths that disagree:

- [`crates/services/src/cell/spawner/worlds.rs`](../../crates/services/src/cell/spawner/worlds.rs):
  `load_world_ids` became `load_world_rows`, returning
  `HashMap<String, WorldRow>` where `WorldRow { world_id: i32, navmesh_mode: NavmeshMode }`.
  `WorldRow::enforcing(id)` is the fixture constructor.
- [`crates/services/src/cell/space_manager/lifecycle.rs`](../../crates/services/src/cell/space_manager/lifecycle.rs):
  `SpaceManager::stamp_world_ids` became `stamp_world_rows`, writing both
  values onto the matching `WorldDef` (which gained a `navmesh_mode` field).
- Called from
  [`crates/services/src/cell/service/startup.rs`](../../crates/services/src/cell/service/startup.rs).

## Call sites routed through the predicate

An advisory world now takes exactly the branch a meshless world has always
taken, everywhere a navmesh could refuse a player a position:

| Call site | File | What changes for an advisory world |
|---|---|---|
| The navmesh containment layer (the snap-back) | [`cell/space_manager/client_move.rs`](../../crates/services/src/cell/space_manager/client_move.rs) | Off-mesh positions are accepted. The predicate is consulted *before* `is_position_valid`, so an advisory world does not pay a Detour query per inbound packet for an answer nothing would act on |
| `reject_outcome`'s `target_is_sound` test | [`cell/space_manager/client_move.rs`](../../crates/services/src/cell/space_manager/client_move.rs) | The AABB is the whole soundness test. Without this term, the first bounds- or teleport-rejected packet from a player standing in a hole would judge their own position unusable and force-relocate them onto the nearest polygon — the same snap, one code path over |
| `resolve_recovery_position`, Detour reprojection | [`cell/space_manager/client_move.rs`](../../crates/services/src/cell/space_manager/client_move.rs) | Skipped. `get_nearest_point` is only as good as the coverage, and on `harset.nav` the nearest polygon to a point in the Command-Center corridor is on a different floor |
| `resolve_recovery_position`, respawner fallback | [`cell/space_manager/client_move.rs`](../../crates/services/src/cell/space_manager/client_move.rs) | The authored respawner coordinate is kept rather than discarded for not being covered |
| `resolve_recovery_position`, AABB clamp | [`cell/space_manager/client_move.rs`](../../crates/services/src/cell/space_manager/client_move.rs) | Kept. This one matters most: answering `None` here returns `CorrectionSuppressed`, leaving a player exactly where the validator refuses to move them from |
| `check_arrival` | [`cell/arrival.rs`](../../crates/services/src/cell/arrival.rs) | An advisory destination returns `ArrivalCheck::Unvalidated` instead of `OffMesh` — "nothing could be checked", not "refused" |
| Ring-pad warmup check | [`cell/ring_transport/runtime/tick.rs`](../../crates/services/src/cell/ring_transport/runtime/tick.rs) | Covered by the `check_arrival` change above |
| `audit_ring_pads` startup sweep | [`cell/ring_transport/regions.rs`](../../crates/services/src/cell/ring_transport/regions.rs) | Covered by the `check_arrival` change above |
| `respawner_fallback` | [`cell/respawner_fallback.rs`](../../crates/services/src/cell/respawner_fallback.rs) | Stays a pure function taking `Option<&NavMesh>`. Both callers now pass the *containment* flavour, so an advisory world reaches it as `None`. The mode decision belongs at the caller, not inside the pure core |

## What deliberately does not change

`SpaceManager::is_position_valid` still means "is this point on the mesh",
and still answers truthfully in an advisory world. Everything that only wants
to *know* keeps working:

- `find_path`
- `line_of_sight` / `has_line_of_sight`
- `get_navmesh_height`
- NPC wander validity ([`cell/service/npc_ai/wander.rs`](../../crates/services/src/cell/service/npc_ai/wander.rs))
- the `on_navmesh` field in [`cell/console/bookmark.rs`](../../crates/services/src/cell/console/bookmark.rs) (`.bug` reports)
- the `on_navmesh` field in [`cell/spawner/npcs.rs`](../../crates/services/src/cell/spawner/npcs.rs) (`spawner.npc_behaviour`)

The other validation layers are untouched. An advisory world still
hard-rejects NaN / `±∞` coordinates, out-of-AABB positions, and
teleport-sized moves. The mode narrows exactly one layer, and only for
containment.

## Why the mode is explicit data, not a heuristic

It is tempting to detect a bad mesh at runtime — a high off-mesh query rate,
a player who keeps getting rejected in the same spot — and flip the mode
automatically. We did not, for one reason:

**Containment is a server-authority gate.** Auto-detection would let a
security-relevant gate turn itself off based on conditions an attacker can
influence: where entities choose to stand, which polygons get queried, how
often a position lands off-mesh. A gate that can be argued out of existence
by traffic is not a gate.

So the mode is a seeded column, changed by a human editing the seed, and
**every fallback runs the strict way**:

| Situation | Result |
|---|---|
| Unparseable column value | `Enforce` + WARN (`navmesh_mode_unrecognised`) |
| World has no `resources.worlds` row | `Enforce` |
| World never stamped (DB down at startup) | `Enforce` |
| Unknown world name passed to `navmesh_mode` | `Enforce` |
| Unknown `space_id` | `enforces_navmesh_containment` is `false` — nothing to gate |

The failure mode of a missed stamp is "stricter than intended", never "a
movement gate quietly disappeared". Note the asymmetry: `mode_from_db_value`
fails *closed* (a data defect keeps the gate on), while the runtime predicate
fails *open* for a space it does not know (there is nothing there to gate).
Both are the safe direction for their respective question.

## When to seed `advisory`

**Seed it when a world's `.nav` is known to be incomplete relative to the map
a player can actually walk** — that is, when there are coverage holes on real
floors.

The evidence pattern that justifies it, either one:

- Probe `is_point_valid` on a grid across the walkable height band and find
  **contiguous holes on a route players must take**. Name the coordinates and
  the route in the seed comment, the way the Harset row does.
- A high `spawn_rows_off_mesh` count in the startup summary (see below).
  Spawn rows are independently authored coordinates of things that stand on
  the floor, so they are the cheapest proxy the server has for "does this
  mesh describe the map it is named after".

**Do not seed it:**

- To work around a single bad authored coordinate. Re-pin the coordinate
  instead — one wrong spawn point is not a mesh problem.
- For a world whose mesh is merely *unverified* but has been walked under
  containment. "Nobody has probed it" is not evidence of holes, and
  `advisory` gives up a real gate.

**Do seed it for a new mesh.** A world that gets its first mesh, or a
rebuilt one nobody has walked yet, starts `advisory` (Castle, and every
NA26 world). The old state of such a world was "no containment at all", so
advisory loses nothing, and an unwalked mesh's holes are unknown. Promote it
with the steps below once play shows the coverage holds.

## Getting back to `enforce`

Removing the flag is the goal, not a nice-to-have. `advisory` is a
containment gate the world is running without.

1. Rebake the mesh (tracked as Harset **GH1**). The real player traffic that
   tells you *which* parts of the world need coverage is the
   `advisory_off_mesh_accepted` TRACE described below — that is the point of
   collecting it, as opposed to running another static probe.
2. Re-probe the walkable band and confirm the holes are gone.
3. Drop the `navmesh_mode` column from that world's seed `INSERT` so it takes
   the `'enforce'` default again, and delete the comment block explaining why
   it was advisory.
4. Confirm at boot: the `navmesh_mode_summary` line for that world should read
   `navmesh_mode = "enforce"` with a `spawn_rows_off_mesh` near zero.

## Observability

All three events are on the `movement.navmesh` target — see the target
catalog in [observability.md](observability.md).

- **`navmesh_mode_summary`** (INFO, once per resident meshed space at
  startup, from `SpaceManager::log_navmesh_summary`). Fields: `space_id`,
  `world_name`, `navmesh_mode`, `poly_count`, `spawn_rows`,
  `spawn_rows_off_mesh`. It is INFO and not WARN even at a high off-mesh
  count, because an advisory world is *expected* to have one and a WARN that
  fires every boot stops being read. The operator-actionable signal is the
  number **moving**, and a high count on an `enforce` world is an
  invisible-wall report waiting to happen.
- **`advisory_off_mesh_accepted`** (TRACE, from
  `cell::space_manager::client_move`). Records the off-mesh positions an
  advisory world accepted: `entity_id`, `space_id`, `client_x` / `client_y` /
  `client_z`. **Level-gated behind `tracing::enabled!` before the query**, so
  an advisory world does not run a Detour lookup per inbound position packet
  per player in production to produce a line nobody is collecting. Turn it on
  when you are gathering rebake input.
- **`navmesh_mode_unrecognised`** (WARN, from `mode_from_db_value`). Names the
  `world_name` and the `raw_value` so the offending row is findable.
  Containment stays on.

The `stamp_world_rows` INFO line also gained an `advisory_worlds` field, so a
boot log answers "which worlds are running without containment" in one row.

## Regression guards

In
[`crates/services/src/cell/space_manager/tests/movement_validation/advisory.rs`](../../crates/services/src/cell/space_manager/tests/movement_validation/advisory.rs),
against the real `harset.nav` (self-skipping on a fixture-less checkout):

- `a_step_into_a_mesh_hole_is_accepted_in_an_advisory_world`
- `the_same_step_is_rejected_off_navmesh_in_an_enforcing_world` — the
  negative control that keeps the advisory accept meaningful
- `an_out_of_bounds_move_is_still_rejected_in_an_advisory_world`
- `a_teleport_sized_jump_is_still_rejected_in_an_advisory_world`
- `a_player_standing_in_a_hole_is_corrected_back_not_relocated`
- `a_gm_still_moves_freely_in_an_advisory_world`
- `the_startup_summary_counts_off_mesh_spawn_rows` — pins that the boot line
  names the mode and counts only this world's rows, one on the mesh and one
  off it

Every one of those asserts its fixture controls before its verdict: ring pad
4 at `(-25.641, -67.828, 15.249)` reads on-mesh (so the mesh loaded), the
start point reads on-mesh, and the target point reads off-mesh. Without the
first, a mesh that failed to load would make the advisory half vacuously
green. Since NA26 the step is a real reject from SigNoz, `(217.61, -41.87,
3.66)` to `(215.32, -42.29, 3.80)`, because the rebuilt mesh closed the
measured Command Center hole the tests used to walk into.

Two live-DB guards in
[`crates/services/src/cell/spawner/worlds.rs`](../../crates/services/src/cell/spawner/worlds.rs)
pin the seed itself: `harset_loads_advisory_and_a_meshed_neighbour_loads_enforce`
(the enforcing neighbour is Castle_CellBlock since NA26)
and `only_the_documented_worlds_are_seeded_advisory`. The second is a list comparison,
not a count, on purpose — a count would not catch a loader bug that demoted
every world.

One arrival guard in
[`crates/services/src/cell/arrival.rs`](../../crates/services/src/cell/arrival.rs):
`an_advisory_destination_is_unvalidated_not_off_mesh`, which also covers the
two ring-transport consumers because they both call `check_arrival`.

Plus the predicate's own truth table in
[`navmesh_mode.rs`](../../crates/services/src/cell/space_manager/navmesh_mode.rs):
`only_the_two_db_spellings_parse`,
`an_unrecognised_column_value_falls_back_to_enforce`,
`containment_needs_both_a_mesh_and_the_enforce_mode`,
`unknown_ids_are_safe_in_both_directions`, and
`an_unstamped_world_keeps_containment_enforced`.

Revert-verified: weakening `enforces_navmesh_containment` to "a mesh is
loaded" fails the advisory accept, the recovery guard, the arrival check and
the predicate table, and leaves the enforce-side controls green — which is
the right shape for a guard on this.

## Documentation debt

- The measured Harset coverage map (the 2-unit-grid probe output) lives only
  in the H53 commit messages and this doc's summary of it. If a second world
  ever needs `advisory`, that probe should become a checked-in tool rather
  than a one-off script, and this section should point at it.
