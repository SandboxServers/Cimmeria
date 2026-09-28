//! How far a player sees (`PLAYER_AOI_RADIUS`, 150 m), and the leave margin
//! (`AOI_LEAVE_MARGIN`, 25 m) that stops an entity at the edge of view from
//! being removed and re-created every time it shifts a step.

use cimmeria_entity::cell_entity::{AOI_LEAVE_MARGIN, PLAYER_AOI_RADIUS};

use super::super::super::messages::CellToBaseMsg;
use super::super::SpaceManager;
use super::make_manager;

const PLAYER: u32 = 1;

/// A player at the origin, connected and initialised (so it is introducible
/// to other players too).
fn player_at(mgr: &mut SpaceManager, id: u32, x: f32) {
    mgr.create_entity(id, "Agnos", [x, 0.0, 0.0], [0.0; 3])
        .unwrap();
    {
        let e = mgr.get_entity_mut(id).unwrap();
        e.account_id = Some(id);
        e.player_id = Some(id as i32);
    }
    mgr.connect_entity(id);
    mgr.get_entity_mut(id).unwrap().archetype_id = Some(4);
}

fn npc_at(mgr: &mut SpaceManager, x: f32) -> u32 {
    let id = mgr.allocate_npc_id();
    mgr.spawn_npc(id, "Agnos", [x, 0.0, 0.0], [0.0; 3]).unwrap();
    id
}

fn move_to(mgr: &mut SpaceManager, id: u32, x: f32) {
    mgr.update_position_preserving_facing(id, [x, 0.0, 0.0], [0.0; 3]);
}

fn entered(events: &[CellToBaseMsg], witness: u32, entity: u32) -> bool {
    events.iter().any(|e| {
        matches!(e, CellToBaseMsg::EnteredAoI { witness_id, entity_id, .. }
            if *witness_id == witness && *entity_id == entity)
    })
}

fn left(events: &[CellToBaseMsg], witness: u32, entity: u32) -> bool {
    events.iter().any(|e| {
        matches!(e, CellToBaseMsg::LeftAoI { witness_id, entity_id }
            if *witness_id == witness && *entity_id == entity)
    })
}

/// The owner's call (2026-09-28): players see 150 m, the legacy server's
/// `grid_vision_distance`, and just inside the client's 160 m cull. NPC
/// perception keeps its own 100 m default.
#[test]
fn a_player_sees_150m_and_an_npc_keeps_its_100m_perception() {
    assert_eq!(PLAYER_AOI_RADIUS, 150.0);
    assert_eq!(AOI_LEAVE_MARGIN, 25.0);

    let mut mgr = make_manager();
    player_at(&mut mgr, PLAYER, 0.0);
    let npc = npc_at(&mut mgr, 50.0);

    assert_eq!(mgr.get_entity(PLAYER).unwrap().aoi_radius, 150.0);
    assert_eq!(mgr.get_entity(PLAYER).unwrap().aoi_leave_radius(), 175.0);
    assert_eq!(
        mgr.get_entity(npc).unwrap().aoi_radius,
        100.0,
        "NPC target-loss and leash read this; the player radius must not move it"
    );
}

/// Revert guard: at the old 100 m radius an NPC 140 m away was never sent.
#[test]
fn an_entity_140m_away_is_in_view() {
    let mut mgr = make_manager();
    player_at(&mut mgr, PLAYER, 0.0);
    let npc = npc_at(&mut mgr, 140.0);
    let events = mgr.compute_aoi_changes();
    assert!(entered(&events, PLAYER, npc));
}

#[test]
fn an_entity_160m_away_is_not_introduced() {
    let mut mgr = make_manager();
    player_at(&mut mgr, PLAYER, 0.0);
    let npc = npc_at(&mut mgr, 160.0);
    let events = mgr.compute_aoi_changes();
    assert!(!entered(&events, PLAYER, npc));
}

/// Revert guard for the leave margin: with a single radius the NPC at 170 m
/// leaves view (and would be re-created on the next step back).
#[test]
fn an_entity_in_view_stays_until_past_the_leave_radius() {
    let mut mgr = make_manager();
    player_at(&mut mgr, PLAYER, 0.0);
    let npc = npc_at(&mut mgr, 140.0);
    assert!(entered(&mgr.compute_aoi_changes(), PLAYER, npc));

    move_to(&mut mgr, npc, 170.0);
    let events = mgr.compute_aoi_changes();
    assert!(
        !left(&events, PLAYER, npc),
        "170 m is inside the 175 m leave radius"
    );
    assert!(!entered(&events, PLAYER, npc), "and it is not re-created");

    move_to(&mut mgr, npc, 176.0);
    assert!(left(&mgr.compute_aoi_changes(), PLAYER, npc));
}

#[test]
fn an_entity_that_left_needs_the_enter_radius_to_come_back() {
    let mut mgr = make_manager();
    player_at(&mut mgr, PLAYER, 0.0);
    let npc = npc_at(&mut mgr, 140.0);
    mgr.compute_aoi_changes();
    move_to(&mut mgr, npc, 180.0);
    assert!(left(&mgr.compute_aoi_changes(), PLAYER, npc));

    move_to(&mut mgr, npc, 160.0);
    assert!(
        !entered(&mgr.compute_aoi_changes(), PLAYER, npc),
        "between the two radii an entity out of view stays out"
    );

    move_to(&mut mgr, npc, 149.0);
    assert!(entered(&mgr.compute_aoi_changes(), PLAYER, npc));
}

/// Two players 140 m apart see each other, both ways.
#[test]
fn two_players_140m_apart_see_each_other() {
    let mut mgr = make_manager();
    player_at(&mut mgr, 1, 0.0);
    player_at(&mut mgr, 2, 140.0);
    let events = mgr.compute_aoi_changes();
    assert!(entered(&events, 1, 2));
    assert!(entered(&events, 2, 1));
}
