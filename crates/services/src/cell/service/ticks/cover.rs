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

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_common::Vector3;
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;

    use crate::cell::cover::{Cover, CoverHeight, CoverNode, CoverQuality};

    fn node(chunk_id: i32, x: f32, z: f32) -> CoverNode {
        CoverNode {
            chunk_id,
            node_id: 0,
            pos: Vector3::new(x, 0.0, z),
            orient: 0.0,
            height: CoverHeight::Mid,
            quality: CoverQuality::Best,
            tail: [0; 4],
        }
    }

    /// Castle space + one connected player at origin. The cover handle
    /// the caller passes in determines the data the tick scans.
    fn make_castle_with_player(cover: Cover) -> crate::cell::space_manager::SpaceManager {
        let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.is_player = true;
            p.player_id = Some(100);
        }
        mgr.connect_entity(1);
        mgr.cover = cover;
        mgr
    }

    /// Empty cover service → tick returns immediately without
    /// touching the per-player loop. Pins the fast-path: the
    /// `node_count() == 0` branch is the most-frequently-hit one on
    /// any cell that didn't load cover data (most spaces).
    #[tokio::test]
    async fn cover_detection_tick_skips_when_no_cover_data_loaded() {
        let mut mgr = make_castle_with_player(Cover::empty());
        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(16);

        cover_detection_tick(&tx, &mut mgr, &engine).await;

        assert!(
            rx.try_recv().is_err(),
            "empty cover data must short-circuit before any dispatch fires"
        );
        // Detection table must NOT have been touched (no insert).
        assert_eq!(mgr.cover_detection.tracked_player_count(), 0);
    }

    /// Cover data loaded but no players → tick exits via the
    /// `players.is_empty()` early return after the prune. Pins the
    /// second short-circuit gate.
    #[tokio::test]
    async fn cover_detection_tick_skips_when_no_players() {
        let cover = Cover::from_loaded(Vec::new(), vec![node(7, 0.0, 0.0)]);
        let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.cover = cover;
        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(16);

        cover_detection_tick(&tx, &mut mgr, &engine).await;
        assert!(rx.try_recv().is_err());
    }

    /// End-to-end of the tick: a player at the cover-node position
    /// fires `OnPlayerEnteredCover`, the registered chain matches and
    /// increments the player's counter. This pins the full pipeline:
    ///   tick → run_detection_tick → fire_cover_entered → resolve →
    ///   executor → entity.counters.
    /// Reverting the cover_detection_tick to the previous "no-op
    /// stub" would break this end-to-end.
    #[tokio::test]
    async fn cover_detection_tick_dispatches_entered_event_through_engine() {
        let cover = Cover::from_loaded(Vec::new(), vec![node(123, 0.0, 0.0)]);
        let mut mgr = make_castle_with_player(cover);

        let mut engine = cimmeria_content_engine::chain::ChainEngine::new();
        engine.register_chain(Chain {
            id: 0x7000_2700,
            name: "tick: any cover entered → bump counter".to_string(),
            enabled: true,
            trigger: Trigger::OnPlayerEnteredCover { cover_set_id: None },
            conditions: vec![],
            actions: vec![Action::IncrementCounter {
                counter_name: "tick_cover_entered".to_string(),
                amount: 1,
            }],
            priority: 0,
        });

        let (tx, _rx) = mpsc::channel(16);
        cover_detection_tick(&tx, &mut mgr, &engine).await;

        let player = mgr.get_entity(1).expect("player must still exist");
        assert_eq!(
            player.counters.get("tick_cover_entered"),
            Some(&1),
            "cover_detection_tick must dispatch the entered event through \
             fire_cover_entered → engine → executor; missing counter \
             indicates the tick stopped short of the dispatch loop or the \
             content-engine wiring is severed. Got counters: {:?}",
            player.counters,
        );
        // Tracked baseline must reflect the entry.
        assert_eq!(mgr.cover_detection.tracked_player_count(), 1);
    }

    /// `sql_height_name` and `sql_quality_name` are pure mapping
    /// functions; pin every enum variant so a future enum addition
    /// (or rename of the SQL spelling) is caught at compile-time via
    /// the exhaustive match. Asserting the strings here also catches
    /// a typo in the SQL-side spelling — chain authors filter on
    /// these exact strings in their trigger payloads.
    #[test]
    fn sql_height_and_quality_names_map_every_variant() {
        assert_eq!(sql_height_name(CoverHeight::Low), "HEIGHT_Low");
        assert_eq!(sql_height_name(CoverHeight::Mid), "HEIGHT_Mid");
        assert_eq!(sql_height_name(CoverHeight::High), "HEIGHT_High");
        assert_eq!(sql_height_name(CoverHeight::Los), "HEIGHT_LOS");

        assert_eq!(sql_quality_name(CoverQuality::Good), "QUALITY_Good");
        assert_eq!(sql_quality_name(CoverQuality::Better), "QUALITY_Better");
        assert_eq!(sql_quality_name(CoverQuality::Best), "QUALITY_Best");
        assert_eq!(sql_quality_name(CoverQuality::None_), "QUALITY_None");
    }
}
