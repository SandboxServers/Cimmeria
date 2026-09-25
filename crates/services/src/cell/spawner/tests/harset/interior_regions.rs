//! Live-DB regression guards for the interior named regions seeded by packet
//! H15 / placement cluster PL-C — `Harset_CmdCenter.Lab` (2120),
//! `Harset_Market.Marketplace` (2121) and `Harset_StorageRm.Storage` (2122).
//!
//! Seed-data guards over `db/resources/Events/Seed/point_sets.sql` and
//! `point_set_points.sql`. What they pin:
//!
//! - **The three sets load as `AreaSet` in the right world.**
//!   `load_regions_from_db` filters on `type = 'AreaSet'` and inner-joins
//!   `resources.worlds`; a wrong `type` or an unknown `world_id` drops the
//!   region silently and every `enter_region` on the key then matches nothing.
//! - **The keys are exact.** Region keys are compared byte for byte and carry
//!   the world prefix plus a dot so the cross-file linters can see them.
//! - **The footprint really contains the room.** A four-corner box that got a
//!   sign wrong still loads; only a containment probe catches it.
//! - **The Storage box is on the navmesh.** World 70 is the one interior with
//!   a shipped mesh, so it is the one place the "is this floor actually
//!   walkable" claim can be tested rather than asserted.
//! - **The Storage box matches where real players stood.** World 70 is also the
//!   only interior with telemetry. Twenty server-accepted player positions say
//!   which parts of the room people use and at what height, so the footprint
//!   and the ceiling are both checked against evidence that is independent of
//!   the `obj_slab` geometry they were derived from.
//!
//! Worlds 68 and 69 have **neither** a navmesh nor telemetry, so nothing here
//! can check their two regions beyond load, key, world and containment.

use super::*;
use crate::cell::spawner::{is_point_in_region, load_regions_from_db, region_contains_xz};

/// World 70's twenty distinct **server-accepted** player positions, from three
/// days of SigNoz `movement.validation_reject` logs (the `last_valid_*` field
/// on each reject: wherever the player *was* when a move was refused, which the
/// server had already accepted). Cleaned list:
/// `$O\harset\harset_storagerm_last_valid_probes.txt`.
///
/// **Only the cleaned list counts as evidence.** Most of world 70's reject
/// volume is synthetic — (50, 2, 50) x7,958, (2, 30, 50) x2,765,
/// (2, 2, 50) x1,744, (2, 2, 100) x1,617 — an entity parked at a default or
/// test position and refused on every packet. Those are excluded upstream; see
/// `harset_suspicious_points.txt`.
///
/// Split by which vertical band they landed in, because that is what
/// `Harset_StorageRm.Storage`'s ceiling has to discriminate. The seven
/// `PEN_FLOOR` entries are the ones on navmesh component 36, the pen-grid
/// floor; the rest are under it, on the upper arrival deck, or on interior
/// gantries and catwalks.
const STORAGE_ACCEPTED_PEN_FLOOR: [[f32; 3]; 7] = [
    [37.07, 1.25, 82.27],
    [75.67, 1.32, 83.43],
    [56.27, 1.58, 62.65],
    [68.93, 1.40, 63.43],
    [73.87, 1.53, 73.31],
    [76.45, 1.39, 78.54],
    [77.24, 1.41, 81.23],
];

/// Accepted positions inside the Storage footprint in XZ but **not** on the pen
/// floor, as `(position, what it is)`. The region must exclude every one of
/// them: a player on the arrival deck must not read as "in Storage" before
/// descending, or the `enter_region` edge never fires for them.
const STORAGE_ACCEPTED_NOT_PEN_FLOOR: [([f32; 3], &str); 6] = [
    (
        [51.71, -1.68, 53.30],
        "the -1.2 m under-layer below the pen floor",
    ),
    (
        [52.26, 7.06, 43.36],
        "the upper arrival deck, where it overhangs to z 43.4",
    ),
    ([65.63, 7.69, 46.53], "the y 8-9 gantry over the pen grid"),
    ([69.77, 9.68, 58.25], "a y 8-9 gantry"),
    ([63.20, 13.03, 45.05], "a y 11-13 catwalk"),
    ([63.20, 17.68, 45.05], "the y 15-17 roof truss walkway"),
];

/// The three PL-C interior regions, as
/// `(set_id, key, world_id, world_name, interior probe (x, y, z))`.
///
/// The probe is a point that must be inside: the Lab's console row, the
/// Market's floor centre, the Storage pen grid's centre.
const INTERIOR_REGIONS: [(i32, &str, i32, &str, [f32; 3]); 3] = [
    (
        2120,
        "Harset_CmdCenter.Lab",
        68,
        "Harset_CmdCenter",
        [34.8, 0.32, -25.5],
    ),
    (
        2121,
        "Harset_Market.Marketplace",
        69,
        "Harset_Market",
        [65.0, 3.6, 65.0],
    ),
    (
        2122,
        "Harset_StorageRm.Storage",
        70,
        "Harset_StorageRm",
        [52.0, 0.3, 66.75],
    ),
];

/// Points that must be OUTSIDE their region, with why.
///
/// Each is the room a visitor arrives from. They are excluded on purpose so
/// that an `enter_region` trigger on the key is a real edge crossing rather
/// than a state the player is already standing in when the world loads — the
/// `player_loaded` edge-trigger shape that has bitten three Harset chains.
const ARRIVAL_SIDE_EXCLUSIONS: [(i32, &str, [f32; 3], &str); 3] = [
    (
        2120,
        "Harset_CmdCenter.Lab",
        [0.0, 0.32, 20.0],
        "the cross hall the lab's two doorways open onto",
    ),
    (
        2121,
        "Harset_Market.Marketplace",
        [19.0, 4.8, 15.0],
        "the torch-lit 4.80 vestibule in the south-west corner",
    ),
    (
        2122,
        "Harset_StorageRm.Storage",
        [58.0, 5.2, 20.0],
        "the upper wing at floor 5.10 (navmesh component 6)",
    ),
];

/// Did a path from `from` actually *arrive* at `to`, or stop short?
///
/// `NavMesh::find_path` wraps `dtNavMeshQuery::findPath`, which returns a
/// partial corridor ending at the closest reachable polygon rather than failing
/// when the destination is on a different connected component. So `is_some()`
/// tells you only that both endpoints landed on *some* polygon. Comparing the
/// last waypoint against the request is what turns it into a reachability test.
///
/// The 2 m tolerance is the slack Detour itself introduces: the end point is
/// snapped onto its polygon and then string-pulled, so an arriving path's last
/// waypoint is near the request but rarely equal to it. A partial path, by
/// contrast, stops at a component boundary — tens of metres out for every case
/// this file cares about.
#[cfg(test)]
fn path_reaches(
    mesh: &cimmeria_entity::navigation::NavMesh,
    from: &cimmeria_common::Vector3,
    to: &cimmeria_common::Vector3,
) -> bool {
    const ARRIVAL_TOLERANCE_M: f32 = 2.0;
    let Some(path) = mesh.find_path(from, to) else {
        return false;
    };
    let Some(last) = path.last() else {
        return false;
    };
    let (dx, dz) = (last.x - to.x, last.z - to.z);
    (dx * dx + dz * dz).sqrt() <= ARRIVAL_TOLERANCE_M
}

/// **All three sets load, as `AreaSet`, in the right world, with four points.**
///
/// Goes through `load_regions_from_db` rather than a raw `SELECT` because the
/// loader is where the `type = 'AreaSet'` filter and the `resources.worlds`
/// inner join live: a region with the wrong `type`, or with a `world_id` that
/// has no world row, disappears here and nowhere else.
#[tokio::test]
async fn the_three_interior_regions_load_with_four_corners_each() {
    let pool = require_db_or_skip!();

    let regions = load_regions_from_db(&pool)
        .await
        .expect("load_regions_from_db");

    for (set_id, key, _world_id, world_name, _probe) in INTERIOR_REGIONS {
        let region = regions
            .iter()
            .find(|r| r.set_id == set_id)
            .unwrap_or_else(|| {
                panic!(
                    "point set {set_id} (`{key}`) did not survive \
                     `load_regions_from_db`. The loader filters \
                     `type = 'AreaSet'` and inner-joins `resources.worlds`, \
                     so the row exists but is either the wrong type or points \
                     at a world id with no row."
                )
            });

        assert_eq!(
            region.name, key,
            "point set {set_id} must carry the exact key `{key}` -- region \
             keys are matched byte for byte by `enter_region`",
        );
        assert_eq!(
            region.world_name, world_name,
            "point set {set_id} (`{key}`) is in the wrong world",
        );
        assert_eq!(
            region.points.len(),
            4,
            "point set {set_id} (`{key}`) must have four corner points; \
             `region_contains_xz` returns false for fewer than three, so a \
             truncated box is a region that silently contains nothing",
        );
    }
}

/// **Each box actually contains the room it is named for, and excludes the
/// room its visitors arrive from.**
///
/// The positive half catches a sign error or a transposed corner that still
/// loads as a well-formed four-point box. The negative half is the design
/// claim: these are edge triggers, and a box that swallowed the approach
/// corridor would never fire one.
#[tokio::test]
async fn interior_region_boxes_contain_their_room_and_exclude_the_approach() {
    let pool = require_db_or_skip!();

    let regions = load_regions_from_db(&pool)
        .await
        .expect("load_regions_from_db");
    let find = |set_id: i32| {
        regions
            .iter()
            .find(|r| r.set_id == set_id)
            .unwrap_or_else(|| panic!("point set {set_id} must load"))
    };

    for (set_id, key, _world_id, _world_name, probe) in INTERIOR_REGIONS {
        assert!(
            region_contains_xz(&find(set_id).points, probe[0], probe[2]),
            "`{key}` ({set_id}) must contain {probe:?} -- that is the point \
             the room was identified from, so a box that misses it is a box \
             around the wrong place",
        );
    }

    for (set_id, key, probe, why) in ARRIVAL_SIDE_EXCLUSIONS {
        assert!(
            !region_contains_xz(&find(set_id).points, probe[0], probe[2]),
            "`{key}` ({set_id}) must NOT contain {probe:?} -- {why}. A player \
             who is already inside when the world loads never raises \
             `enter_region`, so swallowing the approach silently disables \
             every trigger on this key",
        );
    }
}

/// **Every real player position on the Storage floor is inside the region, and
/// every real position that is *not* on the floor is outside it.**
///
/// This is the only PL-C coordinate with TELEMETRY evidence — worlds 68 and 69
/// have neither telemetry nor a navmesh — and it is independent of the
/// `obj_slab` geometry the footprint was derived from. Seven real players stood
/// on the pen floor at y 1.25-1.58; if a later edit shrinks the box away from
/// where people actually walked, this fails.
///
/// The negative half is what pins the **ceiling** at 4.00 rather than at the
/// room's 15-17 roof. Thirteen more accepted positions share the footprint in
/// XZ but sit under the floor, on the upper arrival deck where it overhangs to
/// z 43.4, or on gantries and catwalks up to y 17.7. Admitting the arrival deck
/// would mean a player reads as "in Storage" before descending, and then the
/// `enter_region` edge never fires for them — the `player_loaded` shape that has
/// already bitten three Harset chains. No mission step takes place on a
/// catwalk, so the cheap, evidence-backed cut is floor-plus-headroom.
///
/// Uses `is_point_in_region` (the tolerance band the security gate applies,
/// AABB widened 1.5 m on every axis **including Y**) rather than
/// `region_contains_xz`, because the vertical discrimination is the whole point
/// and `region_contains_xz` is Y-blind.
#[tokio::test]
async fn the_storage_region_matches_where_real_players_actually_stood() {
    let pool = require_db_or_skip!();

    let regions = load_regions_from_db(&pool)
        .await
        .expect("load_regions_from_db");
    let storage = regions
        .iter()
        .find(|r| r.set_id == 2122)
        .expect("point set 2122 must load");

    for p in STORAGE_ACCEPTED_PEN_FLOOR {
        assert!(
            is_point_in_region(&storage.points, p),
            "`Harset_StorageRm.Storage` must contain {p:?} -- the server \
             accepted a real player there and it is on navmesh component 36, \
             the pen-grid floor. A region that excludes it is smaller than the \
             room players actually use",
        );
    }

    for (p, what) in STORAGE_ACCEPTED_NOT_PEN_FLOOR {
        assert!(
            !is_point_in_region(&storage.points, p),
            "`Harset_StorageRm.Storage` must NOT contain {p:?} ({what}). It \
             shares the footprint in XZ, so only the ceiling excludes it; \
             raising the ceiling admits the arrival deck and kills the \
             `enter_region` edge for anyone descending into the room",
        );
    }
}

/// **The Storage box stands on one connected piece of walkable navmesh.**
///
/// World 70 ships `data/spaces/harset_storagerm.nav`, so it is the one
/// interior where "this is a real floor" can be tested instead of asserted
/// from the chunk OBJ. The probes are the box centre and its four edge
/// midpoints; the *corners* are deliberately not probed, because the mesh is
/// inset from the walls by the 0.6 m agent radius and each corner resolves to
/// the -1.2 m under-layer rather than to the floor.
///
/// **`is_point_valid` alone is not enough here, and the control below proves
/// it.** `harset_storagerm.nav` has 104 components, three of which overlap this
/// footprint in Y: the pen-grid floor (314 polygons, 3055.7 m2, x[16.1,87.5]
/// z[34.1,99.2] at y 0.2-1.2), a disconnected duplicate 1.4 m beneath it, and
/// an 82,249 m2 outdoor terrain sheet at y 0.2-0.4 spanning x/z[-99,199]. A
/// region dragged clean off the building therefore still answers "on-mesh" at
/// y 0.3 — it just answers from the terrain. Pathing from the centre to every
/// edge is what pins the region to *one* island.
///
/// Note that `find_path(...).is_some()` is **not** that test:
/// `dtNavMeshQuery::findPath` returns a *partial* corridor to the closest
/// reachable polygon when the destination is unreachable, so it answers `Some`
/// across a component boundary. [`path_reaches`] is the connectivity predicate
/// — it checks the last waypoint actually arrived.
///
/// Asserting on the region read back from the DB, not on literals, is what
/// makes this a guard on the seed: shrink or move set 2122 and the probes move
/// with it.
#[tokio::test]
async fn the_storage_region_sits_on_the_shipped_world_70_navmesh() {
    use cimmeria_common::Vector3;
    use cimmeria_entity::navigation::NavMesh;

    let pool = require_db_or_skip!();

    let path = std::path::Path::new("../../data/spaces/harset_storagerm.nav");
    if !path.exists() {
        return; // fixture-less checkout: skip
    }
    let mesh = NavMesh::load(path).expect("load data/spaces/harset_storagerm.nav");

    let regions = load_regions_from_db(&pool)
        .await
        .expect("load_regions_from_db");
    let storage = regions
        .iter()
        .find(|r| r.set_id == 2122)
        .expect("point set 2122 must load");

    let xs: Vec<f32> = storage.points.iter().map(|p| p[0]).collect();
    let zs: Vec<f32> = storage.points.iter().map(|p| p[2]).collect();
    let (x_lo, x_hi) = (
        xs.iter().cloned().fold(f32::INFINITY, f32::min),
        xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
    );
    let (z_lo, z_hi) = (
        zs.iter().cloned().fold(f32::INFINITY, f32::min),
        zs.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
    );
    // The floor Y is the lower of the two Ys in the box: three corners carry
    // the floor and one carries the ceiling (the `GenericRegion.workaround()`
    // convention the seed follows).
    let floor_y = storage
        .points
        .iter()
        .map(|p| p[1])
        .fold(f32::INFINITY, f32::min);

    let (cx, cz) = ((x_lo + x_hi) / 2.0, (z_lo + z_hi) / 2.0);
    // Edge midpoints pulled 1 m inside the box, clear of the mesh's inset.
    let probes: [(&str, f32, f32); 5] = [
        ("centre", cx, cz),
        ("west edge", x_lo + 1.0, cz),
        ("east edge", x_hi - 1.0, cz),
        ("south edge", cx, z_lo + 1.0),
        ("north edge", cx, z_hi - 1.0),
    ];

    let centre = Vector3::new(cx, floor_y, cz);

    for (what, x, z) in probes {
        let p = Vector3::new(x, floor_y, z);
        assert!(
            mesh.is_point_valid(&p),
            "`Harset_StorageRm.Storage` {what} ({x}, {floor_y}, {z}) is off \
             `harset_storagerm.nav` entirely -- not even the terrain sheet \
             reaches it. Either the region moved or its floor Y was taken from \
             the wrong storey",
        );
        assert!(
            path_reaches(&mesh, &centre, &p),
            "`Harset_StorageRm.Storage` {what} ({x}, {floor_y}, {z}) is on the \
             mesh but not reachable from the box centre ({cx}, {floor_y}, \
             {cz}), so the footprint straddles two disconnected components -- \
             the pen-grid floor and either its 1.4 m under-layer or the outdoor \
             terrain sheet. A mission step that fires on this region would then \
             be satisfiable from a place an NPC cannot path to",
        );
    }

    // Control: the terrain sheet answers `is_point_valid` at the same Y as the
    // Storage floor, 100 m outside the building. If this ever starts pathing
    // from the centre, the connectivity assertions above have stopped
    // discriminating and this test is no longer guarding anything.
    let outdoors = Vector3::new(150.0, floor_y, 150.0);
    assert!(
        mesh.is_point_valid(&outdoors),
        "control: {outdoors:?} must read on-mesh (it is on the outdoor terrain \
         sheet) -- that is the false positive the path checks exist to catch",
    );
    assert!(
        !path_reaches(&mesh, &centre, &outdoors),
        "control: {outdoors:?} is outside the building and must NOT be \
         reachable from the Storage floor; if it is, the two components have \
         been joined and the assertions above no longer pin the region to the \
         pen grid",
    );
}
