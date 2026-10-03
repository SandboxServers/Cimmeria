//! A `setAutoCycle` press whose immediate shot meets a wall tells the player
//! once. Seen on colo 2026-10-03: the press and the next tick each sent the
//! no-line-of-sight notice (39), so chat showed the line twice.

use super::tests::{empty_engine, make_auto_cycle_mgr};
use super::*;

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut v = Vec::new();
    while let Ok(m) = rx.try_recv() {
        v.push(m);
    }
    v
}

/// The press's immediate shot sends the one notice and marks it sent; the
/// tick passes that follow stay silent until the line clears.
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
