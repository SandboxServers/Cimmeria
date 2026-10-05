//! Seed-vs-navmesh guards for the Debug Area gate (stargate 29, world 1300,
//! packet DA-07): the row, its arrival pin, its gate volume and its DHD.
//!
//! Live-DB tests that also load the real `data/spaces/ihpet_crater_light.nav`,
//! the mesh world 1300 reads (D-DA5): a pure DB test would pass on a
//! coordinate nothing can stand on, a pure mesh test on a coordinate the seed
//! no longer holds. Loaded by path for the reason `harset_placement_tests`
//! gives (the space loader resolves `.nav` files against the process CWD).

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::NavMesh;

use crate::cell::spawner::{is_point_in_region, load_regions_from_db, load_stargates};
use crate::test_support::require_db_or_skip;

const HUB: i32 = 29;
/// `Ihpet Crater (SGU)`, world 73: the same gate prop on the same map.
const IHPET_CRATER_LIGHT_GATE: i32 = 20;
/// The Z1 arrival (respawner 130, docs/analysis/debug-area/README.md).
const Z1: [f32; 3] = [251.0, 8.0, -962.0];
/// `point_sets` row for the Debug Area gate volume.
const DEBUG_AREA_GATE_VOLUME: i32 = 13800;
/// The Debug Area DHD (`spawnlist`).
const DEBUG_AREA_DHD_SPAWN: i32 = 13800;
/// `INT_DHD` in `entity_templates.interaction_type`.
const INT_DHD: i64 = 16;

fn ihpet_crater_light_mesh() -> Option<NavMesh> {
    let path = std::path::Path::new("../../data/spaces/ihpet_crater_light.nav");
    path.exists()
        .then(|| NavMesh::load(path).expect("ihpet_crater_light.nav loads"))
}

/// The centre plus twelve points on the agent-radius ring, all on the mesh.
fn disc_on_mesh(mesh: &NavMesh, c: [f32; 3], radius: f32) -> Vec<[f32; 3]> {
    let mut off = Vec::new();
    let mut probe = |p: [f32; 3]| {
        if !mesh.is_point_valid(&Vector3::new(p[0], p[1], p[2])) {
            off.push(p);
        }
    };
    probe(c);
    for i in 0..12 {
        let a = i as f32 * std::f32::consts::TAU / 12.0;
        probe([c[0] + radius * a.cos(), c[1], c[2] + radius * a.sin()]);
    }
    off
}

/// Row 29 is the Ihpet_Crater_Light gate prop under world 1300, the only
/// dial hub, with an arrival pin on Z1 that is standable, faces away from
/// the gate and lies outside the gate's own volume.
///
/// Fails if the row, its hub flag, the pin or the volume are reverted, or if
/// a second hub appears.
#[tokio::test]
async fn live_db_debug_area_gate_is_the_ihpet_prop_the_only_hub_and_arrives_on_z1() {
    let pool = require_db_or_skip!();
    let gates = load_stargates(&pool).await.expect("load_stargates");
    let hub = gates.get(&HUB).expect("stargate 29 (Debug Area) is seeded");
    let ihpet = gates
        .get(&IHPET_CRATER_LIGHT_GATE)
        .expect("stargate 20 is seeded");

    assert_eq!(hub.world_name, "DebugArea");
    assert!(hub.debug_dial_hub, "gate 29 must be outbound only");
    assert_eq!(
        (
            hub.x,
            hub.y,
            hub.z,
            hub.yaw,
            hub.address_origin,
            hub.event_set_id
        ),
        (
            ihpet.x,
            ihpet.y,
            ihpet.z,
            ihpet.yaw,
            ihpet.address_origin,
            ihpet.event_set_id
        ),
        "gate 29 is gate 20's prop: same transform, origin glyph and event set"
    );
    let mut hubs: Vec<i32> = gates
        .iter()
        .filter(|(_, g)| g.debug_dial_hub)
        .map(|(id, _)| *id)
        .collect();
    hubs.sort_unstable();
    assert_eq!(hubs, vec![HUB], "exactly one dial hub");

    let (arrival, yaw) = hub.desired_arrival();
    assert_eq!(
        arrival, Z1,
        "the `.gotolocation DebugArea` entry point is Z1"
    );
    // yaw 0 faces +Z; the gate is at z -989.8, Z1 at z -962.
    assert!(
        yaw.cos() > 0.9 && hub.z < arrival[2],
        "faces away from the gate"
    );

    let regions = load_regions_from_db(&pool).await.expect("load regions");
    let volume = regions
        .iter()
        .find(|r| r.set_id == DEBUG_AREA_GATE_VOLUME)
        .expect("point set 13800 DebugArea.Stargate is seeded");
    assert_eq!(volume.world_name, "DebugArea");
    assert!(
        volume.flags & 2 != 0,
        "REGION_FLAG_Stargate: without it a dial travels at once, no gate animation"
    );
    assert!(
        is_point_in_region(&volume.points, [hub.x, hub.y, hub.z]),
        "control: the gate prop is inside its own volume"
    );
    assert!(
        !is_point_in_region(&volume.points, arrival),
        "the arrival must not land inside the gate volume"
    );

    let Some(mesh) = ihpet_crater_light_mesh() else {
        return;
    };
    let off = disc_on_mesh(&mesh, arrival, 0.6);
    assert!(
        off.is_empty(),
        "Z1 {arrival:?} is not standable on ihpet_crater_light.nav: {off:?}"
    );
}

/// The Debug Area DHD is spawned on world 1300 from a template that carries
/// INT_DHD, so right-clicking it reaches `try_open_dhd`; and every DHD prop
/// in the seed stands on a world with a stargate row, or it opens nothing.
/// Reverting template 1 to `interaction_type = 0` fails the first half.
#[tokio::test]
async fn live_db_every_seeded_dhd_opens_and_the_debug_area_has_one() {
    let pool = require_db_or_skip!();

    let debug: Option<(i32, String, i64)> = sqlx::query_as(
        "SELECT s.world_id, s.tag::text, t.interaction_type FROM resources.spawnlist s \
           JOIN resources.entity_templates t ON t.template_id = s.template_id \
          WHERE s.spawn_id = $1",
    )
    .bind(DEBUG_AREA_DHD_SPAWN)
    .fetch_optional(&pool)
    .await
    .expect("query");
    let (world_id, tag, interaction) = debug.expect("spawn 13800 (Debug Area DHD) is seeded");
    assert_eq!((world_id, tag.as_str()), (1300, "DebugArea_DHD"));
    assert!(
        interaction & INT_DHD != 0,
        "the Debug Area DHD's template lacks INT_DHD — it cannot be right-clicked"
    );

    let gateless: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT s.spawn_id, s.world_id FROM resources.spawnlist s \
           JOIN resources.entity_templates t ON t.template_id = s.template_id \
          WHERE t.interaction_type & $1 <> 0 \
            AND NOT EXISTS (SELECT 1 FROM resources.stargates g WHERE g.world_id = s.world_id) \
          ORDER BY s.spawn_id",
    )
    .bind(INT_DHD)
    .fetch_all(&pool)
    .await
    .expect("query");
    assert!(
        gateless.is_empty(),
        "DHD props on worlds with no stargate row (spawn, world): {gateless:?}"
    );

    let dark_dhds: Vec<(i32, String)> = sqlx::query_as(
        "SELECT s.spawn_id, s.tag::text FROM resources.spawnlist s \
           JOIN resources.entity_templates t ON t.template_id = s.template_id \
          WHERE t.static_mesh LIKE '%DHD%' AND t.interaction_type & $1 = 0 \
          ORDER BY s.spawn_id",
    )
    .bind(INT_DHD)
    .fetch_all(&pool)
    .await
    .expect("query");
    assert!(
        dark_dhds.is_empty(),
        "DHD props that cannot be right-clicked (spawn, tag): {dark_dhds:?}"
    );
}

/// The Debug Area gate's six glyphs belong to no other row, so nothing a
/// player dials can resolve to it.
#[tokio::test]
async fn live_db_the_debug_area_address_is_unique() {
    let pool = require_db_or_skip!();
    let clashes: Vec<i32> = sqlx::query_scalar(
        "SELECT o.stargate_id FROM resources.stargates o \
           JOIN resources.stargates h ON h.stargate_id = $1 \
          WHERE o.stargate_id <> h.stargate_id \
            AND (o.address1, o.address2, o.address3, o.address4, o.address5, o.address6) \
              = (h.address1, h.address2, h.address3, h.address4, h.address5, h.address6)",
    )
    .bind(HUB)
    .fetch_all(&pool)
    .await
    .expect("query");
    assert!(clashes.is_empty(), "gate 29's address is also: {clashes:?}");
}
