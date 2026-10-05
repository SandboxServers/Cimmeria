//! Live-DB guards on the Debug Area ring network (world 1300, DA-08).
//!
//! The eight stations are seeded across three files (Events, Worlds and
//! Content seeds) and only work when every link holds: pad row -> point set
//! (the trigger volume) -> event set -> Teleport Out / In sequences -> console
//! spawn -> `interact_tag` chain. These tests pin each link against the loaded
//! database, plus the two map facts the seed depends on: every pad is on the
//! navmesh (or the trip aborts with `ring_pad_off_navmesh`) and stands on the
//! map's floor where client patch `010-debug-area-rings` put the rig.

use std::collections::{BTreeMap, BTreeSet};

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::NavMesh;
use cimmeria_occluder::PagedOccluder;
use sqlx::Row;

use super::regions::{load_ring_regions, RingRegion};
use crate::test_support::require_db_or_skip;

const DEBUG_AREA: i32 = 1300;
/// The live Ihpet Crater, which loads the same patched map.
const IHPET_CRATER_LIGHT: i32 = 73;
const REGIONS: std::ops::RangeInclusive<i32> = 35..=42;
/// Region rows sit this far above the rig's base platform origin (region 1
/// and region 3 do; see `debug_area_rings.sql`).
const PAD_LIFT: f32 = 0.537;
/// The rig's rings rise this high above the base when they play.
const RING_CLEARANCE: f32 = 3.5;

async fn debug_area_regions(pool: &sqlx::PgPool) -> BTreeMap<i32, RingRegion> {
    load_ring_regions(pool)
        .await
        .expect("load_ring_regions must succeed against the seeded DB")
        .into_iter()
        .filter(|(_, r)| r.world_id == DEBUG_AREA)
        .collect()
}

/// Every Debug Area station reaches every other, and nothing else does: the
/// destination list a console opens is the whole Debug Area. A dropped or
/// mistyped id is filtered out by `load_ring_regions` with only a warning, so
/// a station missing from one list would silently vanish from that picker.
#[tokio::test]
async fn debug_area_ring_network_is_fully_connected_live_db() {
    let pool = require_db_or_skip!();
    let regions = debug_area_regions(&pool).await;
    let ids: BTreeSet<i32> = regions.keys().copied().collect();
    assert_eq!(ids, REGIONS.collect(), "world 1300 must hold regions 35-42");
    for (id, r) in &regions {
        assert!(
            r.tag.starts_with("DebugArea_Ring_"),
            "region {id} tag {}",
            r.tag
        );
        assert_eq!(r.required_mission_id, None, "region {id} must be ungated");
        let dests: BTreeSet<i32> = r.destination_ids.iter().copied().collect();
        let others: BTreeSet<i32> = ids.iter().copied().filter(|o| o != id).collect();
        assert_eq!(dests, others, "region {id} must list every other station");
        assert_eq!(
            r.destination_ids.len(),
            7,
            "region {id} lists a station twice"
        );
    }
}

/// Each station's whole chain of rows exists and points at the right thing.
#[tokio::test]
async fn debug_area_ring_stations_are_wired_end_to_end_live_db() {
    let pool = require_db_or_skip!();
    let regions = debug_area_regions(&pool).await;
    assert_eq!(regions.len(), 8);
    let mut paths = BTreeSet::new();
    for (id, r) in &regions {
        // Trigger volume: a world-1300 cylinder centred on the pad.
        let ps = sqlx::query(
            "SELECT s.world_id, s.shape, p.x, p.y, p.z FROM resources.point_sets s \
             JOIN resources.point_set_points p ON p.set_id = s.set_id WHERE s.set_id = $1",
        )
        .bind(r.point_set_id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            ps.len(),
            1,
            "region {id}: point set {} needs one point",
            r.point_set_id
        );
        assert_eq!(
            ps[0].get::<i32, _>("world_id"),
            DEBUG_AREA,
            "region {id} point set world"
        );
        assert_eq!(ps[0].get::<String, _>("shape"), "Cylinder");
        let at = [
            ps[0].get::<f32, _>("x"),
            ps[0].get::<f32, _>("y"),
            ps[0].get::<f32, _>("z"),
        ];
        assert!(
            (at[0] - r.x).abs() < 0.01 && (at[1] - r.y).abs() < 0.01 && (at[2] - r.z).abs() < 0.01,
            "region {id}: trigger volume at {at:?}, pad at ({}, {}, {})",
            r.x,
            r.y,
            r.z
        );

        // Event set: one Teleport Out and one Teleport In, both naming this
        // station's rig in the patched chunk.
        let seqs = sqlx::query(
            "SELECT s.event_id, s.kismet_script_name FROM resources.event_sets_sequences e \
             JOIN resources.sequences s ON s.sequence_id = e.sequence_id \
             WHERE e.event_set_id = $1 ORDER BY s.event_id",
        )
        .bind(r.event_set_id)
        .fetch_all(&pool)
        .await
        .unwrap();
        let events: Vec<i32> = seqs.iter().map(|s| s.get("event_id")).collect();
        assert_eq!(
            events,
            vec![8000, 8001],
            "region {id} event set {}",
            r.event_set_id
        );
        let names: BTreeSet<String> = seqs.iter().map(|s| s.get("kismet_script_name")).collect();
        assert_eq!(
            names.len(),
            1,
            "region {id}: out and in must play the same rig"
        );
        let path = names.into_iter().next().unwrap();
        assert!(
            path.starts_with("Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs."),
            "region {id}: {path}"
        );
        assert!(
            paths.insert(path.clone()),
            "region {id} shares rig {path} with another station"
        );

        // Console: a template-3 spawn beside the pad, and a chain that opens
        // this region's destination list when it is clicked.
        let console_tag = r.tag.trim_end_matches("Region");
        let spawn = sqlx::query(
            "SELECT x, y, z, template_id, world_id FROM resources.spawnlist WHERE tag = $1",
        )
        .bind(console_tag)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            spawn.len(),
            1,
            "region {id}: one console tagged {console_tag}"
        );
        assert_eq!(spawn[0].get::<i32, _>("template_id"), 3);
        assert_eq!(spawn[0].get::<i32, _>("world_id"), DEBUG_AREA);
        let dx = spawn[0].get::<f32, _>("x") - r.x;
        let dz = spawn[0].get::<f32, _>("z") - r.z;
        assert!(
            (dx * dx + dz * dz).sqrt() < r.radius,
            "region {id}: the console must stand on the pad, within {} m",
            r.radius
        );
        let chain_region: Vec<String> = sqlx::query(
            "SELECT a.params->>'regionId' AS region FROM resources.content_chains c \
             JOIN resources.content_triggers t ON t.chain_id = c.chain_id \
             JOIN resources.content_actions a ON a.chain_id = c.chain_id \
             WHERE c.enabled AND c.scope_type = 'space' AND c.scope_id = $1 \
               AND t.event_type = 'interact_tag' AND t.event_key = $2 \
               AND a.action_type = 'trigger_transporter'",
        )
        .bind(DEBUG_AREA)
        .bind(console_tag)
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get("region"))
        .collect();
        assert_eq!(
            chain_region,
            vec![id.to_string()],
            "region {id}: console chain"
        );
    }
}

fn map_file(rel: &str) -> std::path::PathBuf {
    std::path::Path::new("../../data/spaces").join(rel)
}

/// Every pad is a valid arrival on world 1300's navmesh (the client map's,
/// D-DA5), and the rig under it stands on the map's own floor: the occluder
/// (built from the cooked map's collision) has a surface within 0.5 m of the
/// base platform origin and nothing in the 3.5 m the rings rise through.
#[tokio::test]
async fn debug_area_ring_pads_stand_on_the_map_floor_live_db() {
    let pool = require_db_or_skip!();
    let (nav, occ) = (
        map_file("ihpet_crater_light.nav"),
        map_file("ihpet_crater_light.occ"),
    );
    if !nav.exists() || !occ.exists() {
        eprintln!("SKIP: data/spaces/ihpet_crater_light.{{nav,occ}} not present");
        return;
    }
    let mesh = NavMesh::load(&nav).expect("ihpet_crater_light.nav loads");
    let occ = PagedOccluder::from_bytes(std::fs::read(&occ).unwrap())
        .expect("ihpet_crater_light.occ loads");

    let regions = debug_area_regions(&pool).await;
    assert_eq!(regions.len(), 8);
    for (id, r) in &regions {
        assert!(
            mesh.is_point_valid(&Vector3::new(r.x, r.y, r.z)),
            "region {id} ({}) is off the navmesh at ({}, {}, {}): every trip there aborts",
            r.tag,
            r.x,
            r.y,
            r.z
        );
        let base = r.y - PAD_LIFT;
        let column = occ.column(r.x, r.z);
        assert!(
            column.iter().any(|&(_, _, hi)| (hi - base).abs() < 0.5),
            "region {id} ({}): no floor within 0.5 m of the rig base {base}; column {column:?}",
            r.tag
        );
        assert!(
            !column
                .iter()
                .any(|&(_, lo, _)| lo > base + 0.3 && lo < base + RING_CLEARANCE),
            "region {id} ({}): something hangs over the pad; column {column:?}",
            r.tag
        );
    }
}

/// World 73 loads the same patched map, so it renders the eight rigs. None
/// of them may do anything there: no pad, no console, no chain that could
/// open a ring list.
#[tokio::test]
async fn the_live_ihpet_crater_gets_no_ring_stations_live_db() {
    let pool = require_db_or_skip!();
    let pads: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resources.ring_transport_regions WHERE world_id = $1",
    )
    .bind(IHPET_CRATER_LIGHT)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(pads, 0, "world 73 must have no ring pads");
    let consoles: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resources.spawnlist \
         WHERE tag LIKE 'DebugArea\\_Ring\\_%' AND world_id <> $1",
    )
    .bind(DEBUG_AREA)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        consoles, 0,
        "Debug Area ring consoles must spawn only in world 1300"
    );
    let chains: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resources.content_chains c \
         JOIN resources.content_actions a ON a.chain_id = c.chain_id \
         WHERE a.action_type = 'trigger_transporter' \
           AND c.scope_type = 'space' AND c.scope_id = $1",
    )
    .bind(IHPET_CRATER_LIGHT)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(chains, 0, "no ring chain may be scoped to world 73");
}
