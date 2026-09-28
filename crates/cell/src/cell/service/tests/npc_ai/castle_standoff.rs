//! The Castle standoff (NPC-vs-NPC, #1009, D-CP11) on the real `castle.nav`
//! and `castle.occ`: every `Castle_Standoff_*` row engages a NID guard from
//! where the seed puts it.
//!
//! The scene is built from the seed files themselves (every World 8 `mob` row
//! of `spawnlist.sql`, with faction and aggro radius from
//! `entity_templates.sql`), so moving a row, changing a template's faction or
//! radius, or rebuilding the mesh or occluder under the standoff fails here.
//! The friendly's own scan runs through the production gates (grid query,
//! hostility, vertical band, radius, occluder line of sight), with a player
//! witness 120 u away, outside every radius.
//!
//! Skips on a checkout without `data/spaces/castle.nav` / `castle.occ`.

use std::collections::HashMap;
use std::sync::Arc;

use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::navigation::NavMesh;
use cimmeria_entity::stats::HEALTH;
use cimmeria_occluder::PagedOccluder;
use tokio::sync::mpsc;

use crate::cell::space_manager::SpaceManager;

const PLAYER: u32 = 1;

/// `(cols, values)` of one single-row `INSERT INTO <table> (...) VALUES (...);`
/// line, with quotes and `{...}` arrays kept whole.
fn parse_insert(line: &str, table: &str) -> Option<HashMap<String, String>> {
    let rest = line.strip_prefix(&format!("INSERT INTO {table} ("))?;
    let (cols, rest) = rest.split_once(") VALUES (")?;
    let body = rest.trim_end().strip_suffix(");")?;
    let mut vals = Vec::new();
    let (mut cur, mut quoted, mut depth) = (String::new(), false, 0i32);
    for ch in body.chars() {
        match ch {
            '\'' => quoted = !quoted,
            '{' if !quoted => depth += 1,
            '}' if !quoted => depth -= 1,
            ',' if !quoted && depth == 0 => {
                vals.push(std::mem::take(&mut cur).trim().to_string());
                continue;
            }
            _ => {}
        }
        cur.push(ch);
    }
    vals.push(cur.trim().to_string());
    let cols: Vec<String> = cols.split(',').map(|c| c.trim().to_string()).collect();
    (cols.len() == vals.len()).then(|| cols.into_iter().zip(vals).collect())
}

fn seed(rel: &str) -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel))
        .unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

/// One World 8 mob row: `(spawn_id, tag, [x, y, z], faction, aggro_radius)`.
type Row = (u32, String, [f32; 3], u8, Option<f32>);

fn castle_mobs() -> Vec<Row> {
    let templates: HashMap<String, HashMap<String, String>> =
        seed("../../db/resources/Entities/Seed/entity_templates.sql")
            .lines()
            .filter_map(|l| parse_insert(l, "entity_templates"))
            .map(|t| (t["template_id"].clone(), t))
            .collect();
    seed("../../db/resources/Worlds/Seed/spawnlist.sql")
        .lines()
        .filter_map(|l| parse_insert(l, "spawnlist"))
        .filter(|s| s["world_id"] == "8")
        .filter_map(|s| {
            let t = templates.get(&s["template_id"])?;
            if t["class"] != "'mob'" {
                return None;
            }
            let f = |k: &str| s[k].parse::<f32>().expect("numeric coordinate");
            Some((
                s["spawn_id"].parse().unwrap(),
                s["tag"].trim_matches('\'').to_string(),
                [f("x"), f("y"), f("z")],
                t["faction"].parse().unwrap_or(0),
                t.get("aggro_radius").and_then(|r| r.parse().ok()),
            ))
        })
        .collect()
}

/// A Castle space with the real navmesh and occluder and every World 8 mob
/// Idle at its seed position, entity id = 100_000 + spawn id.
fn scene(rows: &[Row]) -> Option<SpaceManager> {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/spaces");
    let (nav, occ) = (base.join("castle.nav"), base.join("castle.occ"));
    if !nav.exists() || !occ.exists() {
        eprintln!("SKIPPED: castle.nav / castle.occ absent");
        return None;
    }
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-2000" MaxX="2000" MinY="-2000" MaxY="2000" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    let sid = mgr.find_or_create_space("Castle").unwrap();
    let space = mgr.spaces.get_mut(&sid).unwrap();
    space.navmesh = Some(NavMesh::load(&nav).expect("load castle.nav"));
    space.occluder = Some(Arc::new(
        PagedOccluder::load(&occ).expect("load castle.occ"),
    ));
    for (spawn_id, tag, pos, faction, radius) in rows {
        let id = 100_000 + spawn_id;
        mgr.spawn_npc(id, "Castle", *pos, [0.0; 3]).unwrap();
        let e = mgr.get_entity_mut(id).unwrap();
        e.faction = *faction;
        e.tag = Some(tag.clone());
        e.aggro.radius_override = *radius;
        crate::cell::service::npc_ai::force_ai_state(e, AiState::Idle);
        let h = e.stats.get_mut(HEALTH).unwrap();
        h.update(0, 400, 400);
        h.clear_dirty();
    }
    Some(mgr)
}

/// **Content guard.** Each of the eight standoff rows, scanning alone with a
/// witness nearby, engages a faction-10 NPC. Before #1009 (and with the rows
/// back at the barricade or on faction 1) none of them engages anything.
#[tokio::test]
async fn every_castle_standoff_row_engages_a_nid_guard() {
    let rows = castle_mobs();
    let standoff: Vec<&Row> = rows
        .iter()
        .filter(|r| r.1.starts_with("Castle_Standoff_"))
        .collect();
    assert_eq!(standoff.len(), 8, "the eight Castle_Standoff_ rows");
    let hostile: std::collections::HashSet<u32> = rows
        .iter()
        .filter(|r| r.3 == crate::cell::combat::HOSTILE_FACTION)
        .map(|r| 100_000 + r.0)
        .collect();

    for (spawn_id, tag, pos, ..) in standoff {
        let Some(mut mgr) = scene(&rows) else {
            return;
        };
        let npc = 100_000 + spawn_id;
        mgr.create_entity(PLAYER, "Castle", [pos[0] - 120.0, pos[1], pos[2]], [0.0; 3])
            .unwrap();
        mgr.get_entity_mut(PLAYER).unwrap().is_player = true;
        mgr.connect_entity(PLAYER);
        let _ = mgr.compute_aoi_changes();
        assert!(
            !mgr.get_witnesses_of(npc).is_empty(),
            "fixture: {tag} is witnessed"
        );
        let (tx, _rx) = mpsc::channel(256);
        let engaged =
            crate::cell::service::npc_ai::npc_idle_aggro_scan_for_test(npc, &tx, &mut mgr).await;
        let targets: Vec<u32> = mgr
            .get_entity(npc)
            .unwrap()
            .threat_list
            .keys()
            .copied()
            .collect();
        assert!(
            engaged && targets.iter().all(|t| hostile.contains(t)) && !targets.is_empty(),
            "{tag} at {pos:?} must engage a NID guard; engaged={engaged}, targets={targets:?}"
        );
    }
}
