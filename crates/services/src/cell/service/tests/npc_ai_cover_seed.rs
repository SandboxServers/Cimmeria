//! NA22 cover behaviour on the real Castle_CellBlock data: the world-12
//! rows of the shipped cover seed (`db/resources/AI/Seed/cover_nodes.sql`,
//! NA21's extraction) and the shipped `castle_cellblock.nav`.
//!
//! `MessHall_Guard1` (spawn 29, template 24 NID Guard) is authored 0.63 u
//! from cover marker 1200046/0, a High/Best node facing into the mess hall.
//! The designers placed the guard in cover; these tests pin that the server
//! now honours it.
//!
//! Skips on a checkout without the navmesh fixture. The seed is parsed from
//! the SQL file rather than loaded from the DB so the test runs without
//! one; the live-DB guard in `spawner::tests::live_db_use_cover` checks the
//! same rows load through the real loader.

use cimmeria_common::{EntityId, Vector3};
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::navigation::NavMesh;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use super::npc_ai_cover_behaviour::{cover_defense, seed_cover_stance_effects};
use crate::cell::cover::{
    horizontal, is_flanked, Cover, CoverHeight, CoverNode, CoverQuality, CoverSlotKey,
    COVER_ARRIVE_RADIUS,
};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::SpawnRecord;

pub(super) const NPC: u32 = 200;
pub(super) const PLAYER: u32 = 101;
pub(super) const WORLD: &str = "Castle_CellBlock";
const WORLD_ID: i32 = 12;
/// `MessHall_Guard1` (`spawnlist.sql` spawn 29).
const MESS_HALL_GUARD1: [f32; 3] = [-96.25, 34.591, -91.59];
/// The marker the guard is authored at.
const GUARD_SLOT: CoverSlotKey = CoverSlotKey {
    chunk_id: 1200046,
    node_id: 0,
};

/// Every world-12 row of the shipped cover seed.
fn seed_nodes() -> Vec<CoverNode> {
    let sql = std::fs::read_to_string("../../db/resources/AI/Seed/cover_nodes.sql")
        .expect("read cover_nodes.sql seed");
    let mut nodes = Vec::new();
    for line in sql.lines() {
        let row = line.trim();
        let Some(body) = row.strip_prefix('(') else {
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
    assert!(nodes.len() > 200, "world 12 has 236 seeded cover nodes");
    nodes
}

pub(super) struct Seeded {
    pub(super) mgr: SpaceManager,
    pub(super) nodes: Vec<CoverNode>,
    pub(super) navmesh_snap: Box<dyn Fn([f32; 3]) -> Vector3>,
}

/// A non-instanced Castle_CellBlock space with the real navmesh and the
/// real world-12 cover, stamped as world 12.
pub(super) fn seeded() -> Option<Seeded> {
    let nav = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    if !nav.exists() {
        return None;
    }
    let navmesh = NavMesh::load(nav).expect("load castle_cellblock.nav");
    let snap_mesh = NavMesh::load(nav).expect("load castle_cellblock.nav");
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-450" MaxX="450" MinY="-450" MaxY="450" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    mgr.stamp_world_rows(&std::collections::HashMap::from([(
        WORLD.to_string(),
        crate::cell::spawner::WorldRow::enforcing(WORLD_ID),
    )]));
    let space_id = mgr.find_or_create_space(WORLD).unwrap();
    mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(navmesh);
    let nodes = seed_nodes();
    mgr.cover = Cover::from_loaded(Vec::new(), nodes.clone());
    seed_cover_stance_effects(&mut mgr);
    Some(Seeded {
        mgr,
        nodes,
        navmesh_snap: Box::new(move |p| {
            snap_mesh.get_nearest_point(&Vector3::new(p[0], p[1], p[2]))
        }),
    })
}

pub(super) fn node_pos(nodes: &[CoverNode], key: CoverSlotKey) -> &CoverNode {
    nodes.iter().find(|n| n.key() == key).expect("seeded node")
}

/// The spawnlist row for `MessHall_Guard1`, with template 24's combat data
/// (faction 10, `use_cover = true`, ability set 3).
pub(super) fn mess_hall_guard() -> SpawnRecord {
    SpawnRecord {
        spawn_id: 29,
        world_name: WORLD.to_string(),
        x: MESS_HALL_GUARD1[0],
        y: MESS_HALL_GUARD1[1],
        z: MESS_HALL_GUARD1[2],
        heading: 2.094,
        tag: Some("MessHall_Guard1".to_string()),
        template_id: 24,
        template_name: "NID Guard".to_string(),
        class: "mob".to_string(),
        static_mesh: None,
        body_set: "BS_HumanMale.BS_HumanMale".to_string(),
        components: None,
        flags: 0,
        interaction_type: 0,
        event_set_id: None,
        level: Some(1),
        alignment: Some(0),
        faction: Some(10),
        name_id: None,
        speaker_id: None,
        static_interaction_sets: vec![],
        has_dynamic_properties: false,
        loot_table_id: None,
        is_stationary: false,
        ability_ids: vec![559],
        respawn_secs: None,
        patrol_path: vec![],
        patrol_point_delay_secs: 2.0,
        wander_radius: 0.0,
        wander_min_dwell_secs: 3.0,
        wander_max_dwell_secs: 8.0,
        follow_min_distance: 2.0,
        follow_max_distance: 5.0,
        move_speed: 0.6,
        leash_distance: None,
        aggro_radius: None,
        assist_radius: None,
        aggression_override: None,
        use_cover: Some(true),
    }
}

/// A point on the mesh `dist` u from the guard's marker along (or against)
/// its facing: in front of the cover, or behind it.
fn along_facing(s: &Seeded, dist: f32) -> Vector3 {
    let n = node_pos(&s.nodes, GUARD_SLOT);
    (s.navmesh_snap)([
        n.pos.x + dist * n.orient.cos(),
        n.pos.y,
        n.pos.z + dist * n.orient.sin(),
    ])
}

pub(super) fn engage(mgr: &mut SpaceManager, player_pos: Vector3) {
    mgr.create_entity(
        PLAYER,
        WORLD,
        [player_pos.x, player_pos.y, player_pos.z],
        [0.0; 3],
    )
    .unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER as i32);
    p.stats.get_mut(HEALTH).unwrap().update(0, 100, 100);
    mgr.connect_entity(PLAYER);
    let _ = mgr.compute_aoi_changes();
    let npc = mgr.get_entity_mut(NPC).unwrap();
    crate::cell::service::npc_ai::force_ai_state(npc, AiState::Fighting);
    npc.threat_list.insert(PLAYER, 10.0);
}

pub(super) fn held(mgr: &SpaceManager) -> Option<CoverSlotKey> {
    mgr.cover
        .reservations
        .lock()
        .unwrap()
        .slot_for_entity(EntityId(NPC as i32))
}

pub(super) async fn ai_tick(mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(4096);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;
}

/// Audit C4 + the hold: `MessHall_Guard1` spawns holding marker 1200046/0
/// and keeps it, standing still with Cover Stance, while the player it is
/// fighting is in range in front of the cover. Before NA22 nothing was
/// reserved at spawn (the first assertion fails); reverting the in-slot hold
/// lets the fight tick move it or drop the stance.
#[tokio::test]
async fn guard_spawned_in_cover_holds_its_slot_while_the_target_is_in_range() {
    let Some(mut s) = seeded() else {
        return;
    };
    s.mgr
        .spawn_npc_from_record(NPC, &mess_hall_guard())
        .unwrap();
    assert_eq!(held(&s.mgr), Some(GUARD_SLOT), "spawned holding its marker");
    let start = s.mgr.get_entity(NPC).unwrap().position;

    let player = along_facing(&s, 12.0);
    let slot = node_pos(&s.nodes, GUARD_SLOT).clone();
    assert!(
        !is_flanked(slot.pos, slot.orient, player),
        "fixture: player in front"
    );
    assert!(
        start.distance_to(&player) < 28.0,
        "fixture: player in range"
    );
    engage(&mut s.mgr, player);

    for tick in 0..60 {
        if tick % 10 == 0 {
            ai_tick(&mut s.mgr).await;
        }
        crate::cell::service::ticks::npc_movement_tick(&mut s.mgr);
        assert_eq!(held(&s.mgr), Some(GUARD_SLOT), "tick {tick}: slot kept");
    }
    let npc = s.mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.position, start, "held still at the marker");
    assert_eq!(npc.velocity, [0.0; 3]);
    assert!(npc.nav_path.is_empty());
    assert_eq!(
        cover_defense(&s.mgr, NPC),
        100,
        "Cover Stance while in cover"
    );
}

/// Audit C3: an NPC under fire in the mess hall, not standing in cover,
/// walks over the navmesh to a free slot that reaches its target and stops
/// there with zero velocity and Cover Stance. Before NA22 an in-range NPC
/// never took cover, so the loop never sees a held slot.
#[tokio::test]
async fn npc_under_fire_walks_to_a_slot_in_range_and_stops_there() {
    let Some(mut s) = seeded() else {
        return;
    };
    let start = (s.navmesh_snap)([-100.0, 34.591, -95.0]);
    let nearest = s
        .nodes
        .iter()
        .map(|n| horizontal(&n.pos, &start))
        .fold(f32::INFINITY, f32::min);
    assert!(
        nearest > COVER_ARRIVE_RADIUS,
        "fixture: not already in cover ({nearest})"
    );
    s.mgr
        .spawn_npc(NPC, WORLD, [start.x, start.y, start.z], [0.0; 3])
        .unwrap();
    {
        let npc = s.mgr.get_entity_mut(NPC).unwrap();
        npc.use_cover = true;
        npc.faction = 10;
        npc.move_speed = 0.6;
        npc.spawn_position = Some(start);
    }
    // Since NA23 a slot is only picked if the NPC would have a navmesh line
    // to its target from it. The mess-hall tables block most lines across
    // the room, so the player stands where the free slot 1200053/0 sees it
    // over its table (60 degrees off the slot's facing, 10 u).
    let player = (s.navmesh_snap)([-90.03, 34.6, -90.33]);
    assert!(
        start.distance_to(&player) < 28.0,
        "fixture: player in range"
    );
    engage(&mut s.mgr, player);

    let mut prev = start;
    let mut arrived = None;
    for tick in 0..400 {
        if tick % 10 == 0 {
            ai_tick(&mut s.mgr).await;
        }
        crate::cell::service::ticks::npc_movement_tick(&mut s.mgr);
        let npc = s.mgr.get_entity(NPC).unwrap();
        assert!(
            horizontal(&prev, &npc.position) <= 0.6 + 0.01,
            "tick {tick}: moved more than one step"
        );
        prev = npc.position;
        let Some(slot) = held(&s.mgr) else { continue };
        let node = node_pos(&s.nodes, slot);
        if npc.nav_path.is_empty()
            && npc.velocity == [0.0; 3]
            && horizontal(&npc.position, &node.pos) <= COVER_ARRIVE_RADIUS
            && cover_defense(&s.mgr, NPC) == 100
        {
            arrived = Some((slot, npc.position));
            break;
        }
    }
    let (slot, at) = arrived.expect("the NPC never reached and settled at a cover slot");
    assert!(horizontal(&at, &start) > 0.5, "it walked to the slot");
    let node = node_pos(&s.nodes, slot);
    assert!(
        node.pos.distance_to(&player) <= 28.0,
        "the slot reaches the target"
    );
    assert!(
        !is_flanked(node.pos, node.orient, player),
        "the slot defends"
    );

    // Holding: further ticks keep it there.
    for _ in 0..3 {
        ai_tick(&mut s.mgr).await;
        crate::cell::service::ticks::npc_movement_tick(&mut s.mgr);
    }
    assert_eq!(held(&s.mgr), Some(slot));
    assert_eq!(s.mgr.get_entity(NPC).unwrap().position, at);
}

/// The player walks round behind the guard's cover: the slot is released and
/// Cover Stance removed.
#[tokio::test]
async fn flanking_the_guard_releases_its_slot_and_stance() {
    let Some(mut s) = seeded() else {
        return;
    };
    s.mgr
        .spawn_npc_from_record(NPC, &mess_hall_guard())
        .unwrap();
    let front = along_facing(&s, 12.0);
    engage(&mut s.mgr, front);
    ai_tick(&mut s.mgr).await;
    assert_eq!(cover_defense(&s.mgr, NPC), 100);

    let behind = along_facing(&s, -8.0);
    let slot = node_pos(&s.nodes, GUARD_SLOT).clone();
    assert!(
        is_flanked(slot.pos, slot.orient, behind),
        "fixture: player behind"
    );
    s.mgr
        .update_entity_position(PLAYER, [behind.x, behind.y, behind.z], [0, 0, 0], [0.0; 3]);
    ai_tick(&mut s.mgr).await;
    assert_ne!(held(&s.mgr), Some(GUARD_SLOT), "flanked slot released");
    assert_eq!(cover_defense(&s.mgr, NPC), 0);
}
