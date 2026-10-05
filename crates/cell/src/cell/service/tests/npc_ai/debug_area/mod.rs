//! Debug Area (world 1300, `Ihpet_Crater_Light`) packet DA-03 on the real
//! `ihpet_crater_light.nav` and `.occ`: the faction yard (Z4), the AI
//! behaviour slope (Z5) and the enemy gallery (Z7)
//! (`docs/content/debug-area.md`).
//!
//! The scene is built from the seed files themselves: every world-1300 row of
//! every `spawnlist*.sql`, joined with every `entity_templates*.sql` and point
//! set file. Moving a row, changing a template's faction or radius, another
//! packet seeding an NPC within reach, or rebuilding the mesh or the occluder
//! under a station fails here:
//!
//! - [`placement`]: every DA-03 spawn stands on the navmesh and on the
//!   occluder's terrain (the 3.3 m nav-vs-terrain trap on open ground), the
//!   patrol legs, the wander disc and the leash kite route are walkable, and
//!   the assist trio sees itself.
//! - [`isolation`]: through the production aggro and assist gates, the
//!   gallery never pulls and never rallies, friendly and neutral rows never
//!   engage, the pen engages only a player who walks up to it, the assist
//!   trio rallies exactly one neighbour, and shooting a pinned Jaffa wakes
//!   only that Jaffa.
//! - [`reach`]: no station is in reach of another (other packets' rows and
//!   the world's respawners included), and no NPC that fights NPCs can reach a
//!   DA-03 NPC it would target.
//!
//! Skips on a checkout without `data/spaces/ihpet_crater_light.nav` / `.occ`.

mod isolation;
mod placement;
mod reach;

use std::collections::HashMap;
use std::sync::Arc;

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::{AiState, MobAggression, VaultScope};
use cimmeria_entity::navigation::NavMesh;
use cimmeria_occluder::PagedOccluder;

use super::castle_standoff::{parse_insert, seed};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::SpawnRecord;

/// The world DA-01 adds; the name only has to match between the scene and
/// the records here.
const WORLD: &str = "DebugArea";
const WORLD_ID: &str = "1300";

/// Entity id of a seed row: 100 000 + spawn id.
fn eid(spawn_id: i32) -> u32 {
    100_000 + spawn_id as u32
}

/// The player every scene connects.
const PLAYER: u32 = 1;

fn unquote(v: &str) -> String {
    v.trim_matches('\'').replace("''", "'")
}

fn opt<T: std::str::FromStr>(v: Option<&String>) -> Option<T> {
    v.filter(|s| s.as_str() != "NULL")
        .and_then(|s| s.parse().ok())
}

/// The seed files in `dir` (relative to the repo root) whose name starts with
/// `prefix`: the base file and every packet's own file, so rows another
/// packet adds to world 1300 are read without listing them here.
fn seed_files(dir: &str, prefix: &str) -> Vec<String> {
    let abs = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(dir);
    let mut out: Vec<String> = std::fs::read_dir(&abs)
        .unwrap_or_else(|e| panic!("read {}: {e}", abs.display()))
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.starts_with(prefix) && n.ends_with(".sql"))
        .map(|n| format!("../../{dir}/{n}"))
        .collect();
    out.sort();
    assert!(!out.is_empty(), "no {prefix}*.sql in {dir}");
    out
}

/// DA-03's spawn block.
const DA03_SPAWNS: std::ops::RangeInclusive<i32> = 13200..=13599;

fn is_da03(r: &SpawnRecord) -> bool {
    DA03_SPAWNS.contains(&r.spawn_id)
}

/// Every DA-03 spawn row (the 13200-13599 block of world 1300).
fn da03_records() -> Vec<SpawnRecord> {
    let rows: Vec<SpawnRecord> = world_records().into_iter().filter(is_da03).collect();
    assert_eq!(rows.len(), 116, "DA-03 seeds 116 spawns");
    rows
}

/// Every world-1300 respawner position (`respawners.sql`, DA-01's 130/131).
fn world_respawners() -> Vec<(String, Vector3)> {
    seed_rows(
        &seed_files("db/resources/Worlds/Seed", "respawners"),
        "respawners",
    )
    .into_iter()
    .filter(|r| r["world_id"] == WORLD_ID)
    .map(|r| {
        let f = |k: &str| r[k].parse::<f32>().expect("numeric respawner column");
        (
            unquote(&r["name"]),
            Vector3::new(f("pos_x"), f("pos_y"), f("pos_z")),
        )
    })
    .collect()
}

/// Every `INSERT INTO <table>` row of the seed files `files`.
fn seed_rows(files: &[String], table: &str) -> Vec<HashMap<String, String>> {
    files
        .iter()
        .flat_map(|f| {
            seed(f)
                .lines()
                .filter_map(|l| parse_insert(l, table))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Every world-1300 spawn row of every `spawnlist*.sql` seed file, as the
/// loader would build its [`SpawnRecord`] (template columns joined from every
/// `entity_templates*.sql`, patrol path resolved, overrides normalised the way
/// `load_spawns_from_db` does). Other packets' rows (DA-02, DA-04, ...) come
/// in as soon as they are seeded.
fn world_records() -> Vec<SpawnRecord> {
    let templates: HashMap<String, HashMap<String, String>> = seed_rows(
        &seed_files("db/resources/Entities/Seed", "entity_templates"),
        "entity_templates",
    )
    .into_iter()
    .map(|t| (t["template_id"].clone(), t))
    .collect();
    let mut paths: HashMap<String, Vec<(i32, Vector3)>> = HashMap::new();
    for p in seed_rows(
        &seed_files("db/resources/Events/Seed", "point_set"),
        "point_set_points",
    ) {
        let f = |k: &str| p[k].parse::<f32>().expect("numeric point");
        paths.entry(p["set_id"].clone()).or_default().push((
            p["point_id"].parse().expect("point_id"),
            Vector3::new(f("x"), f("y"), f("z")),
        ));
    }
    seed_rows(
        &seed_files("db/resources/Worlds/Seed", "spawnlist"),
        "spawnlist",
    )
    .into_iter()
    .filter(|s| s["world_id"] == WORLD_ID)
    .map(|s| {
        let t = templates
            .get(&s["template_id"])
            .unwrap_or_else(|| panic!("template {} missing", s["template_id"]));
        let f = |k: &str| s[k].parse::<f32>().expect("numeric spawn column");
        let mut path = s
            .get("patrol_path_id")
            .and_then(|id| paths.get(id))
            .cloned()
            .unwrap_or_default();
        path.sort_by_key(|(id, _)| *id);
        SpawnRecord {
            spawn_id: s["spawn_id"].parse().unwrap(),
            world_name: WORLD.to_string(),
            x: f("x"),
            y: f("y"),
            z: f("z"),
            heading: f("heading"),
            tag: s
                .get("tag")
                .filter(|t| t.as_str() != "NULL")
                .map(|t| unquote(t)),
            template_id: t["template_id"].parse().unwrap(),
            template_name: unquote(&t["template_name"]),
            class: unquote(&t["class"]),
            static_mesh: None,
            body_set: unquote(&t["body_set"]),
            components: None,
            flags: 0,
            interaction_type: 0,
            event_set_id: None,
            level: opt(t.get("level")),
            alignment: None,
            faction: opt(t.get("faction")),
            name_id: opt(t.get("name_id")),
            speaker_id: None,
            static_interaction_sets: vec![],
            has_dynamic_properties: true,
            loot_table_id: None,
            is_stationary: s.get("is_stationary").is_some_and(|v| v == "true"),
            ability_ids: vec![],
            respawn_secs: opt(s.get("respawn_secs")),
            patrol_path: path.into_iter().map(|(_, p)| p).collect(),
            patrol_point_delay_secs: opt(s.get("patrol_point_delay")).unwrap_or(2.0),
            wander_radius: opt(t.get("wander_radius")).unwrap_or(0.0),
            wander_min_dwell_secs: opt(t.get("wander_min_dwell_secs")).unwrap_or(3.0),
            wander_max_dwell_secs: opt(t.get("wander_max_dwell_secs")).unwrap_or(8.0),
            follow_min_distance: 2.0,
            follow_max_distance: 5.0,
            move_speed: opt(t.get("move_speed")).unwrap_or(0.6),
            leash_distance: opt(t.get("leash_distance")),
            aggro_radius: opt(t.get("aggro_radius")),
            assist_radius: opt(t.get("assist_radius")),
            aggression_override: opt::<i32>(s.get("aggression_override"))
                .and_then(MobAggression::from_level),
            use_cover: opt(t.get("use_cover")),
            vault_scope: VaultScope::Personal,
        }
    })
    .collect()
}

fn tagged<'a>(records: &'a [SpawnRecord], prefix: &str) -> Vec<&'a SpawnRecord> {
    records
        .iter()
        .filter(|r| r.tag.as_deref().is_some_and(|t| t.starts_with(prefix)))
        .collect()
}

fn by_tag<'a>(records: &'a [SpawnRecord], tag: &str) -> &'a SpawnRecord {
    records
        .iter()
        .find(|r| r.tag.as_deref() == Some(tag))
        .unwrap_or_else(|| panic!("no spawn tagged {tag}"))
}

fn pos(r: &SpawnRecord) -> Vector3 {
    Vector3::new(r.x, r.y, r.z)
}

fn data_path(file: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/spaces")
        .join(file)
}

/// The world's navmesh, or `None` (test skips) on a checkout without it.
fn navmesh() -> Option<NavMesh> {
    let p = data_path("ihpet_crater_light.nav");
    if !p.exists() {
        eprintln!("SKIPPED: {} absent", p.display());
        return None;
    }
    Some(NavMesh::load(&p).expect("load ihpet_crater_light.nav"))
}

fn occluder() -> Option<Arc<PagedOccluder>> {
    let p = data_path("ihpet_crater_light.occ");
    if !p.exists() {
        eprintln!("SKIPPED: {} absent", p.display());
        return None;
    }
    Some(Arc::new(
        PagedOccluder::load(&p).expect("load ihpet_crater_light.occ"),
    ))
}

/// World 1300 with the real navmesh and occluder, every record spawned
/// through the production spawn path and Idle at its seed position, and the
/// player connected at `player` (so every NPC within 150 m is witnessed).
fn scene(records: &[SpawnRecord], player: Vector3) -> Option<SpaceManager> {
    let nav = navmesh()?;
    let occ = occluder()?;
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="DebugArea" Instanced="false" MinX="-2000" MaxX="2000" MinY="-2000" MaxY="2000" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="DebugArea" /></Spaces>"#,
    )
    .unwrap();
    let sid = mgr.find_or_create_space(WORLD).unwrap();
    let space = mgr.spaces.get_mut(&sid).unwrap();
    space.navmesh = Some(nav);
    space.occluder = Some(occ);
    for r in records {
        mgr.spawn_npc_from_record(eid(r.spawn_id), r).unwrap();
        crate::cell::service::npc_ai::force_ai_state(
            mgr.get_entity_mut(eid(r.spawn_id)).unwrap(),
            AiState::Idle,
        );
    }
    mgr.create_entity(PLAYER, WORLD, [player.x, player.y, player.z], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER as i32);
    mgr.connect_entity(PLAYER);
    let _ = mgr.compute_aoi_changes();
    Some(mgr)
}

/// Horizontal distance.
fn xz(a: Vector3, b: Vector3) -> f32 {
    ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
}
