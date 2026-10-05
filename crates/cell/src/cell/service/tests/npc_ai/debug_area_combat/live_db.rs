//! Live-DB guards for the Debug Area combat zones (DA-04): the rows the
//! production loaders build, the faction pairs against the reaction table, the
//! cover in reach of the riflemen, the respawn timers, and two fights with the
//! seeded abilities and effects (the lethal squad against a fresh character,
//! and the spectator pair's NPC-only kill paying nothing).

use std::collections::HashMap;

use cimmeria_entity::cell_entity::{AiState, MobAggression};
use cimmeria_entity::stats::{FOCUS, HEALTH};
use tokio::sync::mpsc;

use super::{DA04_SPAWNS, WORLD};
use crate::cell::combat::faction_reaction::reaction;
use crate::cell::combat::HOSTILE_FACTION;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_ability_defs, load_effect_defs, load_loot_tables, load_spawn_templates,
    load_spawns_from_db, SpawnRecord,
};
use crate::test_support::require_db_or_skip;

/// `(tag prefix, rows, zone centre (x, z), zone radius)`.
const ZONES: [(&str, usize, [f32; 2], f32); 7] = [
    ("DebugArea_Arena_Praxis", 3, [250.0, -725.0], 50.0),
    ("DebugArea_Arena_NID", 3, [250.0, -725.0], 50.0),
    ("DebugArea_Arena_Green", 2, [250.0, -725.0], 50.0),
    ("DebugArea_Arena_Yellow", 2, [250.0, -725.0], 50.0),
    ("DebugArea_Cover_Rifleman", 3, [165.0, -945.0], 30.0),
    ("DebugArea_Death_Operative", 4, [438.0, -916.0], 40.0),
    ("DebugArea_Respawn_", 2, [438.0, -916.0], 40.0),
];

async fn da04_rows(pool: &sqlx::PgPool) -> Vec<SpawnRecord> {
    load_spawns_from_db(pool)
        .await
        .expect("load_spawns_from_db must succeed")
        .into_iter()
        .filter(|r| DA04_SPAWNS.contains(&r.spawn_id))
        .collect()
}

fn with_prefix<'a>(rows: &'a [SpawnRecord], prefix: &str) -> Vec<&'a SpawnRecord> {
    rows.iter()
        .filter(|r| r.tag.as_deref().is_some_and(|t| t.starts_with(prefix)))
        .collect()
}

/// Every DA-04 spawn is in world 1300, carries a `DebugArea_` tag of one of
/// the three zones, and stands inside its zone: the arena rows on the pit
/// floor, the riflemen in the west wing (west of its doorway at x 204), the
/// Z9 rows around respawner 131.
#[tokio::test]
async fn live_db_debug_area_combat_spawns_stand_in_their_zones() {
    let pool = require_db_or_skip!();
    let rows = da04_rows(&pool).await;
    let mut matched = 0;
    for (prefix, count, [cx, cz], radius) in ZONES {
        let zone = with_prefix(&rows, prefix);
        assert_eq!(zone.len(), count, "{prefix}: {count} rows");
        for r in zone {
            assert_eq!(r.world_name, WORLD, "{:?} in world 1300", r.tag);
            let d = (r.x - cx).hypot(r.z - cz);
            assert!(d <= radius, "{:?} is {d:.1} u from its zone centre", r.tag);
            if prefix.starts_with("DebugArea_Arena_") {
                assert!((r.y + 32.4).abs() < 1.0, "{:?} on the pit floor", r.tag);
            }
            if prefix.starts_with("DebugArea_Cover_") {
                assert!(r.x < 204.0, "{:?} inside the west wing", r.tag);
            }
            matched += 1;
        }
    }
    assert_eq!(matched, rows.len(), "every DA-04 row belongs to a zone");
}

/// The arena's pairs, read through the faction reaction table: Praxis and
/// Straegis fight each other, Lucia Green and Yellow fight each other, the
/// spectator pair regards players (who react as faction 3) as non-hostile and
/// is not hostile to them either way, is not damageable (not faction 10), and
/// neither fight is hostile to the other, so nothing chains between them.
#[tokio::test]
async fn live_db_debug_area_arena_factions_follow_the_reaction_table() {
    let pool = require_db_or_skip!();
    let rows = da04_rows(&pool).await;
    let faction = |prefix: &str| -> u8 {
        let fs: Vec<i32> = with_prefix(&rows, prefix)
            .iter()
            .map(|r| r.faction.expect("arena rows carry a faction"))
            .collect();
        assert!(fs.windows(2).all(|w| w[0] == w[1]), "{prefix}: one faction");
        fs[0] as u8
    };
    let (praxis, nid) = (
        faction("DebugArea_Arena_Praxis"),
        faction("DebugArea_Arena_NID"),
    );
    let (green, yellow) = (
        faction("DebugArea_Arena_Green"),
        faction("DebugArea_Arena_Yellow"),
    );
    const PLAYER_FACTION: u8 = 3;
    assert_eq!((praxis, nid), (3, HOSTILE_FACTION));
    for (a, b) in [(praxis, nid), (green, yellow)] {
        assert!(
            reaction(a, b) == MobAggression::Hostile && reaction(b, a) == MobAggression::Hostile,
            "{a} and {b} must be mutually hostile"
        );
    }
    for f in [green, yellow] {
        assert_ne!(f, HOSTILE_FACTION, "the spectator pair is not damageable");
        assert_ne!(reaction(f, PLAYER_FACTION), MobAggression::Hostile);
        assert_ne!(reaction(PLAYER_FACTION, f), MobAggression::Hostile);
        for other in [praxis, nid] {
            assert_ne!(reaction(f, other), MobAggression::Hostile, "{f} vs {other}");
            assert_ne!(reaction(other, f), MobAggression::Hostile, "{other} vs {f}");
        }
    }
}

/// The riflemen take cover (`use_cover`, a ranged set) and the wing has cover
/// for them: at least ten world-1300 markers within 15 u of each on its floor,
/// and rifleman 3 within 1.5 u of one, so it spawns holding it.
#[tokio::test]
async fn live_db_debug_area_riflemen_have_cover_in_reach() {
    let pool = require_db_or_skip!();
    let rows = da04_rows(&pool).await;
    for r in with_prefix(&rows, "DebugArea_Cover_Rifleman") {
        assert_eq!(r.use_cover, Some(true), "{:?} takes cover", r.tag);
        assert_eq!(r.ability_ids, vec![559], "{:?} fires the SMG", r.tag);
        let (near, nearest): (i64, Option<f32>) = sqlx::query_as(
            "SELECT count(*) FILTER (WHERE d <= 15), min(d)::real FROM ( \
               SELECT sqrt((n.pos_x - $1)^2 + (n.pos_z - $3)^2) AS d \
               FROM resources.cover_nodes n \
               JOIN resources.cover_sets s ON s.chunk_id = n.chunk_id \
               WHERE s.world_id = 1300 AND abs(n.pos_y - $2) < 2) q",
        )
        .bind(r.x as f64)
        .bind(r.y as f64)
        .bind(r.z as f64)
        .fetch_one(&pool)
        .await
        .expect("cover query");
        assert!(near >= 10, "{:?}: {near} markers within 15 u", r.tag);
        if r.tag.as_deref() == Some("DebugArea_Cover_Rifleman3") {
            assert!(
                nearest.is_some_and(|d| d <= 1.5),
                "rifleman 3 stands at a marker ({nearest:?})"
            );
        }
    }
}

/// The respawn timers as the loader resolves them
/// (`COALESCE(spawnlist, template)`): the fast target's own 10 s beats its
/// template's 30 s, the slow one inherits 30 s, both are seeded NEUTRAL; the
/// arena and the riflemen come back in 30 s and the lethal squad in 60 s.
#[tokio::test]
async fn live_db_debug_area_respawn_timers_resolve() {
    let pool = require_db_or_skip!();
    let rows = da04_rows(&pool).await;
    let by_tag: HashMap<&str, &SpawnRecord> = rows
        .iter()
        .filter_map(|r| Some((r.tag.as_deref()?, r)))
        .collect();
    let fast = by_tag["DebugArea_Respawn_Fast"];
    let slow = by_tag["DebugArea_Respawn_Slow"];
    assert_eq!(
        fast.template_id, slow.template_id,
        "one template, two timers"
    );
    assert_eq!((fast.respawn_secs, slow.respawn_secs), (Some(10), Some(30)));
    for r in [fast, slow] {
        assert_eq!(r.aggression_override, Some(MobAggression::Neutral));
        assert_eq!(r.faction, Some(HOSTILE_FACTION as i32), "damageable");
    }
    for (prefix, secs) in [
        ("DebugArea_Arena_", 30),
        ("DebugArea_Cover_", 30),
        ("DebugArea_Death_", 60),
    ] {
        for r in with_prefix(&rows, prefix) {
            assert_eq!(r.respawn_secs, Some(secs), "{:?}", r.tag);
        }
    }
}

/// A meshless fight space with the seeded abilities, effects and loot, and a
/// player watching from `watcher`.
async fn fight_space(pool: &sqlx::PgPool, watcher: [f32; 3]) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    crate::test_support::install_effect_scripts(&mut mgr);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="DebugArea" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="DebugArea" /></Spaces>"#,
    )
    .unwrap();
    mgr.ability_defs = load_ability_defs(pool).await.expect("ability defs");
    mgr.effect_defs = load_effect_defs(pool).await.expect("effect defs");
    mgr.loot_tables = load_loot_tables(pool).await.expect("loot tables");
    mgr.create_entity(PLAYER, WORLD, watcher, [0.0; 3]).unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER as i32);
    mgr
}

const PLAYER: u32 = 1;

/// Spawn template `template_id` as `id` at `(x, 0, z)`, Idle.
fn spawn(
    mgr: &mut SpaceManager,
    templates: &HashMap<i32, SpawnRecord>,
    id: u32,
    template_id: i32,
    x: f32,
    z: f32,
) {
    let mut record = templates
        .get(&template_id)
        .unwrap_or_else(|| panic!("seeded template {template_id}"))
        .clone();
    record.world_name = WORLD.to_string();
    (record.x, record.y, record.z) = (x, 0.0, z);
    record.tag = Some(format!("CIMMERIA_TEST_DA04_{id}"));
    mgr.spawn_npc_from_record(id, &record).expect("spawn");
    crate::cell::service::npc_ai::force_ai_state(mgr.get_entity_mut(id).unwrap(), AiState::Idle);
}

/// One fight pass: an AI tick, any warmup fired at once, then cooldowns
/// cleared. With 559's 1.5 s cooldown under the 2 s AI cadence, a pass is
/// one AI tick of real time.
async fn pass(
    mgr: &mut SpaceManager,
    npcs: &[u32],
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
) {
    let events =
        crate::cell::content::EngineEvents(&cimmeria_content_engine::chain::ChainEngine::new());
    crate::cell::service::npc_ai::npc_ai_tick(tx, mgr, &events).await;
    for &id in npcs {
        if let Some(pc) = mgr.get_entity_mut(id).and_then(|e| e.pending_cast.as_mut()) {
            pc.fire_at = std::time::Instant::now();
        }
    }
    crate::cell::abilities::warmup_tick(tx, mgr, &events).await;
    for &id in npcs {
        if let Some(e) = mgr.get_entity_mut(id) {
            e.abilities = cimmeria_entity::abilities::AbilityManager::with_abilities(
                &e.abilities.known_ability_ids(),
            );
        }
    }
    while rx.try_recv().is_ok() {}
}

/// Z9's promise: the four operatives (template 1376, the seeded 559 SMG) kill
/// a fresh character, with a new Commando's 760 Health and 1,570 Focus
/// (`archetypes.sql`), within seven AI ticks (14 s; it takes five when
/// measured). Fails if the squad is thinned to two, its ability set changed to
/// a weaker one, or its damage stops.
#[tokio::test]
async fn live_db_debug_area_lethal_squad_kills_a_fresh_character_quickly() {
    let pool = require_db_or_skip!();
    let templates = load_spawn_templates(&pool).await.expect("templates");
    let squad = da04_rows(&pool).await;
    let squad = with_prefix(&squad, "DebugArea_Death_Operative");
    let template = squad[0].template_id;
    let mut mgr = fight_space(&pool, [0.0, 0.0, 0.0]).await;
    {
        let p = mgr.get_entity_mut(PLAYER).unwrap();
        p.stats.get_mut(HEALTH).unwrap().update(0, 760, 760);
        p.stats.get_mut(FOCUS).unwrap().update(0, 1570, 1570);
    }
    mgr.connect_entity(PLAYER);
    let ids: Vec<u32> = (0..squad.len() as u32).map(|i| 300_000 + i).collect();
    for (i, &id) in ids.iter().enumerate() {
        spawn(
            &mut mgr,
            &templates,
            id,
            template,
            8.0,
            i as f32 * 3.0 - 4.5,
        );
    }
    let _ = mgr.compute_aoi_changes();
    let (tx, mut rx) = mpsc::channel(65_536);
    let mut died_after = None;
    for n in 1..=7 {
        pass(&mut mgr, &ids, &tx, &mut rx).await;
        let p = mgr.get_entity(PLAYER).unwrap();
        if crate::cell::combat::is_dead_state(p.state_field)
            || p.stats.get(HEALTH).unwrap().cur <= 0
        {
            died_after = Some(n);
            break;
        }
    }
    let p = mgr.get_entity(PLAYER).unwrap();
    assert!(
        died_after.is_some(),
        "a fresh character must die to the squad within 7 AI ticks; left with {} Health, {} Focus",
        p.stats.get(HEALTH).unwrap().cur,
        p.stats.get(FOCUS).unwrap().cur
    );
    eprintln!("lethal squad: fresh character dead after {died_after:?} AI ticks");
}

/// D-DA8's spectator fight: a Green Sniper and a Yellow Faction (templates of
/// the arena rows, both on loot table 2) fight to a death with the seeded
/// abilities; the corpse rolls no loot and shows no loot cursor (#1009).
/// Loot row 13 of table 2 drops at probability 1.0, so with the NPC-only-kill
/// gate reverted the corpse always holds item 2893 and this fails. (No XP is
/// asserted: `grant_kill_xp` never credits a plain-NPC killer, so that
/// assertion could not fail.)
#[tokio::test]
async fn live_db_debug_area_spectator_kill_pays_nothing() {
    let pool = require_db_or_skip!();
    let templates = load_spawn_templates(&pool).await.expect("templates");
    let rows = da04_rows(&pool).await;
    let green = with_prefix(&rows, "DebugArea_Arena_Green")[0].template_id;
    let yellow = with_prefix(&rows, "DebugArea_Arena_Yellow")[0].template_id;
    let mut mgr = fight_space(&pool, [6.0, 0.0, -40.0]).await;
    mgr.connect_entity(PLAYER);
    let (g, y) = (300_101, 300_102);
    spawn(&mut mgr, &templates, g, green, 0.0, 0.0);
    spawn(&mut mgr, &templates, y, yellow, 12.0, 0.0);
    for id in [g, y] {
        assert!(
            mgr.get_entity(id).unwrap().loot_table_id.is_some(),
            "seed: both would roll loot on a player kill"
        );
    }
    let _ = mgr.compute_aoi_changes();
    let (tx, mut rx) = mpsc::channel(65_536);
    let mut dead = None;
    for _ in 0..400 {
        pass(&mut mgr, &[g, y], &tx, &mut rx).await;
        dead = [g, y]
            .into_iter()
            .find(|&id| mgr.get_entity(id).unwrap().ai_state() == AiState::Dead);
        if dead.is_some() {
            break;
        }
    }
    let dead = dead.expect("one side must die within 400 passes");
    let corpse = mgr.get_entity(dead).unwrap();
    assert!(corpse.loot.is_empty(), "an NPC-only kill rolls no loot");
    assert_eq!(
        corpse.interaction_type_flags & crate::cell::abilities::INT_NORMAL_LOOT,
        0,
        "and shows no loot cursor"
    );
}

/// Every DA-04 template's aggro radius, through the loader. The seed-wide pin
/// (`live_db_aggression::seed_overrides_only_the_chain_armed_spawns`) skips
/// world 1300 because every Debug Area station tunes it on purpose; this is
/// the pin for DA-04's own, so a changed radius is a deliberate edit here.
#[tokio::test]
async fn live_db_debug_area_combat_aggro_radii() {
    let pool = require_db_or_skip!();
    let rows = da04_rows(&pool).await;
    let mut radii: Vec<(i32, Option<u32>)> = rows
        .iter()
        .map(|r| (r.template_id, r.aggro_radius.map(|v| v as u32)))
        .collect();
    radii.sort();
    radii.dedup();
    assert_eq!(
        radii,
        vec![
            (1370, Some(30)),
            (1371, Some(30)),
            (1372, Some(30)),
            (1373, Some(26)),
            (1374, Some(26)),
            (1375, Some(25)),
            (1376, Some(10)),
            (1377, None),
        ]
    );
}
