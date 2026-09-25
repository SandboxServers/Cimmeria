//! NA32 ranged step-back (D-NA15): a ranged NPC attacking in place steps
//! back from a target inside its 2 u comfort range; a melee, stationary or
//! in-cover one does not; the 3 s cooldown gates the next step. The last
//! two tests run on the real `castle_cellblock.nav`.
//!
//! Revert-proof: before NA32 only a hard `min_range` stepped back, and every
//! seeded NPC ability has `min_range = 0`, so the headline test's NPC
//! attacked in place with its target in its face.

use std::time::{Duration, Instant};

use cimmeria_common::{EntityId, Vector3};
use cimmeria_entity::abilities::AbilityDef;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::navigation::NavMesh;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use super::{make_ai_fixture, seed_default_ability, seed_target_with_threat};
use crate::cell::combat::NPC_DEFAULT_ABILITY;
use crate::cell::cover::CoverSlotKey;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;

const NPC: u32 = 200;
const PLAYER: u32 = 100;
const MELEE: i32 = 710;

async fn ai_tick(mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(4096);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;
}

fn horizontal(a: &Vector3, b: &Vector3) -> f32 {
    ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// A Fighting NPC at (10,0,0) with the ranged default ability (min 0,
/// max 30) and a player `gap` u further along +X.
fn ranged_npc(gap: f32) -> SpaceManager {
    let mut mgr = make_ai_fixture([10.0, 0.0, 0.0], [10.0, 0.0, 0.0]);
    seed_default_ability(&mut mgr, 0, 30);
    mgr.get_entity_mut(NPC)
        .unwrap()
        .abilities
        .add_ability(NPC_DEFAULT_ABILITY);
    seed_target_with_threat(&mut mgr, NPC, PLAYER, [10.0 + gap, 0.0, 0.0]);
    mgr
}

fn waypoint(mgr: &SpaceManager) -> Option<Vector3> {
    mgr.get_entity(NPC).unwrap().nav_path.back().copied()
}

/// The headline case: a player 1 u from a ranged guard. It steps straight
/// back to the 5 u retreat distance (2 u comfort + 3 u hysteresis).
#[tokio::test]
async fn a_ranged_npc_steps_back_from_a_target_in_its_face() {
    let mut mgr = ranged_npc(1.0);
    let logs = LogCapture::install();
    ai_tick(&mut mgr).await;
    let w = waypoint(&mgr).expect("the NPC steps back");
    let player = Vector3::new(11.0, 0.0, 0.0);
    assert!(
        (horizontal(&w, &player) - 5.0).abs() < 1e-3,
        "retreat to comfort + margin: {w:?}"
    );
    assert!(w.x < 10.0, "away from the player: {w:?}");
    assert!(mgr.get_entity(NPC).unwrap().leash.step_back_at.is_some());
    assert!(logs
        .all()
        .iter()
        .any(|c| c.has_field("decision_outcome", "step_back")));
}

/// Outside the comfort range it attacks in place, as before.
#[tokio::test]
async fn a_ranged_npc_fires_from_outside_the_comfort_range() {
    let mut mgr = ranged_npc(2.5);
    ai_tick(&mut mgr).await;
    assert!(waypoint(&mgr).is_none());
    assert!(mgr.get_entity(NPC).unwrap().leash.step_back_at.is_none());
}

/// A melee NPC never steps back: at 1 u it is in swing reach.
#[tokio::test]
async fn a_melee_npc_never_steps_back() {
    let mut mgr = ranged_npc(1.0);
    mgr.ability_defs.insert(
        MELEE,
        AbilityDef {
            ability_id: MELEE,
            name: "Staff Melee AA".to_string(),
            cooldown: 1.0,
            is_ranged: false,
            max_range: 3,
            ..mgr.ability_defs[&NPC_DEFAULT_ABILITY].clone()
        },
    );
    let npc = mgr.get_entity_mut(NPC).unwrap();
    npc.abilities = Default::default();
    npc.abilities.add_ability(MELEE);
    ai_tick(&mut mgr).await;
    assert!(waypoint(&mgr).is_none(), "melee holds its ground");
    assert!(mgr.get_entity(NPC).unwrap().leash.step_back_at.is_none());
}

/// A stationary NPC is pinned.
#[tokio::test]
async fn a_stationary_npc_never_steps_back() {
    let mut mgr = ranged_npc(1.0);
    mgr.get_entity_mut(NPC).unwrap().is_stationary = true;
    ai_tick(&mut mgr).await;
    assert!(waypoint(&mgr).is_none());
    assert!(mgr.get_entity(NPC).unwrap().leash.step_back_at.is_none());
}

/// The 3 s cooldown: a player who follows the NPC gets shot at, not
/// kited. Once the cooldown runs out the NPC steps again.
#[tokio::test]
async fn the_cooldown_gates_the_next_step() {
    let mut mgr = ranged_npc(1.0);
    ai_tick(&mut mgr).await;
    assert!(waypoint(&mgr).is_some(), "first step");

    // It arrived; the player closed in again.
    let npc = mgr.get_entity_mut(NPC).unwrap();
    npc.nav_path.clear();
    mgr.update_entity_position(PLAYER, [11.0, 0.0, 0.0], [0, 0, 0], [0.0; 3]);
    ai_tick(&mut mgr).await;
    assert!(waypoint(&mgr).is_none(), "cooling: it fires instead");

    mgr.get_entity_mut(NPC).unwrap().leash.step_back_at =
        Instant::now().checked_sub(Duration::from_secs(3));
    ai_tick(&mut mgr).await;
    assert!(waypoint(&mgr).is_some(), "the cooldown is over");
}

/// A step-back still being walked is left alone during the cooldown: the
/// attack arm would clear the route.
#[tokio::test]
async fn a_step_back_in_flight_is_not_cut_short() {
    let mut mgr = ranged_npc(1.0);
    ai_tick(&mut mgr).await;
    let first = waypoint(&mgr).expect("first step");
    ai_tick(&mut mgr).await;
    assert_eq!(waypoint(&mgr), Some(first), "still walking the step");
}

/// A sniper (`min_range` 5) inside its dead zone during the cooldown cannot
/// fire, so it holds rather than attacking.
#[tokio::test]
async fn a_dead_zone_during_the_cooldown_holds_fire() {
    let mut mgr = make_ai_fixture([10.0, 0.0, 0.0], [10.0, 0.0, 0.0]);
    seed_default_ability(&mut mgr, 5, 30);
    mgr.get_entity_mut(NPC)
        .unwrap()
        .abilities
        .add_ability(NPC_DEFAULT_ABILITY);
    seed_target_with_threat(&mut mgr, NPC, PLAYER, [13.0, 0.0, 0.0]);
    mgr.get_entity_mut(NPC).unwrap().leash.step_back_at = Some(Instant::now());
    let logs = LogCapture::install();
    ai_tick(&mut mgr).await;
    assert!(waypoint(&mgr).is_none());
    assert!(logs
        .all()
        .iter()
        .any(|c| c.has_field("decision_outcome", "step_back_cooling")));
}

// ── Cover (NA22/NA23 keep their slot) ──────────────────────────────────

/// NPC holding the slot at (4,0,0) facing +X, player at `player`.
fn npc_in_cover(player: [f32; 3]) -> SpaceManager {
    use super::super::npc_ai_cover::{make_cover_fixture, node};
    let mut mgr = make_cover_fixture(
        [4.0, 0.0, 0.0],
        [4.0, 0.0, 0.0],
        vec![node(50, 0, 4.0, 0.0, 0.0)],
    );
    mgr.cover
        .reservations
        .lock()
        .unwrap()
        .reserve_for_entity(
            EntityId(NPC as i32),
            CoverSlotKey {
                chunk_id: 50,
                node_id: 0,
            },
        )
        .unwrap();
    seed_target_with_threat(&mut mgr, NPC, PLAYER, player);
    mgr
}

/// In cover and not flanked, the NPC keeps its slot however close the
/// player comes.
#[tokio::test]
async fn an_npc_in_cover_keeps_its_slot() {
    let mut mgr = npc_in_cover([5.0, 0.0, 0.0]);
    ai_tick(&mut mgr).await;
    assert!(waypoint(&mgr).is_none(), "no step out of cover");
    assert!(mgr.get_entity(NPC).unwrap().leash.step_back_at.is_none());
}

/// Flanked from behind at 1 u, the NPC gives the slot up and then steps back.
#[tokio::test]
async fn a_flanked_npc_steps_back() {
    let mut mgr = npc_in_cover([3.0, 0.0, 0.0]);
    ai_tick(&mut mgr).await;
    let held = mgr
        .cover
        .reservations
        .lock()
        .unwrap()
        .slot_for_entity(EntityId(NPC as i32));
    assert_eq!(held, None, "flanked: the slot is released");
    ai_tick(&mut mgr).await;
    let w = waypoint(&mgr).expect("then it steps back");
    assert!(w.x > 4.0, "away from the player behind it: {w:?}");
}

// ── castle_cellblock.nav ───────────────────────────────────────────────

/// `MessHall_Guard1`'s spawn, on the main interior island.
const MESSHALL: [f32; 3] = [-96.25, 34.591, -91.59];

/// A Fighting ranged NPC on the real Cellblock mesh at `npc`, a player at
/// `player`.
fn cellblock(npc: Vector3, player: Vector3) -> SpaceManager {
    let nav = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/spaces/castle_cellblock.nav");
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    let space_id = mgr.space_id_for_world("Castle_CellBlock").unwrap();
    mgr.spaces.get_mut(&space_id).unwrap().navmesh =
        Some(NavMesh::load(&nav).expect("load castle_cellblock.nav"));
    mgr.create_entity(NPC, "Castle_CellBlock", [npc.x, npc.y, npc.z], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(NPC).unwrap();
    e.class_id = 0x04;
    e.spawn_position = Some(npc);
    e.leash.distance_override = Some(10_000.0);
    e.aoi_radius = 2_000.0;
    e.abilities.add_ability(NPC_DEFAULT_ABILITY);
    crate::cell::service::npc_ai::force_ai_state(e, AiState::Fighting);
    e.stats.get_mut(HEALTH).unwrap().update(0, 100, 100);
    mgr.create_entity(
        PLAYER,
        "Castle_CellBlock",
        [player.x, player.y, player.z],
        [0.0; 3],
    )
    .unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER as i32);
    p.stats.get_mut(HEALTH).unwrap().update(0, 100, 100);
    mgr.connect_entity(PLAYER);
    let _ = mgr.compute_aoi_changes();
    seed_default_ability(&mut mgr, 0, 30);
    mgr.get_entity_mut(NPC)
        .unwrap()
        .threat_list
        .insert(PLAYER, 10.0);
    mgr
}

fn mess_hall_floor() -> Vector3 {
    let nav = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/spaces/castle_cellblock.nav");
    NavMesh::load(&nav)
        .unwrap()
        .get_nearest_point(&Vector3::new(MESSHALL[0], MESSHALL[1], MESSHALL[2]))
}

/// In the mess hall the step lands on the navmesh floor, outside the
/// comfort range.
#[tokio::test]
async fn on_the_cellblock_mesh_the_step_lands_on_the_floor() {
    let at = mess_hall_floor();
    let player = Vector3::new(at.x + 1.0, at.y, at.z);
    let mut mgr = cellblock(at, player);
    ai_tick(&mut mgr).await;
    let w = waypoint(&mgr).expect("the guard steps back");
    assert!(mgr.is_position_valid(NPC, &w), "on the mesh: {w:?}");
    let floor = mgr.get_navmesh_height(NPC, w.x, at.y, w.z).unwrap();
    assert!((w.y - floor).abs() <= 0.3, "on the floor: {w:?} / {floor}");
    assert!(
        horizontal(&w, &player) > 2.0,
        "outside the comfort range: {w:?}"
    );
}

/// Back to a wall: the slide gains nothing, so the NPC fires from where it
/// is (and the cooldown starts, so it does not re-plan every tick).
#[tokio::test]
async fn on_the_cellblock_mesh_a_cornered_npc_fires_instead() {
    let at = mess_hall_floor();
    let probe = cellblock(at, Vector3::new(at.x + 50.0, at.y, at.z));
    // Find a wall the mess-hall floor runs straight into.
    let (wall, dir) = (0..16)
        .map(|i| {
            let a = i as f32 * std::f32::consts::TAU / 16.0;
            (a.cos(), a.sin())
        })
        .find_map(|(dx, dz)| {
            let far = Vector3::new(at.x + dx * 30.0, at.y, at.z + dz * 30.0);
            let wall = probe.move_along_navmesh(NPC, &at, &far)?;
            if horizontal(&wall, &at) > 29.0 {
                return None;
            }
            let push = Vector3::new(wall.x + dx * 5.0, wall.y, wall.z + dz * 5.0);
            let slid = probe.move_along_navmesh(NPC, &wall, &push)?;
            (horizontal(&slid, &wall) < 0.2).then_some((wall, (dx, dz)))
        })
        .expect("the mess hall has a wall square to some heading");
    let player = Vector3::new(wall.x - dir.0, wall.y, wall.z - dir.1);
    let mut mgr = cellblock(wall, player);
    let logs = LogCapture::install();
    ai_tick(&mut mgr).await;
    assert!(waypoint(&mgr).is_none(), "no step into the wall");
    assert!(mgr.get_entity(NPC).unwrap().leash.step_back_at.is_some());
    assert!(logs
        .all()
        .iter()
        .any(|c| c.has_field("decision_outcome", "step_back_cornered")));
}
