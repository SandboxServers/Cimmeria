//! CR-02: the duration-effect timer (`onTimerUpdate` type 5) carries an
//! absolute `BigWorldTimeComplete` on the server's game clock.
//!
//! The client's `EffectSet` handler (`0x00e09160`) creates the buff/debuff
//! entry only when its clock is below `BigWorldTimeComplete`, so the old
//! relative duration drew no icon once the clock had passed it.

use std::time::Instant;

use tokio::sync::mpsc;

use super::register_active_effect;
use super::tests::{make_dot_effect, make_mgr};
use crate::mercury::game_clock::{game_time_secs, init};

#[tokio::test]
async fn duration_effect_timer_expiry_is_absolute_on_the_game_clock() {
    let mut mgr = make_mgr();
    let mut effect = make_dot_effect(2, 1.0, 10);
    effect.effect_id = 8890;
    mgr.effect_defs.insert(effect.effect_id, effect.clone());
    let (tx, mut rx) = mpsc::channel(64);
    // Past the epoch, so a relative expiry cannot pass the window.
    init();
    while game_time_secs() < 0.01 {
        std::thread::sleep(std::time::Duration::from_millis(2));
    }

    let before = game_time_secs();
    register_active_effect(&mut mgr, 1, 2, &effect, Instant::now(), &tx).await;
    let after = game_time_secs();

    let mut timers = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let crate::cell::messages::CellToBaseMsg::EntityMethodCall {
            entity_id: 1,
            method_index,
            args,
        } = msg
        {
            if method_index == crate::cell::client_methods::being::ON_TIMER_UPDATE {
                timers.push(args);
            }
        }
    }
    assert_eq!(timers.len(), 1, "one start timer");
    let total = f32::from_le_bytes(timers[0][13..17].try_into().unwrap());
    assert_eq!(total, effect.total_duration(), "TotalTime is the duration");
    let expiry = f32::from_le_bytes(timers[0][17..21].try_into().unwrap());
    assert!(
        (before + total..=after + total).contains(&expiry),
        "BigWorldTimeComplete {expiry} is not game time [{before}, {after}] + {total}"
    );
}
