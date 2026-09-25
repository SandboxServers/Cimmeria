//! The `player_entered_cover` form of the step-activation replay.
//!
//! Cover entry is an edge exactly like `enter_region`, and it is lost the same
//! way. 2026-09-20, Castle_CellBlock mission 639: the Ambernol vial sits
//! inside the med-station desk's 5 m cover radius (set 1381), so the player
//! was in cover 1.4 s *before* picking the vial up activated step 2144. Chains
//! 1132 / 1133 are gated on that step, saw the edge, failed the gate, and —
//! because the player never left the radius — never saw another. The player
//! stood on the "take cover" marker with the drone dead and nothing happened.
//!
//! The source of truth for "edges already spent" is the detection table, not
//! the player's position. A set the table holds has had its enter edge; a set
//! the player walked into since the last 1 Hz detection tick has not, and the
//! tick will deliver that one itself, with the step already active. Replaying
//! from the table therefore covers exactly the edges that cannot recur, and
//! never races the tick into a double fire.
//!
//! Everything else follows the region replay: mission-gated chains only, so a
//! second delivery fails the gate the first one moved; containment re-checked
//! before every fire, because an earlier replayed chain may have moved the
//! player; and the caller's [`StepRegionReplayGuard`](super::StepRegionReplayGuard)
//! bounds a replayed chain that advances into another step.

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::{Chain, ChainEngine};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use crate::cell::cover::{sets_near, COVER_PROXIMITY_RADIUS};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::super::executor;
use super::super::super::mission_context::{populate_mission_context, populate_world_context};
use super::REPLAY_REASON;

/// Re-fire `player_entered_cover` for every cover set whose enter edge the
/// player has already spent and is still standing in.
pub(super) async fn replay_cover_sets(
    entity_id: u32,
    player_id: i32,
    mission_id: i32,
    step_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Cheap exits first: most worlds load no cover data, and most content has
    // no cover-entered chain to replay into.
    if space_mgr.cover.node_count() == 0
        || engine.chains_for_trigger(&TriggerType::PlayerEnteredCover) == 0
    {
        return;
    }

    // Collect first, fire second — a replayed chain can teleport or remove
    // the player. `current_sets` is already sorted by set id, which keeps the
    // firing order deterministic when two sets overlap.
    let spent: Vec<i32> = space_mgr
        .cover_detection
        .current_sets(cimmeria_common::EntityId(entity_id as i32), Instant::now())
        .into_iter()
        .map(|(set, _secs)| set)
        .collect();

    for cover_set_id in spent {
        // Re-validate against the server-known position before every fire.
        let Some(position) = space_mgr.get_entity(entity_id).map(|e| e.position) else {
            return;
        };
        let Some((height, quality)) =
            sets_near(&space_mgr.cover, &position, COVER_PROXIMITY_RADIUS)
                .get(&cover_set_id)
                .copied()
        else {
            tracing::debug!(
                entity_id,
                mission_id,
                step_id,
                cover_set_id,
                reason = "player_left_during_replay",
                "step-activation cover replay: skipping a set the player no longer occupies"
            );
            continue;
        };

        replay_one(
            entity_id,
            player_id,
            mission_id,
            step_id,
            cover_set_id,
            height.sql_name(),
            quality.sql_name(),
            engine,
            tx,
            space_mgr,
        )
        .await;
    }
}

/// Build the same context [`fire_cover_entered`](super::super::cover::fire_cover_entered)
/// builds, resolve it against mission-gated chains only, and run what matched.
#[allow(clippy::too_many_arguments)]
async fn replay_one(
    entity_id: u32,
    player_id: i32,
    mission_id: i32,
    step_id: i32,
    cover_set_id: i32,
    height: &str,
    quality: &str,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("cover_set_id".to_string(), serde_json::json!(cover_set_id));
    ctx.set_param("height".to_string(), serde_json::json!(height));
    ctx.set_param("quality".to_string(), serde_json::json!(quality));

    populate_world_context(entity_id, space_mgr, &mut ctx);
    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerEnteredCover,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event_filtered(&event, &ctx, Chain::is_mission_gated);
    if resolved.actions.is_empty() {
        tracing::debug!(
            entity_id,
            mission_id,
            step_id,
            cover_set_id,
            reason = REPLAY_REASON,
            "step-activation cover replay: no mission-gated chain matched"
        );
        return;
    }

    tracing::info!(
        entity_id,
        player_id,
        mission_id,
        step_id,
        cover_set_id,
        actions = resolved.actions.len(),
        reason = REPLAY_REASON,
        "step-activation cover replay: matched"
    );
    crate::cell::player_journal::note(
        entity_id,
        crate::cell::player_journal::kinds::COVER_REPLAY,
        format!("set={cover_set_id} mission={mission_id} step={step_id}"),
    );

    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}
