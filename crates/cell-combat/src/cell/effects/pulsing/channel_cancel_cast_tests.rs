//! A channel's cancel clears its effect timer with the cast that registered
//! it (the AB-T4 carry-over into AB-T6/T7).
//!
//! The cancel runs outside any cast scope (a different ability's launch, the
//! movement tick), so before the fix its `onTimerUpdate` clear logged
//! `cast_id` empty and the clear could not be joined to its cast. Each test
//! registers the channel inside a cast scope, leaves it, cancels, and reads
//! the clear's `abilities.wire` row.

use std::time::Instant;

use tokio::sync::mpsc;

use super::tests::{make_dot_effect, make_mgr};
use super::{
    cancel_channels_for_invoker_ability, cancel_channels_from_attacker, register_active_effect,
};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{Captured, LogCapture};

const CAST: i32 = 4242;
const ABILITY: i32 = 610;
const EFFECT: i32 = 9610;

/// NPC 2 channels on player 1 (the timer goes to a player's client only)
/// under cast [`CAST`].
async fn channel_on_player(
    mgr: &mut SpaceManager,
    tx: &mpsc::Sender<crate::cell::messages::CellToBaseMsg>,
) {
    let mut channel = make_dot_effect(0, 0.5, 1);
    channel.effect_id = EFFECT;
    channel.ability_id = ABILITY;
    mgr.effect_defs.insert(EFFECT, channel.clone());
    let outer = mgr.enter_cast_scope(Some(CAST));
    register_active_effect(mgr, 1, 2, &channel, Instant::now(), tx).await;
    mgr.exit_cast_scope(outer);
    assert_eq!(
        mgr.current_cast_id(),
        None,
        "the cancel runs outside the cast"
    );
}

fn cancel_clear(logs: &[Captured]) -> Captured {
    logs.iter()
        .find(|c| {
            c.target == "abilities.wire"
                && c.has_field("event", "wire_sent")
                && c.has_field("method", "onTimerUpdate")
                && c.has_field("origin", "channel_cancel")
        })
        .cloned()
        .unwrap_or_else(|| panic!("no channel_cancel timer clear: {logs:#?}"))
}

#[tokio::test]
async fn attacker_cancel_clears_the_timer_with_the_channels_cast_id() {
    let mut mgr = make_mgr();
    let (tx, _rx) = mpsc::channel(64);
    channel_on_player(&mut mgr, &tx).await;
    let logs = LogCapture::install();

    let cancelled = cancel_channels_from_attacker(2, None, &tx, &mut mgr).await;

    assert_eq!(cancelled, 1);
    let row = cancel_clear(&logs.all());
    assert!(row.has_field("cast_id", &CAST.to_string()), "{row:?}");
    assert!(row.has_field("action", "clear"), "{row:?}");
}

#[tokio::test]
async fn movement_cancel_clears_the_timer_with_the_channels_cast_id() {
    let mut mgr = make_mgr();
    let (tx, _rx) = mpsc::channel(64);
    channel_on_player(&mut mgr, &tx).await;
    let logs = LogCapture::install();

    let cancelled = cancel_channels_for_invoker_ability(2, ABILITY, &tx, &mut mgr).await;

    assert_eq!(cancelled, 1);
    let row = cancel_clear(&logs.all());
    assert!(row.has_field("cast_id", &CAST.to_string()), "{row:?}");
}
