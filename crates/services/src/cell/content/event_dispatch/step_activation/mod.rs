//! Step-activation edge replay (Harset H52).
//!
//! `enter_region` is an **edge** event: the client reports a volume crossing
//! once, and a chain gated on a mission step that is not yet active sees that
//! edge, fails its gate, and never gets another one. The 2026-09-18 Castle
//! playtest lost objective 2484 to exactly this — the cover edge fired about
//! one second before the step activated (report finding H9) — and the Harset
//! seed lanes have since found four more instances of the same shape.
//!
//! `player_entered_cover` is the same edge from a different source (the 1 Hz
//! cover-detection tick rather than a client hint), and H9 itself was that
//! form. [`cover_replay`] closes it; the rest of this file is the
//! `enter_region` form and the re-entrancy guard both share.
//!
//! This module closes the `enter_region` form in the engine: when a mission
//! step activates, every client-hinted region of the player's world that
//! contains the player's **server-known** position is re-fired through the
//! normal trigger path, so a chain whose gate has only just become true gets
//! the edge it missed.
//!
//! Three hazards shaped the design; each is answered structurally rather than
//! by convention.
//!
//! **Re-entrancy.** A replayed chain may itself `advance_step`, which
//! activates another step, which replays again. [`StepRegionReplayGuard`]
//! bounds that with a depth cap *and* a per-activation visited set, so a
//! two-region / two-step ping-pong terminates instead of recursing.
//!
//! **Double fire.** The client may send the real hint a moment later, which
//! would deliver the same event twice. Only chains that carry a mission /
//! step / objective condition are admitted
//! ([`Chain::is_mission_gated`](cimmeria_content_engine::chain::Chain::is_mission_gated)):
//! for those the second delivery fails the gate the first delivery moved, so
//! the double fire is a no-op by construction. An ungated chain — a bare
//! `enter_region` → `display_dialog` — is refused and logged, because
//! replaying it would show the dialog twice. That is a deliberate trade: an
//! author who wants the replay writes the mission gate they should have
//! written anyway.
//!
//! **Rings and stargates.** `triggerClientHintedGenericRegion` drives three
//! things off one hint — content chains, the ring-transporter FSM, and
//! `REGION_FLAG_STARGATE` passage. Those last two are sequenced by the
//! *dispatch arm* in `cell_methods::player::world`, not by
//! [`fire_enter_region`](super::fire_enter_region). This module calls the
//! content path only, so a replay can never start a ring transport or carry a
//! player through a gate. `replay_never_touches_rings_or_gates` pins it.

use std::collections::HashSet;

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::{Chain, ChainEngine};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{SpaceManager, REGION_FLAG_CLIENT_HINTED};
use crate::cell::spawner::is_point_in_region;

use super::super::executor;
use super::super::mission_context::{populate_mission_context, populate_world_context};

mod cover_replay;

/// The `reason` field every replay carries, in the log and in the player
/// journal. Named so an ops query and the worknote can quote the same string.
pub(crate) const REPLAY_REASON: &str = "already_inside_on_step_activation";

/// How deep a chain of step activations may replay before the guard stops it.
///
/// Four is a budget, not a modelled depth: the longest authored chain-of-steps
/// in the seed that could plausibly self-advance through regions is two, and a
/// run that reaches four is a content bug worth a WARN rather than a shape
/// worth serving.
const MAX_REPLAY_DEPTH: u32 = 4;

/// Re-entrancy bound for [`fire_step_activation_regions`].
///
/// Lives on [`SpaceManager`] because the recursion runs through
/// `executor::execute_actions`, which cannot thread a depth parameter back
/// here. A thread-local would be wrong: the cell task is `async` and tokio may
/// move it between worker threads at any `await`. The `&mut SpaceManager` the
/// whole call chain already holds *is* the exclusive token, so a plain field
/// on it is both correct and un-lockable.
#[derive(Debug, Default)]
pub(crate) struct StepRegionReplayGuard {
    depth: u32,
    /// `(entity_id, mission_id, step_id)` triples already replayed inside the
    /// current outermost activation. Cleared when `depth` returns to zero, so
    /// a later, genuine activation of the same step replays again.
    visited: HashSet<(u32, i32, i32)>,
}

impl StepRegionReplayGuard {
    /// Claim a replay slot. `false` means the caller must not replay and must
    /// not call [`Self::exit`].
    fn enter(&mut self, entity_id: u32, mission_id: i32, step_id: i32) -> Option<&'static str> {
        if self.depth >= MAX_REPLAY_DEPTH {
            return Some("replay_depth_exceeded");
        }
        if !self.visited.insert((entity_id, mission_id, step_id)) {
            return Some("step_already_replayed");
        }
        self.depth += 1;
        None
    }

    fn exit(&mut self) {
        self.depth = self.depth.saturating_sub(1);
        if self.depth == 0 {
            self.visited.clear();
        }
    }

    /// No replay in flight and nothing remembered — the state the guard must
    /// be back in after every balanced `enter`/`exit` pair.
    #[cfg(test)]
    pub(crate) fn is_idle(&self) -> bool {
        self.depth == 0 && self.visited.is_empty()
    }
}

/// Re-fire `enter_region` for every client-hinted volume the player is already
/// standing in, and `player_entered_cover` for every cover set they are already
/// in, because `step_id` of `mission_id` has just become active.
///
/// Called from the sites that own the [`ChainEngine`] and have just activated
/// a step: the executor's `accept_mission` / `advance_step` arms and the two
/// GM mission commands. The mutation is already committed when this runs, so
/// `step_status` and `mission_status` conditions read the post-activation
/// state — the whole point of the replay.
///
/// Boxed for the same reason as
/// [`fire_mission_accepted`](super::fire_mission_accepted): the resolved
/// actions run through `executor::execute_actions`, which can call back into
/// this function, and an `async fn` would compute an infinitely sized future.
pub(crate) fn fire_step_activation_regions<'a>(
    entity_id: u32,
    player_id: i32,
    mission_id: i32,
    step_id: i32,
    engine: &'a ChainEngine,
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    space_mgr: &'a mut SpaceManager,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
    Box::pin(async move {
        if let Some(refusal) = space_mgr
            .step_region_replay
            .enter(entity_id, mission_id, step_id)
        {
            tracing::warn!(
                entity_id,
                mission_id,
                step_id,
                reason = refusal,
                max_depth = MAX_REPLAY_DEPTH,
                "step-activation region replay refused — a replayed chain advanced \
                 back into this step; the chain is looping"
            );
            return;
        }

        replay_regions(
            entity_id, player_id, mission_id, step_id, engine, tx, space_mgr,
        )
        .await;
        cover_replay::replay_cover_sets(
            entity_id, player_id, mission_id, step_id, engine, tx, space_mgr,
        )
        .await;

        space_mgr.step_region_replay.exit();
    })
}

/// The body, split out so the guard's `enter`/`exit` pair brackets exactly one
/// call and cannot be unbalanced by an early return in the middle.
async fn replay_regions(
    entity_id: u32,
    player_id: i32,
    mission_id: i32,
    step_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // NPCs carry no mission state and no client to have hinted a region.
    if !space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player) {
        return;
    }
    let Some(world) = space_mgr.get_entity_world_name(entity_id) else {
        return;
    };

    // Collect first, fire second. A replayed chain can teleport the player,
    // destroy the entity or move them out of a volume, and iterating the
    // region map while that happens would borrow `space_mgr` across the
    // executor call. `regions` is a `HashMap`, so sort for a deterministic
    // firing order — otherwise which of two overlapping volumes wins a race
    // would vary run to run.
    let mut tags: Vec<String> = match containing_regions(space_mgr, entity_id, &world) {
        Some(tags) => tags,
        None => return,
    };
    tags.sort();
    tags.dedup();
    if tags.is_empty() {
        return;
    }

    tracing::debug!(
        entity_id,
        mission_id,
        step_id,
        world = %world,
        candidates = tags.len(),
        reason = REPLAY_REASON,
        "step-activation region replay: candidate volumes"
    );

    for tag in tags {
        // Re-validate before every fire: an earlier chain in this same loop
        // may have moved the player, sent them to another world, or removed
        // the entity outright.
        let still_inside = containing_regions(space_mgr, entity_id, &world)
            .is_some_and(|current| current.contains(&tag));
        if !still_inside {
            tracing::debug!(
                entity_id,
                mission_id,
                step_id,
                region_tag = %tag,
                reason = "player_left_during_replay",
                "step-activation region replay: skipping a volume the player no longer occupies"
            );
            continue;
        }

        replay_one(
            entity_id, player_id, mission_id, step_id, &tag, engine, tx, space_mgr,
        )
        .await;
    }
}

/// Tags of the client-hinted regions of `world` that contain `entity_id`'s
/// server-known position. `None` when the entity is gone or has left `world`.
///
/// Containment is [`is_point_in_region`] — H06's tolerance band, including its
/// vertical arm — and not the exact XZ test, because the question here is the
/// same one the client hint asks: "is this player standing in the volume?".
/// Using the exact test would refuse a player one unit above the floor that
/// the real hint path would have accepted.
fn containing_regions(
    space_mgr: &SpaceManager,
    entity_id: u32,
    world: &str,
) -> Option<Vec<String>> {
    if space_mgr.get_entity_world_name(entity_id).as_deref() != Some(world) {
        return None;
    }
    let entity = space_mgr.get_entity(entity_id)?;
    let position = [entity.position.x, entity.position.y, entity.position.z];
    Some(
        space_mgr
            .regions_for_world(world)
            .into_iter()
            .filter(|r| r.flags & REGION_FLAG_CLIENT_HINTED != 0)
            .filter(|r| is_point_in_region(&r.points, position))
            .map(|r| r.tag.clone())
            .collect(),
    )
}

/// Build the same `RegionEnter` context [`super::fire_enter_region`] builds,
/// resolve it against mission-gated chains only, and run what matched.
#[allow(clippy::too_many_arguments)]
async fn replay_one(
    entity_id: u32,
    player_id: i32,
    mission_id: i32,
    step_id: i32,
    region_tag: &str,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("region_key".to_string(), serde_json::json!(region_tag));

    // Load-bearing exactly as it is on the real hint path: `region_key` is a
    // bare `point_sets.name`, so a `world` condition is the only thing that
    // separates two worlds with an identically-named volume.
    populate_world_context(entity_id, space_mgr, &mut ctx);
    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
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
            region_tag,
            reason = REPLAY_REASON,
            "step-activation region replay: no mission-gated chain matched"
        );
        return;
    }

    tracing::info!(
        entity_id,
        player_id,
        mission_id,
        step_id,
        region_tag,
        actions = resolved.actions.len(),
        reason = REPLAY_REASON,
        "step-activation region replay: matched"
    );
    crate::cell::player_journal::note(
        entity_id,
        crate::cell::player_journal::kinds::REGION_REPLAY,
        format!("{region_tag} mission={mission_id} step={step_id}"),
    );

    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}

#[cfg(test)]
mod cover_replay_tests;
#[cfg(test)]
mod tests;
