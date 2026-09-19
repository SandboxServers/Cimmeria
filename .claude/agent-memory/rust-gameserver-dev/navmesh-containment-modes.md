---
name: navmesh-containment-modes
description: Per-world navmesh_mode (enforce/advisory) names and the traps around navmesh gating, the shared space_manager TEST_SPACES_XML fixture, and harset.nav's two-floor geometry
metadata:
  type: project
---

# Per-world navmesh containment mode (H53)

Shipped on `harset/H53`. A `.nav` mesh has two roles and they were conflated:
**information** (`find_path`, line of sight, `get_navmesh_height`, NPC wander,
`on_navmesh` diagnostics) and **containment** (a hard gate on where a player
may be). A meshless world fails open in both; a **partial** mesh fails open as
information and *closed* as containment, so coverage holes become invisible
walls. Every containment gate is warn-only for a GM, which is why a partial
mesh can ship for months without a report.

**Why:** `harset.nav` is badly incomplete and made Harset unwalkable for
ordinary players. **How to apply:** read this before touching anything that
rejects a position, an arrival or a ring trip because a point is off-mesh.

## Names (stable, consumed by other sessions)

- Column `resources.worlds.navmesh_mode`, `varchar(16)`,
  `DEFAULT 'enforce' NOT NULL`, `CHECK (navmesh_mode IN ('enforce','advisory'))`.
  Defined in `db/resources/Worlds/Tables/worlds.sql`, seeded in
  `db/resources/Worlds/Seed/worlds.sql`. **No migration script** (owner rule).
- `cell::space_manager::NavmeshMode { Enforce (#[default]), Advisory }`.
- **The predicate:** `SpaceManager::enforces_navmesh_containment(space_id: u32)
  -> bool` — mesh resident AND world is `Enforce`.
- Wrappers: `containment_navmesh(space_id)`,
  `containment_navmesh_for_world(world_name)`, both `-> Option<&NavMesh>`.
- `SpaceManager::navmesh_mode(world_name) -> NavmeshMode`.
- Loader `cell::spawner::load_world_rows` → `HashMap<String, WorldRow>` with
  `WorldRow { world_id, navmesh_mode }` and `WorldRow::enforcing(id)`.
  `SpaceManager::stamp_world_rows`. **`load_world_ids` / `stamp_world_ids` no
  longer exist.**
- `SpaceManager::log_navmesh_summary(&[SpawnRecord])` — startup INFO per
  meshed world with `navmesh_mode`, `poly_count`, `spawn_rows_off_mesh`.

To demote a second world: add the column to its seed INSERT **and** update
`exactly_one_world_is_seeded_advisory` in `cell/spawner/worlds.rs`, which pins
the advisory world list by name (deliberately a list, not a count — a count
would not catch a loader bug that demoted everything).

`is_position_valid` still means "is this point on the mesh" and stays truthful
in advisory worlds. Bounds, speed and teleport layers are untouched.

## Traps found doing it

- **`crates/services/src/cell/space_manager/tests/mod.rs`'s `TEST_SPACES_XML`
  and `TEST_CELL_SPACES_XML` are load-bearing.** Four tests pin
  `space_count()` and the exact `(cell_id << 16) | index` startup space ids
  (`parse_spaces_xml_loads_all_worlds`, `startup_spaces_get_correct_ids`,
  `instanced_space_created_on_demand`,
  `create_entity_in_instanced_space`). Adding **one** non-instanced world to
  the fixture breaks all four. Build a one-world `SpaceManager` in your own
  test module instead.
- **`cell::arrival::test_insert_navmesh_space` did not insert a `WorldDef`.**
  Any per-world setting stamped by `stamp_world_rows` (which walks
  `self.worlds`) silently stays at its default for a grafted space. Fixed
  there; watch for the same shape if another per-world field is added.
- **`NavMesh::get_nearest_point` returns its input unchanged on a miss**
  (`unwrap_or(*pos)` in `crates/entity/src/navigation/mod.rs`). Never use it
  as a distance-to-mesh measure, and always re-validate its output.
- `git checkout -- <file>` restores from **HEAD**, so it is useless for
  undoing something you already committed earlier in the same session. See
  [[revert-verification-loses-uncommitted-fmt]].
- Python heredocs through the Bash tool on this host choke on non-ASCII (em
  dashes); use the Edit tool for prose edits, and keep scripted edits ASCII.

## harset.nav geometry (measured 2026-09-19, 19,345 polys)

Two distinct floors in the Command-Center corridor, not one:

| Band | Coverage at X -24..0, Z -190..-236 |
|---|---|
| `Y ~ -41` | continuous through the corridor |
| `Y ~ -68/-71` | the plaza floor: covered to Z -198, **nothing Z -200..-228**, then a strip at X -18..-10 from Z -230 |
| `Y = -58.65` | off-mesh everywhere here — it sits *between* the two floors, outside both vertical tolerances. This is the H51 nine-inert-sentries finding from another angle |

Players stand in the `-68` band (ring pads are at `-67.828`, chain 6007's door
at `-67.6`). Known on-mesh control point: ring pad 4 at
`(-25.641, -67.828, 15.249)`. Use it as the "did the mesh load" assertion in
any harset.nav test — without it an empty mesh makes an off-mesh verdict
vacuously true.
