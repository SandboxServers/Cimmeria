//! Seed-vs-navmesh guards for the Harset placements made from map data.
//!
//! Every coordinate this module pins was *placed*, not walked: the owner
//! stopped waiting for the in-client M0 session and asked for estimates
//! labelled with their evidence instead
//! (`docs/analysis/harset-rebuild/placements/METHOD.md`). The ledger row for
//! each one is in `placements/A-arrival-and-travel.md`.
//!
//! These are live-DB tests that *also* load the real
//! `data/spaces/harset.nav` / `harset_storagerm.nav`, which is the pairing
//! that makes them worth writing: a pure live-DB test would happily pass on
//! a coordinate nothing can stand on, and a pure navmesh test would pass on
//! a coordinate the seed no longer contains. Reading the coordinate from the
//! database and asking the mesh about it is the only shape that fails when
//! either half is reverted.
//!
//! **Why the meshes are loaded by path rather than through `SpaceManager`:**
//! `create_space_instance` resolves `data/spaces/{world}.nav` relative to the
//! process CWD, which under `cargo test -p cimmeria-services` is the crate
//! directory, so no space built by the ordinary fixtures ever has a mesh.
//! Same reason `arrival::test_fixture_mesh` exists.

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::NavMesh;

use crate::cell::spawner::{load_respawners, load_stargates};
use crate::test_support::require_db_or_skip;

/// `data/spaces/harset.nav` (world 57), or `None` on a checkout without
/// `data/` — the same self-skip `arrival::test_fixture_mesh` uses.
fn harset_mesh() -> Option<NavMesh> {
    load_mesh("../../data/spaces/harset.nav")
}

/// `data/spaces/harset_storagerm.nav` (world 70).
fn storage_mesh() -> Option<NavMesh> {
    load_mesh("../../data/spaces/harset_storagerm.nav")
}

fn load_mesh(rel: &str) -> Option<NavMesh> {
    let path = std::path::Path::new(rel);
    if !path.exists() {
        return None;
    }
    Some(NavMesh::load(path).unwrap_or_else(|e| panic!("{rel} failed to load: {e}")))
}

fn v(p: [f32; 3]) -> Vector3 {
    Vector3::new(p[0], p[1], p[2])
}

/// PL-A-01. The Harset gate's authored arrival pin must be a point the real
/// `harset.nav` accepts, and the *unpinned* gate row must not be — that
/// second half is the whole reason the four `arrival_*` columns exist.
///
/// Reading both from the database rather than hard-coding them is what makes
/// this a regression guard: drop the four `arrival_*` values from
/// `db/resources/Worlds/Seed/stargates.sql` and `desired_arrival()` falls
/// back to the gate row, which fails the first assertion with the control
/// assertion still green.
///
/// Note this asserts on `is_point_valid` directly rather than through
/// `check_arrival`: world 57 is `navmesh_mode = 'advisory'` (H53), so
/// `check_arrival` answers `Unvalidated` for *any* coordinate there and could
/// not tell a good pin from a bad one. Advisory is a runtime policy about
/// whose word to trust; it is not a licence to pin a coordinate the geometry
/// rejects.
#[tokio::test]
async fn harset_gate_arrival_pin_is_on_the_mesh_and_the_gate_row_is_not() {
    let pool = require_db_or_skip!();
    let Some(mesh) = harset_mesh() else { return };

    let gates = load_stargates(&pool).await.expect("load_stargates");
    let gate = gates
        .get(&3)
        .expect("stargate_id 3 (Harset) must exist in the seed");
    assert_eq!(gate.world_name, "Harset");

    let (pin, yaw) = gate.desired_arrival();
    assert!(
        gate.arrival.is_some(),
        "gate 3 has no arrival pin — PL-A-01 seeded arrival_x/y/z/yaw on \
         db/resources/Worlds/Seed/stargates.sql; without them a traveller \
         lands on the prefab origin 2 m above the plaza dais"
    );
    assert!(
        mesh.is_point_valid(&v(pin)),
        "gate 3's arrival pin {pin:?} is off harset.nav — re-place it from \
         docs/analysis/harset-rebuild/placements/A-arrival-and-travel.md \
         (row PL-A-01) rather than leaving it here"
    );

    // Control: the row the pin replaces. If this ever starts passing the
    // navmesh has been rebuilt and the pin should be re-derived (and this
    // test rewritten) rather than silently kept.
    let row = [gate.x, gate.y, gate.z];
    assert!(
        !mesh.is_point_valid(&v(row)),
        "the raw gate row {row:?} now reads on-mesh — harset.nav has changed \
         under this pin; re-derive PL-A-01 against the new mesh"
    );

    // The facing is derived, not defaulted. Asserted as a DIRECTION rather
    // than a number: yaw is atan2(dx, dz) with 0 = +Z, so the unit facing is
    // (sin yaw, cos yaw), and "away from the gate" means the -Z half. The
    // gate sits at z 38 and every telemetry point is at z 0..20, so a pin
    // facing +Z would have the player arrive staring into the event horizon.
    // Checking the vector also survives a future re-pin that legitimately
    // rotates the arrival a few degrees.
    let (facing_x, facing_z) = (yaw.sin(), yaw.cos());
    assert!(
        facing_z < -0.9 && facing_x.abs() < 0.2,
        "arrival_yaw {yaw} faces ({facing_x:.3}, {facing_z:.3}) — expected \
         roughly -Z, away from the gate and down the plaza"
    );
}

/// PL-A-01 / PL-A-02 together, through the production entry point rather
/// than a raw mesh query.
///
/// `validate_gate_arrival` is what `gate_travel` actually calls, and under
/// `enforce` it is the layer that would replace a bad pin with a respawner.
/// Asserting `Validated` (not merely `is_usable`) pins that the pin stands on
/// its own: a pin that only survives because the fallback caught it is a
/// re-pin waiting to happen, and `is_usable` cannot tell the two apart.
#[tokio::test]
async fn validate_gate_arrival_accepts_the_harset_pin_verbatim() {
    use crate::cell::arrival::{test_insert_navmesh_space, validate_gate_arrival, ArrivalSource};

    let pool = require_db_or_skip!();
    let Some(mesh) = harset_mesh() else { return };

    let gates = load_stargates(&pool).await.expect("load_stargates");
    let gate = gates.get(&3).expect("stargate_id 3 (Harset)").clone();
    let (pin, yaw) = gate.desired_arrival();

    // Deliberately NOT stamped advisory. An unstamped world defaults to
    // `NavmeshMode::Enforce`, which is the only mode in which this call can
    // return anything other than `Unvalidated` — i.e. the only mode in which
    // it can say something about the coordinate at all.
    let mut mgr = crate::test_support::make_space_manager();
    test_insert_navmesh_space(&mut mgr, "Harset", mesh);
    mgr.respawners = load_respawners(&pool).await.expect("load_respawners");

    let out = validate_gate_arrival(&mgr, &gate);
    assert_eq!(
        out.source,
        ArrivalSource::Validated,
        "gate 3's arrival resolved as {:?} at {:?} — the pin itself must be \
         standable, not rescued by the respawner fallback",
        out.source,
        out.position
    );
    assert_eq!(out.position, pin);
    assert_eq!(out.yaw, yaw);
    assert!(out.is_usable());
}

/// PL-A-02 / PL-A-03 / PL-A-04. The three Harset respawner rows H10 left
/// unseeded now exist, are not placeholders, and — where the world has a
/// mesh that is allowed to judge them — stand on it.
///
/// The on-mesh half is not decoration. `nearest_valid_respawner` filters
/// candidates through `is_point_valid`, so an off-mesh respawner row in an
/// enforcing world is indistinguishable from no row at all: the world ends up
/// with no recovery target and an off-mesh arrival there becomes
/// `UnrecoverableOffMesh`. World 70 is the case — it has a mesh and is left at
/// the default `enforce`.
#[tokio::test]
async fn the_three_harset_respawners_exist_and_stand_on_real_ground() {
    let pool = require_db_or_skip!();
    let rows = load_respawners(&pool).await.expect("load_respawners");

    for (id, world) in [
        (20, "Harset"),
        (22, "Harset_Market"),
        (23, "Harset_StorageRm"),
    ] {
        let r = rows
            .iter()
            .find(|r| r.respawner_id == id)
            .unwrap_or_else(|| {
                panic!(
                    "respawner {id} ({world}) is missing — PL-A-02..04 seeded it in \
                     db/resources/Worlds/Seed/respawners.sql; without it a death in \
                     that world respawns the player in place at the death position"
                )
            });
        assert_eq!(r.world_name, world, "respawner {id} moved worlds");
        assert_ne!(
            r.pos,
            [0.0, 0.0, 0.0],
            "respawner {id} ({world}) is an unauthored placeholder — \
             `is_unauthored` filters it straight back out, so it is worse \
             than no row"
        );
    }

    let find = |id: i32| rows.iter().find(|r| r.respawner_id == id).unwrap().pos;

    if let Some(mesh) = harset_mesh() {
        let p = find(20);
        assert!(
            mesh.is_point_valid(&v(p)),
            "respawner 20 {p:?} is off harset.nav — it is the recovery \
             candidate for every off-mesh world-57 arrival, so an off-mesh \
             row leaves world 57 with none"
        );
    }
    if let Some(mesh) = storage_mesh() {
        let p = find(23);
        assert!(
            mesh.is_point_valid(&v(p)),
            "respawner 23 {p:?} is off harset_storagerm.nav — world 70 is \
             left at the default `enforce`, so an off-mesh row is filtered \
             out and the world has no recovery target"
        );
    }
    // Row 22 (world 69 Harset_Market) has no navmesh to check against —
    // there is no `harset_market.nav`. Its floor evidence is obj_slab only
    // and the seed comment says so. Deliberately unasserted rather than
    // asserted against a mesh that does not exist.
}

/// PL-A-05, a **characterization** guard, not a regression guard: it pins the
/// finding that four of Harset's five ring pads have no navmesh at the height
/// they actually sit at, and that `navmesh_mode = 'advisory'` is the only
/// thing keeping those four rings alive.
///
/// The pad rows themselves are correct and are deliberately NOT changed:
/// `obj_slab` finds an up-facing ring-platform surface within 0.04 m of every
/// one of the five authored `y` values (a disc ~1.1 m above the surrounding
/// floor). What is missing is mesh, not ground — `harset.nav`'s nearest
/// polygon to pads 5/6/7/8 is 9 to 238 m away vertically, in one case on a
/// different storey entirely.
///
/// Consequence, and the reason this is worth a test: the day someone flips
/// world 57 to `enforce` without rebuilding the mesh, `runtime::tick` starts
/// aborting every trip to those four pads and `audit_ring_pads` starts
/// reporting them at boot. This test fails first and says so.
#[tokio::test]
async fn four_of_the_five_harset_ring_pads_survive_only_because_world_57_is_advisory() {
    use crate::cell::arrival::{check_arrival, test_insert_navmesh_space, ArrivalCheck};
    use crate::cell::ring_transport::load_ring_regions;

    let pool = require_db_or_skip!();
    let Some(mesh) = harset_mesh() else { return };

    let regions = load_ring_regions(&pool).await.expect("load_ring_regions");

    let mut mgr = crate::test_support::make_space_manager();
    test_insert_navmesh_space(&mut mgr, "Harset", mesh);

    // Unstamped => `Enforce`. This is the hypothetical, not today's world.
    let mut on_mesh = Vec::new();
    let mut off_mesh = Vec::new();
    for id in [4, 5, 6, 7, 8] {
        let r = regions
            .get(&id)
            .unwrap_or_else(|| panic!("ring region {id} must exist in the seed"));
        assert_eq!(r.world_name, "Harset");
        assert_ne!(
            [r.x, r.y, r.z],
            [0.0, 0.0, 0.0],
            "ring pad {id} is at the world origin"
        );
        match check_arrival(&mgr, "Harset", [r.x, r.y, r.z]) {
            ArrivalCheck::Validated => on_mesh.push(id),
            ArrivalCheck::OffMesh => off_mesh.push(id),
            ArrivalCheck::Unvalidated => panic!(
                "ring pad {id} came back Unvalidated from an enforcing world \
                 that has a mesh — the fixture did not graft the mesh"
            ),
        }
    }
    assert_eq!(
        (on_mesh.as_slice(), off_mesh.as_slice()),
        ([4].as_slice(), [5, 6, 7, 8].as_slice()),
        "the Harset ring-pad mesh coverage has changed. If harset.nav was \
         rebuilt this is good news: re-run the obj_slab check in \
         docs/analysis/harset-rebuild/placements/A-arrival-and-travel.md \
         (row PL-A-05) and update this expectation. If a pad ROW was edited \
         instead, revert it — all five rows sit within 0.04 m of their \
         authored ring platform and the mesh is what is wrong."
    );

    // And today: world 57 is advisory, so none of them is refused and
    // `audit_ring_pads` prints nothing at boot. That silence is the thing a
    // reader would otherwise mistake for "the pads are fine".
    mgr.stamp_world_rows(&std::collections::HashMap::from([(
        "Harset".to_string(),
        crate::cell::spawner::WorldRow {
            world_id: 57,
            navmesh_mode: crate::cell::space_manager::NavmeshMode::Advisory,
        },
    )]));
    for id in [4, 5, 6, 7, 8] {
        let r = &regions[&id];
        assert_eq!(
            check_arrival(&mgr, "Harset", [r.x, r.y, r.z]),
            ArrivalCheck::Unvalidated,
            "ring pad {id} is refused even in advisory mode"
        );
    }
    assert!(
        crate::cell::ring_transport::audit_ring_pads(&regions, &mgr).is_empty(),
        "audit_ring_pads reports offenders in advisory mode — the boot log \
         must stay quiet about a mesh the server has already declared \
         untrustworthy"
    );
}
