//! Player ↔ player AoI introduction: the gate that keeps a still-loading
//! player out of other players' view, and the live-state snapshot a player
//! observee carries on `EnteredAoI`.

use super::super::super::messages::CellToBaseMsg;
use super::make_manager;

/// Create a player cell entity the way production does: identity stamped at
/// `CreateEntity`, nothing else yet (the client is still loading the map).
fn create_loading_player(mgr: &mut super::super::SpaceManager, id: u32, pos: [f32; 3]) {
    mgr.create_entity(id, "Agnos", pos, [0.0; 3]).unwrap();
    let e = mgr.get_entity_mut(id).unwrap();
    e.account_id = Some(id);
    e.player_id = Some(id as i32);
}

/// Drive a loading player through `ConnectEntity` + the part of
/// `InitPlayerState` the AoI gate keys on.
fn finish_loading(mgr: &mut super::super::SpaceManager, id: u32) {
    mgr.connect_entity(id);
    mgr.get_entity_mut(id).unwrap().archetype_id = Some(4);
}

fn entered_events(events: &[CellToBaseMsg], witness: u32, entity: u32) -> Vec<&CellToBaseMsg> {
    events
        .iter()
        .filter(|e| {
            matches!(e, CellToBaseMsg::EnteredAoI { witness_id, entity_id, .. }
                if *witness_id == witness && *entity_id == entity)
        })
        .collect()
}

/// Regression guard for the shared-world introduction race. A player's cell
/// entity exists from `CreateEntity`, so a nearby player's AoI tick sees it
/// for the whole map load. Introducing it then ships an NPC-shaped blank
/// entity AND marks the witness set, so the real player is never
/// re-introduced. It must stay out of view until connected *and*
/// initialised, then be introduced exactly once, as a player.
#[test]
fn loading_player_is_introduced_once_and_only_after_init() {
    let mut mgr = make_manager();
    create_loading_player(&mut mgr, 100, [10.0, 0.0, 10.0]);
    finish_loading(&mut mgr, 100);
    create_loading_player(&mut mgr, 200, [20.0, 0.0, 20.0]);

    // 200 is still loading the map.
    let events = mgr.compute_aoi_changes();
    assert!(
        entered_events(&events, 100, 200).is_empty(),
        "a player still loading the map must not be introduced to witnesses"
    );

    // Connected, but InitPlayerState has not landed (the cell loop can run an
    // AoI tick between the two messages) — stats/archetype are still unset.
    mgr.connect_entity(200);
    let events = mgr.compute_aoi_changes();
    assert!(
        entered_events(&events, 100, 200).is_empty(),
        "a connected-but-uninitialised player must not be introduced yet"
    );
    // The gate is one-directional: the newcomer's own view starts at connect,
    // so it already sees the player who was standing there.
    assert_eq!(
        entered_events(&events, 200, 100).len(),
        1,
        "the newcomer sees the already-present player from connect onward"
    );

    mgr.get_entity_mut(200).unwrap().archetype_id = Some(4);
    let events = mgr.compute_aoi_changes();
    let entered = entered_events(&events, 100, 200);
    assert_eq!(entered.len(), 1, "initialised player is introduced");
    match entered[0] {
        CellToBaseMsg::EnteredAoI {
            npc_data,
            player_data,
            ..
        } => {
            assert!(
                npc_data.is_none(),
                "a player must never ride the NPC cascade"
            );
            assert!(player_data.is_some(), "a player must carry PlayerAoIData");
        }
        _ => unreachable!(),
    }
    let events = mgr.compute_aoi_changes();
    assert!(
        entered_events(&events, 100, 200).is_empty(),
        "introduction is one-shot"
    );
}

/// The live half of the player-ghost cascade is a snapshot of the observee
/// at the moment it enters view: a player who is in combat with a target and
/// a half-empty health bar must be introduced that way, not as a fresh
/// full-health idle entity.
#[test]
fn player_observee_carries_its_live_state() {
    use cimmeria_entity::stats::HEALTH;

    let mut mgr = make_manager();
    create_loading_player(&mut mgr, 100, [10.0, 0.0, 10.0]);
    finish_loading(&mut mgr, 100);
    create_loading_player(&mut mgr, 200, [20.0, 0.0, 20.0]);
    finish_loading(&mut mgr, 200);
    {
        let e = mgr.get_entity_mut(200).unwrap();
        e.state_field = 0b1000; // BSF_InCombat
        e.current_target_id = Some(555);
        let health = e.stats.get_mut(HEALTH).unwrap();
        health.update(0, 40, 100);
    }
    let expected =
        super::super::super::messages::PlayerAoIData::from_entity(mgr.get_entity(200).unwrap());

    let events = mgr.compute_aoi_changes();
    let entered = entered_events(&events, 100, 200);
    let CellToBaseMsg::EnteredAoI { player_data, .. } = entered[0] else {
        unreachable!()
    };
    let live = player_data
        .as_ref()
        .expect("player observee carries live state");
    assert_eq!(live, &expected);
    assert_eq!(live.state_field, 0b1000);
    assert_eq!(live.target_id, 555);
    // Public stats only, in onStatUpdate wire form: HEALTH's (min, cur, max)
    // must be the damaged values, not the template default.
    let health_record: Vec<u8> = [HEALTH, 0, 40, 100]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    assert!(
        live.stat_update
            .windows(health_record.len())
            .any(|w| w == health_record.as_slice()),
        "stat_update must carry the live HEALTH tuple"
    );
}

/// NPCs are unaffected by the introduction gate and still ride `npc_data`.
#[test]
fn npc_observee_is_introduced_immediately_with_npc_data() {
    let mut mgr = make_manager();
    create_loading_player(&mut mgr, 100, [10.0, 0.0, 10.0]);
    finish_loading(&mut mgr, 100);
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc(npc, "Agnos", [12.0, 0.0, 12.0], [0.0; 3])
        .unwrap();

    let events = mgr.compute_aoi_changes();
    let entered = entered_events(&events, 100, npc);
    assert_eq!(entered.len(), 1);
    let CellToBaseMsg::EnteredAoI {
        npc_data,
        player_data,
        ..
    } = entered[0]
    else {
        unreachable!()
    };
    assert!(npc_data.is_some() && player_data.is_none());
}
