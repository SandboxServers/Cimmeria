//! The Debug Area's combat zones (world 1300, DA-04, `docs/content/debug-area.md`)
//! on the real `ihpet_crater_light.nav` / `.occ` and the world-1300 rows of the
//! shipped cover seed.
//!
//! The scene is built from the seed files themselves: every world-1300 `mob`
//! row of every `spawnlist*.sql` seed (DA-02..DA-04 each keep their own file),
//! with its template from every `entity_templates*.sql` seed and its abilities
//! from `ability_set_abilities.sql`, spawned through the production
//! `spawn_npc_from_record` (navmesh grounding, faction, aggro radius, cover
//! hold). Moving a row, changing a template, or rebuilding the mesh, occluder
//! or cover under a zone fails here without a database.
//!
//! - [`arena`]: Z6, the NPC-vs-NPC arena in the pit (D-DA8).
//! - [`cover`]: Z8, the cover course in the south compound's west wing.
//! - [`death_respawn`]: Z9, the lethal squad and the respawn-timer targets
//!   around respawner 131, and the navmesh placement of every DA-04 spawn.
//! - [`live_db`]: the same content through the database loaders, with the
//!   seeded abilities and effects.
//!
//! Skips (with a SKIPPED line) on a checkout without the navmesh or occluder.

mod arena;
mod cover;
mod death_respawn;
mod live_db;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::{AiState, MobAggression, VaultScope};
use cimmeria_entity::navigation::NavMesh;
use cimmeria_occluder::PagedOccluder;
use tokio::sync::mpsc;

use super::castle_standoff::parse_insert;
use crate::cell::cover::{Cover, CoverHeight, CoverNode, CoverQuality, CoverSlotKey};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{SpawnRecord, WorldRow};

pub(super) const WORLD: &str = "DebugArea";
pub(super) const WORLD_ID: i32 = 1300;
/// Seed entity ids: `NPC_BASE + spawn_id`.
pub(super) const NPC_BASE: u32 = 100_000;
pub(super) const PLAYER: u32 = 1;

/// DA-04's spawn block (docs/analysis/debug-area/README.md, Id blocks).
pub(super) const DA04_SPAWNS: std::ops::RangeInclusive<i32> = 13600..=13799;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// Every `INSERT INTO <table>` row of every `<prefix>*.sql` file in `dir`.
fn seed_rows(dir: &str, prefix: &str, table: &str) -> Vec<HashMap<String, String>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(repo(dir))
        .unwrap_or_else(|e| panic!("read {dir}: {e}"))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(prefix) && n.ends_with(".sql"))
        })
        .collect();
    files.sort();
    files
        .iter()
        .flat_map(|f| {
            std::fs::read_to_string(f)
                .unwrap_or_else(|e| panic!("read {}: {e}", f.display()))
                .lines()
                .filter_map(|l| parse_insert(l, table))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn opt<T: std::str::FromStr>(row: &HashMap<String, String>, key: &str) -> Option<T> {
    row.get(key)
        .map(|v| v.trim_matches('\''))
        .filter(|v| *v != "NULL")
        .and_then(|v| v.parse().ok())
}

fn text(row: &HashMap<String, String>, key: &str) -> Option<String> {
    opt::<String>(row, key)
}

/// Every world-1300 `mob` spawn of the seed, parsed once per test binary.
pub(super) fn world_records() -> Vec<SpawnRecord> {
    static RECORDS: OnceLock<Vec<SpawnRecord>> = OnceLock::new();
    RECORDS.get_or_init(parse_world_records).clone()
}

/// The ordered points of every point set in the `Events/Seed` files.
fn point_sets() -> HashMap<i32, Vec<Vector3>> {
    let mut rows: Vec<(i32, i32, Vector3)> =
        seed_rows("db/resources/Events/Seed", "point_set", "point_set_points")
            .into_iter()
            .filter_map(|r| {
                Some((
                    opt(&r, "set_id")?,
                    opt(&r, "point_id")?,
                    Vector3::new(opt(&r, "x")?, opt(&r, "y")?, opt(&r, "z")?),
                ))
            })
            .collect();
    rows.sort_by_key(|r| (r.0, r.1));
    let mut sets: HashMap<i32, Vec<Vector3>> = HashMap::new();
    for (set, _, p) in rows {
        sets.entry(set).or_default().push(p);
    }
    sets
}

/// The seed rows as the loader would build them: `COALESCE(spawn, template)`
/// for respawn, loot and the patrol path, template values for the rest.
fn parse_world_records() -> Vec<SpawnRecord> {
    let paths = point_sets();
    let templates: HashMap<i32, HashMap<String, String>> = seed_rows(
        "db/resources/Entities/Seed",
        "entity_templates",
        "entity_templates",
    )
    .into_iter()
    .filter_map(|t| Some((opt(&t, "template_id")?, t)))
    .collect();
    let mut sets: HashMap<i32, Vec<i32>> = HashMap::new();
    for r in seed_rows(
        "db/resources/Abilities/Seed",
        "ability_set_abilities",
        "ability_set_abilities",
    ) {
        if let (Some(set), Some(id)) = (opt(&r, "ability_set_id"), opt(&r, "ability_id")) {
            sets.entry(set).or_default().push(id);
        }
    }
    seed_rows("db/resources/Worlds/Seed", "spawnlist", "spawnlist")
        .into_iter()
        .filter(|s| opt::<i32>(s, "world_id") == Some(WORLD_ID))
        .filter_map(|s| {
            let template_id: i32 = opt(&s, "template_id")?;
            let t = templates.get(&template_id)?;
            if text(t, "class").as_deref() != Some("mob") {
                return None;
            }
            let mut ability_ids = opt::<i32>(t, "ability_set_id")
                .and_then(|set| sets.get(&set).cloned())
                .unwrap_or_default();
            ability_ids.sort_unstable();
            Some(SpawnRecord {
                spawn_id: opt(&s, "spawn_id")?,
                world_name: WORLD.to_string(),
                x: opt(&s, "x")?,
                y: opt(&s, "y")?,
                z: opt(&s, "z")?,
                heading: opt(&s, "heading")?,
                tag: text(&s, "tag"),
                template_id,
                template_name: text(t, "template_name").unwrap_or_default(),
                class: "mob".to_string(),
                static_mesh: None,
                body_set: text(t, "body_set").unwrap_or_default(),
                components: None,
                flags: 0,
                interaction_type: 0,
                event_set_id: None,
                level: opt(t, "level"),
                alignment: opt(t, "alignment"),
                faction: opt(t, "faction"),
                name_id: opt(t, "name_id"),
                speaker_id: None,
                static_interaction_sets: vec![],
                has_dynamic_properties: true,
                loot_table_id: opt(&s, "loot_table_id").or_else(|| opt(t, "loot_table_id")),
                is_stationary: text(&s, "is_stationary").as_deref() == Some("true"),
                ability_ids,
                respawn_secs: opt(&s, "respawn_secs").or_else(|| opt(t, "respawn_secs")),
                patrol_path: opt::<i32>(&s, "patrol_path_id")
                    .or_else(|| opt(t, "patrol_path_id"))
                    .and_then(|set| paths.get(&set).cloned())
                    .unwrap_or_default(),
                patrol_point_delay_secs: 2.0,
                wander_radius: opt(t, "wander_radius").unwrap_or(0.0),
                wander_min_dwell_secs: 3.0,
                wander_max_dwell_secs: 8.0,
                follow_min_distance: 2.0,
                follow_max_distance: 5.0,
                move_speed: 0.6,
                leash_distance: opt(t, "leash_distance"),
                aggro_radius: opt(t, "aggro_radius"),
                assist_radius: opt(t, "assist_radius"),
                aggression_override: opt(&s, "aggression_override")
                    .and_then(MobAggression::from_level),
                use_cover: text(t, "use_cover").map(|v| v == "true"),
                vault_scope: VaultScope::Personal,
            })
        })
        .collect()
}

/// The DA-04 rows whose tag starts with `prefix`.
pub(super) fn da04<'a>(records: &'a [SpawnRecord], prefix: &str) -> Vec<&'a SpawnRecord> {
    records
        .iter()
        .filter(|r| DA04_SPAWNS.contains(&r.spawn_id))
        .filter(|r| r.tag.as_deref().is_some_and(|t| t.starts_with(prefix)))
        .collect()
}

pub(super) fn npc_id(r: &SpawnRecord) -> u32 {
    NPC_BASE + r.spawn_id as u32
}

/// Every world-1300 row of the shipped cover seed (DA-01's extraction),
/// parsed once per test binary.
pub(super) fn cover_nodes() -> Vec<CoverNode> {
    static NODES: OnceLock<Vec<CoverNode>> = OnceLock::new();
    NODES.get_or_init(parse_cover_nodes).clone()
}

fn parse_cover_nodes() -> Vec<CoverNode> {
    let sql = std::fs::read_to_string(repo("db/resources/AI/Seed/cover_nodes.sql"))
        .expect("read cover_nodes.sql seed");
    let mut nodes = Vec::new();
    for line in sql.lines() {
        let Some(body) = line.trim().strip_prefix('(') else {
            continue;
        };
        let f: Vec<&str> = body.split(", ").collect();
        if f.len() < 10 {
            continue;
        }
        let Ok(chunk_id) = f[0].parse::<i32>() else {
            continue;
        };
        if chunk_id / 100_000 != WORLD_ID {
            continue;
        }
        let num = |i: usize| f[i].parse::<f32>().expect("numeric cover column");
        let unquote = |s: &str| s.trim_matches('\'').to_string();
        nodes.push(CoverNode {
            chunk_id,
            node_id: f[1].parse().unwrap(),
            world_id: WORLD_ID,
            pos: Vector3::new(num(2), num(3), num(4)),
            orient: num(5),
            height: CoverHeight::from_sql_name(&unquote(f[6])).unwrap(),
            quality: CoverQuality::from_sql_name(&unquote(f[7])).unwrap(),
            width: num(8),
            tail: [0; 4],
        });
    }
    assert!(
        nodes.len() > 6000,
        "world 1300 carries Ihpet_Crater_Light's 6,324 cover nodes (DA-01); found {}",
        nodes.len()
    );
    nodes
}

pub(super) fn navmesh() -> Option<NavMesh> {
    let nav = repo("data/spaces/ihpet_crater_light.nav");
    nav.exists()
        .then(|| NavMesh::load(&nav).expect("load ihpet_crater_light.nav"))
}

/// The DebugArea space on the real mesh, occluder and cover, stamped as world
/// 1300, with every record in `records` spawned Idle at `NPC_BASE + spawn_id`.
pub(super) fn scene(records: &[SpawnRecord]) -> Option<SpaceManager> {
    let occ = repo("data/spaces/ihpet_crater_light.occ");
    let Some(navmesh) = navmesh() else {
        eprintln!("SKIPPED: ihpet_crater_light.nav absent");
        return None;
    };
    if !occ.exists() {
        eprintln!("SKIPPED: ihpet_crater_light.occ absent");
        return None;
    }
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="DebugArea" Instanced="false" MinX="-2000" MaxX="2000" MinY="-2000" MaxY="2000" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="DebugArea" /></Spaces>"#,
    )
    .unwrap();
    mgr.stamp_world_rows(&HashMap::from([(
        WORLD.to_string(),
        WorldRow::enforcing(WORLD_ID),
    )]));
    let sid = mgr.find_or_create_space(WORLD).unwrap();
    let space = mgr.spaces.get_mut(&sid).unwrap();
    space.navmesh = Some(navmesh);
    static OCCLUDER: OnceLock<Arc<PagedOccluder>> = OnceLock::new();
    space.occluder = Some(
        OCCLUDER
            .get_or_init(|| {
                Arc::new(PagedOccluder::load(&occ).expect("load ihpet_crater_light.occ"))
            })
            .clone(),
    );
    // Cover before the spawns: `spawn_npc_from_record` reserves the marker an
    // NPC is authored at (NA22).
    mgr.cover = Cover::from_loaded(Vec::new(), cover_nodes());
    super::super::npc_ai_cover_behaviour::seed_cover_stance_effects(&mut mgr);
    for r in records {
        mgr.spawn_npc_from_record(npc_id(r), r)
            .unwrap_or_else(|e| panic!("spawn {:?}: {e}", r.tag));
        crate::cell::service::npc_ai::force_ai_state(
            mgr.get_entity_mut(npc_id(r)).unwrap(),
            AiState::Idle,
        );
    }
    Some(mgr)
}

/// A connected player at `pos`, then the AoI pass that makes it a witness.
pub(super) fn add_player(mgr: &mut SpaceManager, id: u32, pos: [f32; 3]) {
    mgr.create_entity(id, WORLD, pos, [0.0; 3]).unwrap();
    let p = mgr.get_entity_mut(id).unwrap();
    p.is_player = true;
    p.player_id = Some(id as i32);
    mgr.connect_entity(id);
    let _ = mgr.compute_aoi_changes();
}

/// One Idle scan of `npc`: whether it engaged, and its threat-list keys.
pub(super) async fn scan(mgr: &mut SpaceManager, npc: u32) -> (bool, Vec<u32>) {
    let (tx, _rx) = mpsc::channel(256);
    let engaged = crate::cell::service::npc_ai::npc_idle_aggro_scan_for_test(npc, &tx, mgr).await;
    let targets = mgr
        .get_entity(npc)
        .unwrap()
        .threat_list
        .keys()
        .copied()
        .collect();
    (engaged, targets)
}

/// The cover slot `npc` holds, if any.
pub(super) fn held(mgr: &SpaceManager, npc: u32) -> Option<CoverSlotKey> {
    mgr.cover
        .reservations
        .lock()
        .unwrap()
        .slot_for_entity(cimmeria_common::EntityId(npc as i32))
}

/// Put every NPC in `ids` back to Idle with an empty threat list, so one scene
/// can serve several independent scans.
pub(super) fn reset_idle(mgr: &mut SpaceManager, ids: impl IntoIterator<Item = u32>) {
    for id in ids {
        if let Some(e) = mgr.get_entity_mut(id) {
            e.threat_list.clear();
            crate::cell::service::npc_ai::force_ai_state(e, AiState::Idle);
        }
    }
}
