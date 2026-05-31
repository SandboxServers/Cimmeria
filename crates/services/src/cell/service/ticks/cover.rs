//! Cover-detection tick — runs the player cover-detection sweep and
//! dispatches `OnPlayerEnteredCover` / `OnPlayerLeftCover` /
//! `OnPlayerInCoverDuration` content-engine events.
//!
//! Schedule: once per second (configurable via the cell loop). 100-ms
//! resolution isn't needed for cover (player movement integrates over
//! ~200 ms tick boundaries on the client anyway), and the spatial query
//! cost scales linearly with player count.

use std::time::Instant;

use cimmeria_common::EntityId;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use crate::cell::content;
use crate::cell::cover::{
    run_detection_tick, COVER_DURATION_MILESTONES_SECS, COVER_PROXIMITY_RADIUS,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Run one cover-detection tick. Pulls the current player list from
/// `space_mgr.spaces[*].players`, runs the per-player proximity test
/// against the cover index, and dispatches enter/leave/duration events
/// through the content engine.
///
/// Cheap on quiet ticks: when no player is near any cover node, the
/// inner spatial query short-circuits (grid lookup returns empty cells)
/// and the dispatched event vec is empty.
#[tracing::instrument(
    name = "cover.detection_tick",
    level = "debug",
    skip_all,
    fields(player_count = tracing::field::Empty, events = tracing::field::Empty),
)]
pub(in crate::cell::service) async fn cover_detection_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    // Cheap fast-path: if no cover data was loaded, the detection state
    // would always come back empty. Skip the per-tick iteration entirely.
    if space_mgr.cover.node_count() == 0 {
        return;
    }

    // Collect (entity_id, position) for every player across every space.
    // EntityId + Vector3 are Copy — cheap to clone into a Vec so we can
    // drop the immutable borrow before the fire_* dispatch loop (which
    // needs &mut space_mgr).
    let players: Vec<(EntityId, cimmeria_common::Vector3)> = space_mgr
        .spaces
        .values()
        .flat_map(|space| {
            space.players.iter().filter_map(|&eid| {
                let entity = space.entities.get(&eid)?;
                Some((EntityId(eid as i32), entity.position))
            })
        })
        .collect();

    tracing::Span::current().record("player_count", players.len());

    if players.is_empty() {
        return;
    }

    let tick = run_detection_tick(
        &space_mgr.cover,
        &players,
        &mut space_mgr.cover_detection,
        Instant::now(),
        COVER_PROXIMITY_RADIUS,
        COVER_DURATION_MILESTONES_SECS,
    );

    if tick.is_empty() {
        return;
    }

    let event_count = tick.entered.len() + tick.left.len() + tick.duration_milestones.len();
    tracing::Span::current().record("events", event_count);

    // Dispatch each event through the content engine. The fire_*
    // functions need &mut space_mgr for the mission-context lookups
    // and executor; we drop the players Vec above, so the loop is free
    // to mutate.
    for entered in tick.entered {
        let player_id = entered.player_id.0 as u32;
        // Look up the player's DB player_id (i32) from the entity — the
        // fire_* helpers thread that through to the content engine's
        // mission-context populator.
        let db_player_id = space_mgr
            .get_entity(player_id)
            .and_then(|e| e.player_id)
            .unwrap_or(0);
        content::fire_cover_entered(
            player_id,
            db_player_id,
            entered.cover_set_id,
            sql_height_name(entered.representative_height),
            sql_quality_name(entered.representative_quality),
            engine,
            tx,
            space_mgr,
        )
        .await;
    }

    for left in tick.left {
        let player_id = left.player_id.0 as u32;
        let db_player_id = space_mgr
            .get_entity(player_id)
            .and_then(|e| e.player_id)
            .unwrap_or(0);
        content::fire_cover_left(
            player_id,
            db_player_id,
            left.cover_set_id,
            engine,
            tx,
            space_mgr,
        )
        .await;
    }

    for milestone in tick.duration_milestones {
        let player_id = milestone.player_id.0 as u32;
        let db_player_id = space_mgr
            .get_entity(player_id)
            .and_then(|e| e.player_id)
            .unwrap_or(0);
        content::fire_cover_duration(
            player_id,
            db_player_id,
            milestone.cover_set_id,
            milestone.seconds,
            engine,
            tx,
            space_mgr,
        )
        .await;
    }
}

fn sql_height_name(h: crate::cell::cover::CoverHeight) -> &'static str {
    use crate::cell::cover::CoverHeight;
    match h {
        CoverHeight::Low => "HEIGHT_Low",
        CoverHeight::Mid => "HEIGHT_Mid",
        CoverHeight::High => "HEIGHT_High",
        CoverHeight::Los => "HEIGHT_LOS",
    }
}

fn sql_quality_name(q: crate::cell::cover::CoverQuality) -> &'static str {
    use crate::cell::cover::CoverQuality;
    match q {
        CoverQuality::Good => "QUALITY_Good",
        CoverQuality::Better => "QUALITY_Better",
        CoverQuality::Best => "QUALITY_Best",
        CoverQuality::None_ => "QUALITY_None",
    }
}
