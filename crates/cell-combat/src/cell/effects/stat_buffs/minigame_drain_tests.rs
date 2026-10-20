//! The minigame drain on the combat seam (MG-24, N3): the tick and the flush
//! each start the launches an effect script queued. The effect script has its
//! own tests; these pin the two calls that carry a queued launch to a
//! `StartMinigame`, since nothing else would notice if either went.

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_cell_world::cell::minigame_launch::MinigameLaunchRequest;

use super::tests::{make_mgr, PLAYER};
use super::{flush_stat_buff_timers, stat_buff_tick_at};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Queue a launch for `PLAYER`, as an effect script does. The entity's
/// player id is 100 in `make_mgr`, so the launch names that player.
fn queue_launch(mgr: &mut SpaceManager) {
    mgr.minigame_launch.queue.push(MinigameLaunchRequest {
        entity_id: PLAYER,
        player_id: 100,
        game_name: "Livewire".into(),
        difficulty: 1,
        on_victory_chains: vec![],
        on_failure_chains: vec![],
        source: "effect_script",
    });
}

/// How many `StartMinigame` messages are waiting on the channel.
fn starts(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> usize {
    std::iter::from_fn(|| rx.try_recv().ok())
        .filter(|m| matches!(m, CellToBaseMsg::StartMinigame { .. }))
        .count()
}

/// The tick drains a queued launch even with no stat-buff work, which is the
/// early-return path of the tick.
/// Revert: deleting the tick's drain call sends no `StartMinigame`.
#[tokio::test]
async fn the_tick_drains_the_launch_queue_with_no_stat_buff_work() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    queue_launch(&mut mgr);

    stat_buff_tick_at(Instant::now(), &tx, &mut mgr).await;

    assert_eq!(starts(&mut rx), 1);
    assert!(mgr.minigame_launch.queue.is_empty());
}

/// The flush drains a queued launch for the entity it flushes.
/// Revert: deleting the flush's drain call sends no `StartMinigame`.
#[tokio::test]
async fn the_flush_drains_the_launch_queue() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    queue_launch(&mut mgr);

    flush_stat_buff_timers(PLAYER, Instant::now(), &tx, &mut mgr).await;

    assert_eq!(starts(&mut rx), 1);
    assert!(mgr.minigame_launch.queue.is_empty());
}
