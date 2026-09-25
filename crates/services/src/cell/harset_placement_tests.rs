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

/// `data/spaces/harset_market.nav` (world 69), first built by NA26.
fn market_mesh() -> Option<NavMesh> {
    load_mesh("../../data/spaces/harset_market.nav")
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

/// Point set `Harset.Stargate` — the gate's `REGION_FLAG_Stargate` volume.
const HARSET_STARGATE_SET: i32 = 1001;

/// Samples on the ring of the agent-radius disc check; the centre is the
/// thirteenth.
const DISC_SAMPLES: usize = 12;

/// Is the whole disc of `radius` around `c` on the mesh? The centre plus
/// [`DISC_SAMPLES`] points on the ring, the same 13-sample method the
/// placement ledger (METHOD.md) uses. A centre that is merely within the
/// containment tolerance of a mesh edge fails here.
fn disc_on_mesh(mesh: &NavMesh, c: [f32; 3], radius: f32) -> Result<(), Vec<[f32; 3]>> {
    let mut off = Vec::new();
    if !mesh.is_point_valid(&v(c)) {
        off.push(c);
    }
    for i in 0..DISC_SAMPLES {
        let a = i as f32 * std::f32::consts::TAU / DISC_SAMPLES as f32;
        let p = [c[0] + radius * a.cos(), c[1], c[2] + radius * a.sin()];
        if !mesh.is_point_valid(&v(p)) {
            off.push(p);
        }
    }
    if off.is_empty() {
        Ok(())
    } else {
        Err(off)
    }
}

/// PL-A-01 as revised by NA29. The Harset gate carries no arrival pin, so a
/// traveller arrives on the gate row — what the 2009 server did — and this
/// pins the three things that make that safe.
///
/// 1. **On the mesh.** The row is the gate prefab's origin, standing on the
///    dais. The 2012 `harset.nav` had no polygon there (4.60 m away), which
///    is the only reason PL-A-01 ever pinned a plaza point; the NA26 mesh has
///    one 0.04 m under it. The whole agent-radius disc must be on-mesh, not
///    just the centre, so a rebuild that trims the dais to a sliver fails.
/// 2. **Facing out of the gate.** yaw is atan2(dx, dz) with 0 = +Z; the gate
///    is at z 38 and the plaza at z 0..20, so the row's facing must be -Z.
/// 3. **Inert inside the gate volume.** The row is INSIDE point set 1001
///    `Harset.Stargate` (0.72 m off its axis, radius 2.5), so the client's
///    first region hint after arrival is an enter on the gate volume. That
///    is a no-op today for three reasons, two of which are data and are
///    asserted here: no content chain triggers on the region's tag, and the
///    region is not a ring pad. The third — the gate dial is per entity, so
///    an arriving traveller never holds one, whoever else has the gate open
///    — is behaviour, pinned by
///    `gate_travel::tests::dial_timer::a_traveller_arriving_while_another_player_holds_an_open_dial_is_not_crossed`.
///    Author an arrival-side trigger on that volume and this fails, which is
///    the point: every traveller would fire it on landing.
///
/// Why `is_point_valid` and not `check_arrival`: world 57 is
/// `navmesh_mode = 'advisory'` (H53), so `check_arrival` answers
/// `Unvalidated` for any coordinate there and could not tell a good arrival
/// from a bad one.
#[tokio::test]
async fn harset_gate_arrival_is_the_gate_row_on_the_mesh_and_inert_in_the_gate_volume() {
    use crate::cell::space_manager::REGION_FLAG_STARGATE;
    use crate::cell::spawner::{is_point_in_region, load_regions_from_db};

    let pool = require_db_or_skip!();
    let Some(mesh) = harset_mesh() else { return };

    let gates = load_stargates(&pool).await.expect("load_stargates");
    let gate = gates
        .get(&3)
        .expect("stargate_id 3 (Harset) must exist in the seed");
    assert_eq!(gate.world_name, "Harset");

    // (1) The arrival is the gate row.
    let row = [gate.x, gate.y, gate.z];
    assert!(
        gate.arrival.is_none(),
        "gate 3 carries an arrival pin {:?} again — NA29 dropped PL-A-01's pin so \
         travellers land on the gate row (the 2009 behaviour). A re-pin is a \
         placement decision: record it in \
         docs/analysis/harset-rebuild/placements/A-arrival-and-travel.md and \
         rewrite this guard",
        gate.arrival
    );
    let (arrival, yaw) = gate.desired_arrival();
    assert_eq!(arrival, row, "an unpinned gate must arrive on its own row");

    // (1) On the mesh, whole disc.
    if let Err(off) = disc_on_mesh(&mesh, arrival, 0.6) {
        panic!(
            "the Harset gate row {arrival:?} is no longer standable on harset.nav: \
             {} of {} disc samples at r = 0.6 are off-mesh ({off:?}). The NA26 \
             mesh has the gate dais; a rebuild lost it. Fix the build, or re-pin \
             stargates.arrival_* on the plaza (PL-A-01) until it is fixed",
            off.len(),
            DISC_SAMPLES + 1
        );
    }

    // (2) Facing out of the gate, down the plaza.
    let (facing_x, facing_z) = (yaw.sin(), yaw.cos());
    assert!(
        facing_z < -0.9 && facing_x.abs() < 0.2,
        "gate 3's yaw {yaw} faces ({facing_x:.3}, {facing_z:.3}) — expected \
         roughly -Z, out of the gate and down the plaza"
    );

    // (3) The volume the arrival lands in, and that nothing is keyed on it.
    let regions = load_regions_from_db(&pool).await.expect("load regions");
    let volume = regions
        .iter()
        .find(|r| r.set_id == HARSET_STARGATE_SET)
        .expect("point set 1001 Harset.Stargate must load as an AreaSet");
    assert!(
        volume.flags & REGION_FLAG_STARGATE != 0,
        "point set 1001 lost REGION_FLAG_Stargate — the gate can no longer be crossed"
    );
    // Control, not the claim: the arrival really is inside the volume, so the
    // two "nothing keyed on it" assertions below are about a region every
    // traveller enters. If a later edit moves the row out of the volume
    // they stop being load-bearing, and this says so.
    assert!(
        is_point_in_region(&volume.points, arrival),
        "the Harset gate row {arrival:?} is no longer inside {} — the inertness \
         checks below guard a volume travellers no longer land in; revisit this \
         test",
        volume.name
    );

    let keyed: Vec<(i32, String)> = sqlx::query_as(
        "SELECT chain_id, event_type FROM resources.content_triggers \
         WHERE event_key = $1 ORDER BY chain_id",
    )
    .bind(&volume.name)
    .fetch_all(&pool)
    .await
    .expect("query must succeed");
    assert!(
        keyed.is_empty(),
        "content chains trigger on {} (chain, event): {keyed:?}. Every gate \
         traveller arrives inside that volume, so an enter_region chain there \
         fires on every arrival to Harset. Key it on a region outside the gate, \
         or re-pin the arrival out of the volume",
        volume.name
    );

    let ring_pads: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.ring_transport_regions WHERE point_set_id = $1",
    )
    .bind(HARSET_STARGATE_SET)
    .fetch_one(&pool)
    .await
    .expect("query must succeed");
    assert_eq!(
        ring_pads, 0,
        "point set 1001 is also a ring pad — an arriving traveller would stand on it"
    );
}

/// PL-A-01 / PL-A-02 together, through the production entry point rather
/// than a raw mesh query.
///
/// `validate_gate_arrival` is what `gate_travel` actually calls, and under
/// `enforce` it is the layer that would replace an unstandable arrival with
/// a respawner. Asserting `Validated` (not merely `is_usable`) pins that the
/// gate row stands on its own: an arrival that only survives because the
/// fallback caught it would put every traveller on the respawner instead of
/// the gate, and `is_usable` cannot tell the two apart.
#[tokio::test]
async fn validate_gate_arrival_accepts_the_harset_gate_row_verbatim() {
    use crate::cell::arrival::{test_insert_navmesh_space, validate_gate_arrival, ArrivalSource};

    let pool = require_db_or_skip!();
    let Some(mesh) = harset_mesh() else { return };

    let gates = load_stargates(&pool).await.expect("load_stargates");
    let gate = gates.get(&3).expect("stargate_id 3 (Harset)").clone();
    let (arrival, yaw) = gate.desired_arrival();

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
        "gate 3's arrival resolved as {:?} at {:?} — the gate row itself must be \
         standable, not rescued by the respawner fallback",
        out.source,
        out.position
    );
    assert_eq!(out.position, arrival);
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
            "respawner 23 {p:?} is off harset_storagerm.nav — it is world \
             70's only recovery target, and an off-mesh row is filtered out \
             the day the world goes back to `enforce`"
        );
    }
    // Row 22 (world 69 Harset_Market) was placed from obj_slab alone, before
    // world 69 had a mesh. NA26 built `harset_market.nav`, which confirms it.
    if let Some(mesh) = market_mesh() {
        let p = find(22);
        assert!(
            mesh.is_point_valid(&v(p)),
            "respawner 22 {p:?} is off harset_market.nav — it is world 69's \
             only recovery target"
        );
    }
}

/// PL-A-05, a **characterization** guard, not a regression guard: it pins
/// which of Harset's five ring pads the shipped mesh covers at the height
/// they actually sit at.
///
/// The pad rows themselves are correct and are deliberately NOT changed.
/// `obj_slab` finds an up-facing ring-platform surface within 0.04 m of every
/// one of the five authored `y` values (a disc ~1.1 m above the surrounding
/// floor). The 2012 `harset.nav` covered only pad 4 — its nearest polygon to
/// pads 5/6/7/8 was 9 to 238 m away vertically — so `navmesh_mode =
/// 'advisory'` was the only thing keeping those four rings alive. The NA26
/// rebuild (humanoid agent 1.8/0.6, 372 components) puts **all five** on the
/// mesh, as the rebuilt meshes probed for PL-A-05 predicted.
///
/// Why it is still worth a test: if a future mesh loses a pad again and
/// someone flips world 57 to `enforce`, `runtime::tick` aborts every trip to
/// that pad and `audit_ring_pads` reports it at boot. This test fails first
/// and names the pad.
#[tokio::test]
async fn all_five_harset_ring_pads_are_on_the_mesh_and_advisory_refuses_none() {
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
        ([4, 5, 6, 7, 8].as_slice(), [].as_slice()),
        "the Harset ring-pad mesh coverage has changed: the NA26 harset.nav \
         covers all five pads. If harset.nav was rebuilt, the new mesh has lost \
         a pad — fix the build (docs/engine/navmesh-build-pipeline.md) rather \
         than this expectation, and record it at row PL-A-05 in \
         docs/analysis/harset-rebuild/placements/A-arrival-and-travel.md. If a \
         pad ROW was edited instead, revert it — all five rows sit within \
         0.04 m of their authored ring platform."
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
