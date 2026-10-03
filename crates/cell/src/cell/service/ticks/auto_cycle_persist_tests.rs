//! Every `BSF_AUTO_CYCLING` transition is saved, not just the button press,
//! and a press at a target behind a wall tells the player once.
//!
//! Colo UAT 2026-10-03: a loop the server stopped (friendly NPC selected,
//! target killed) left `sgw_player.state_field = 2`, because only the
//! `setAutoCycle` handler sent `StateFieldUpdate`. The next login restored a
//! lit button and an armed loop the player had watched switch off. Each test
//! here fails if its site goes back to a bare `onStateFieldUpdate` broadcast.

use super::tests::{empty_engine, make_auto_cycle_mgr};
use super::*;
use cimmeria_wire::state_field::{BSF_AUTO_CYCLING, BSF_IN_COMBAT};

/// `make_auto_cycle_mgr`'s player 1 carries `player_id = 100`.
const PLAYER_ID: i32 = 100;

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut v = Vec::new();
    while let Ok(m) = rx.try_recv() {
        v.push(m);
    }
    v
}

/// The `(player_id, state_field)` of every save in `msgs`.
fn saves(msgs: &[CellToBaseMsg]) -> Vec<(i32, u32)> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::StateFieldUpdate {
                player_id,
                state_field,
            } => Some((*player_id, *state_field)),
            _ => None,
        })
        .collect()
}

/// The tick's `not_hostile` stop (#1144) saves the cleared bit.
#[tokio::test]
async fn tick_stop_on_friendly_npc_saves_the_cleared_bit() {
    let mut mgr = make_auto_cycle_mgr();
    mgr.get_entity_mut(50).unwrap().faction = 1;
    mgr.get_entity_mut(1).unwrap().state_field |= BSF_AUTO_CYCLING;

    let (tx, mut rx) = mpsc::channel(64);
    auto_cycle_tick(&tx, &mut mgr, &empty_engine()).await;

    assert_eq!(mgr.get_entity(1).unwrap().state_field & BSF_AUTO_CYCLING, 0);
    assert_eq!(saves(&drain(&mut rx)), vec![(PLAYER_ID, 0)]);
}

/// The tick's stop on a target that is gone saves the cleared bit, and the
/// save is masked: the transient `BSF_InCombat` riding along never reaches
/// the database.
#[tokio::test]
async fn tick_stop_on_missing_target_saves_only_the_masked_bits() {
    let mut mgr = make_auto_cycle_mgr();
    mgr.get_entity_mut(1).unwrap().state_field |= BSF_AUTO_CYCLING | BSF_IN_COMBAT;
    mgr.get_entity_mut(1).unwrap().current_target_id = Some(9_999);

    let (tx, mut rx) = mpsc::channel(64);
    auto_cycle_tick(&tx, &mut mgr, &empty_engine()).await;

    assert_eq!(saves(&drain(&mut rx)), vec![(PLAYER_ID, 0)]);
}

/// The death sweep (target killed) saves the cleared bit for the player
/// whose loop it stopped.
#[tokio::test]
async fn target_death_saves_the_cleared_bit() {
    let mut mgr = make_auto_cycle_mgr();
    mgr.get_entity_mut(1).unwrap().state_field |= BSF_AUTO_CYCLING;

    let (tx, mut rx) = mpsc::channel(256);
    assert!(crate::cell::abilities::resolve_death_for_test(50, 1, &tx, &mut mgr).await);

    let p = mgr.get_entity(1).unwrap();
    assert!(!p.abilities.auto_cycle, "the death sweep stops the loop");
    assert_eq!(saves(&drain(&mut rx)), vec![(PLAYER_ID, 0)]);
}

/// The first committed shot of an armed loop lights the bit (a right-click
/// before any press) and saves it, so the saved value matches the button.
#[tokio::test]
async fn first_commit_arm_saves_the_lit_bit() {
    let mut mgr = make_auto_cycle_mgr();
    {
        let p = mgr.get_entity_mut(1).unwrap();
        p.abilities.auto_cycle_ability_id = None;
        assert_eq!(
            p.state_field & BSF_AUTO_CYCLING,
            0,
            "fixture: bit starts clear"
        );
    }

    let (tx, mut rx) = mpsc::channel(256);
    assert!(crate::cell::abilities::handle_use_ability(1, 7, 50, &tx, &mut mgr).await);

    assert_ne!(mgr.get_entity(1).unwrap().state_field & BSF_AUTO_CYCLING, 0);
    let s = saves(&drain(&mut rx));
    assert_eq!(s.len(), 1, "one save for the one transition: {s:?}");
    assert_eq!(s[0], (PLAYER_ID, BSF_AUTO_CYCLING));
}

/// A press whose immediate shot meets a wall sends the one no-line-of-sight
/// notice; the tick passes that follow stay silent until the line clears.
/// Seen on colo: the press and the first tick each sent one, so chat showed
/// the line twice.
#[tokio::test]
async fn press_at_a_target_behind_a_wall_notifies_once() {
    use cimmeria_common::Vector3;
    let mut mgr = make_auto_cycle_mgr();
    let sid = mgr.get_entity_space_id(1).unwrap();
    mgr.spaces.get_mut(&sid).unwrap().occluder =
        Some(crate::cell::space_manager::occluder_fixtures::corner());
    mgr.get_entity_mut(1).unwrap().position = Vector3::new(5.0, 0.0, 10.0);
    mgr.get_entity_mut(50).unwrap().position = Vector3::new(21.0, 0.0, 18.0);
    {
        // Off: the press below is the transition that fires.
        let p = mgr.get_entity_mut(1).unwrap();
        p.abilities.auto_cycle = false;
        p.abilities.auto_cycle_ability_id = None;
        p.abilities.last_fired_ability_id = Some(7);
    }
    let los_errors = |msgs: &[CellToBaseMsg]| {
        msgs.iter()
            .filter(|m| {
                matches!(m, CellToBaseMsg::EntityMethodCall { entity_id: 1, method_index, args }
                    if *method_index == crate::mercury::method_idx::ON_ERROR_CODE
                        && args[5..] == [39, 0])
            })
            .count()
    };
    let (tx, mut rx) = mpsc::channel(256);

    let set_auto_cycle = crate::cell::cell_methods::player::SET_AUTO_CYCLE;
    assert!(
        crate::cell::cell_methods::player::dispatch(
            1,
            set_auto_cycle,
            &[1],
            &tx,
            &mut mgr,
            &empty_engine()
        )
        .await
    );
    for _ in 0..3 {
        auto_cycle_tick(&tx, &mut mgr, &empty_engine()).await;
    }

    assert_eq!(
        los_errors(&drain(&mut rx)),
        1,
        "one notice for press + ticks"
    );
    assert!(
        mgr.get_entity(1).unwrap().abilities.auto_cycle,
        "the loop stays armed"
    );
}
