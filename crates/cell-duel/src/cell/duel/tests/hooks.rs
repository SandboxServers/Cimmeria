//! `DuelPlugin`'s tick and lifecycle hooks, fired the way core fires them
//! (#962): the travel sites and the death resolver call
//! `SpaceManager::fire_entity_hook` / `fire_death_hook`, and the cell loop
//! runs the tick stages through the installed registry. Each hook must reach
//! the same duel end the inline call reached before the move; without the
//! plugin, nothing ends (the tick's sweep is the only safety net left).

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use cimmeria_cell_world::cell::plugin::{CellPlugins, DeathHookPoint, EntityHookPoint, TickStage};
use cimmeria_wire::cell::client_methods::duel::{
    TEXT_DUEL_ABORTED, TEXT_DUEL_LOST, TEXT_DUEL_LOST_TELEPORT, TEXT_DUEL_WON,
};

use super::end_paths::ended_row;
use super::engage::{aoi_mgr, engage};
use super::*;
use crate::cell::duel::limits::CHALLENGE_TIMEOUT;
use crate::cell::duel::DuelResources;
use crate::test_support::LogCapture;

/// The travel hook ends the traveller's engaged duel with them as the loser
/// (`EDUEL_DEFEAT_Teleport`), as the inline `duel::on_travel` at each travel
/// site did. With no plugin installed the same hook fire changes nothing.
#[tokio::test]
async fn the_travel_hook_ends_an_engaged_duel() {
    let capture = LogCapture::install();
    let mut mgr = aoi_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    engage(&mut mgr, &tx, &mut rx).await;
    drain(&mut rx);

    mgr.fire_entity_hook(EntityHookPoint::BeforeTravelSend, A_EID, &tx)
        .await;
    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_DUEL_LOST_TELEPORT]);
    assert_eq!(lines_to(&sent, B_EID), vec![TEXT_DUEL_WON]);
    assert!(!mgr.resources.duels().is_busy(A_PID));
    assert!(ended_row(&capture).has_field("reason", "teleport"));

    // Without the plugin the hook point has no subscriber.
    let mut bare = aoi_mgr();
    let (tx2, mut rx2) = mpsc::channel(256);
    engage(&mut bare, &tx2, &mut rx2).await;
    drain(&mut rx2);
    bare.install_plugins(CellPlugins::empty());
    bare.fire_entity_hook(EntityHookPoint::BeforeTravelSend, A_EID, &tx2)
        .await;
    assert!(drain(&mut rx2).is_empty());
    assert!(bare.resources.duels().can_harm(A_PID, B_PID));
}

/// The death hook ends the victim's engaged duel (`EDUEL_DEFEAT_Health`), as
/// the inline `duel::on_death` in the death resolver did.
#[tokio::test]
async fn the_death_hook_ends_an_engaged_duel() {
    let capture = LogCapture::install();
    let mut mgr = aoi_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    engage(&mut mgr, &tx, &mut rx).await;
    drain(&mut rx);

    mgr.fire_death_hook(DeathHookPoint::AfterPlayerThreatPurge, B_EID, C_EID, &tx)
        .await;
    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, B_EID), vec![TEXT_DUEL_LOST]);
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_DUEL_WON]);
    let row = ended_row(&capture);
    assert!(row.has_field("reason", "health"), "{row:?}");
    assert!(
        row.has_field("killer_entity_id", &C_EID.to_string()),
        "the killer reaches the end row: {row:?}"
    );
}

/// The duel tick runs at `TickStage::AfterGateCrossing` and nowhere else,
/// the position the inline `duel::tick::run` call had in the cell loop: an
/// expired challenge survives the other stages and is aborted at this one.
#[tokio::test]
async fn the_duel_tick_runs_at_the_gate_crossing_stage() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(64);
    let Some(opened) = Instant::now().checked_sub(CHALLENGE_TIMEOUT + Duration::from_secs(1))
    else {
        return; // The clock is too young to open a challenge in the past.
    };
    challenge(&mut mgr, &tx, (A_EID, A_PID), (B_EID, B_PID), opened).await;
    drain(&mut rx);
    let plugins = mgr.plugins().clone();

    for stage in [TickStage::AfterRingTransport, TickStage::AfterStatBuffs] {
        plugins.run_tick(stage, &tx, &mut mgr).await;
        assert!(
            mgr.resources.duels().pending_for(B_PID).is_some(),
            "{stage:?} must not run the duel tick"
        );
    }
    plugins
        .run_tick(TickStage::AfterGateCrossing, &tx, &mut mgr)
        .await;
    assert!(mgr.resources.duels().pending_for(B_PID).is_none());
    let sent = drain(&mut rx);
    assert_eq!(lines_to(&sent, A_EID), vec![TEXT_DUEL_ABORTED]);
    assert_eq!(lines_to(&sent, B_EID), vec![TEXT_DUEL_ABORTED]);
}
