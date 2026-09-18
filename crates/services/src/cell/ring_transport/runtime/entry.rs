//! Outward-facing ring-transport entry points: `interact()`,
//! `selectDestination()`, region-trigger crossings and the cross-world
//! deferred-load callback.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::super::dispatch::{dispatch_effect, dispatch_effects, mark_player_loaded};
use super::super::transporter::{Effect, State};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Returns true only when the player has completed the gating mission.
/// Fail-closed: an unknown entity OR an entity that hasn't accepted the
/// mission BOTH return false (the `is_some_and` short-circuits to false
/// on `None`). The call sites depend on this — a missing entity must
/// not bypass the gate. Shared between `handle_interact` and
/// `handle_select_destination` so both paths stay in lockstep if the
/// gate semantics ever expand (e.g., step-level gating).
fn mission_gate_satisfied(space_mgr: &SpaceManager, entity_id: u32, mission_id: i32) -> bool {
    space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.missions.get_mission(mission_id))
        .is_some_and(|m| m.status == cimmeria_entity::missions::MISSION_COMPLETED)
}

/// `interact()` entry point — called by the `TriggerTransporter` action
/// executor. Sets `ringSourceId` on the player and sends the destination list.
#[tracing::instrument(
    name = "ring_transport.interact",
    level = "info",
    skip_all,
    fields(region_id, entity_id)
)]
pub async fn handle_interact(
    region_id: i32,
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let required_mission_id = match space_mgr.ring_transporters.get(region_id) {
        Some(t) => t.required_mission_id,
        None => {
            tracing::warn!(
                region_id,
                entity_id,
                "TriggerTransporter: no transporter loaded for region"
            );
            return;
        }
    };

    // Mission gate (`ring_transport_regions.required_mission_id`): a no-op for
    // historical rings (NULL → None). Set per-row when a mission must be
    // completed before the destination list opens — chain-level gates are
    // belt-and-suspenders; this is the cell-runtime backstop. We treat
    // "player unknown" the same as "mission not completed" so a missing
    // entity can't bypass the gate.
    if let Some(mission_id) = required_mission_id {
        if !mission_gate_satisfied(space_mgr, entity_id, mission_id) {
            tracing::info!(
                region_id,
                entity_id,
                mission_id,
                "TriggerTransporter: mission gate not satisfied — destination list suppressed"
            );
            return;
        }
    }

    if let Some(player) = space_mgr.get_entity_mut(entity_id) {
        player.ring_source_id = Some(region_id);
    }

    let effect = match space_mgr.ring_transporters.get(region_id) {
        Some(t) => t.interact(entity_id),
        None => return,
    };
    dispatch_effect(effect, tx, space_mgr, engine).await;
}

/// `selectDestination()` — called by the `setRingTransporterDestination`
/// inbound cell method handler.
#[tracing::instrument(
    name = "ring_transport.select_destination",
    level = "info",
    skip_all,
    fields(source_region_id, destination_region_id, entity_id)
)]
pub async fn handle_select_destination(
    source_region_id: i32,
    destination_region_id: i32,
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    // Cross-world is permitted: the FSM produces `Effect::TeleportCrossWorld`
    // which the dispatcher routes through the GateTravel pipeline. Source
    // ring still resets to Idle in `warmup_timer_expired`; destination
    // advances out of `RemoteLoadWait` only after the base sends
    // `BaseToCellMsg::AdvanceRingDestination` once the player finishes
    // loading on the destination world — or, failing that, after
    // `REMOTE_LOAD_WAIT_TIMEOUT`.
    let src_required_mission_id = match space_mgr.ring_transporters.get(source_region_id) {
        Some(src) => {
            if let Err(e) = src.validate_destination(destination_region_id) {
                tracing::warn!(
                    source_region_id, destination_region_id, entity_id, error = %e,
                    "selectDestination: rejected"
                );
                return;
            }
            src.required_mission_id
        }
        None => {
            tracing::warn!(
                source_region_id,
                "selectDestination: source transporter not loaded"
            );
            return;
        }
    };
    if let Some(mission_id) = src_required_mission_id {
        if !mission_gate_satisfied(space_mgr, entity_id, mission_id) {
            tracing::info!(
                source_region_id,
                destination_region_id,
                entity_id,
                mission_id,
                "selectDestination: mission gate not satisfied — rejecting"
            );
            return;
        }
    }
    if space_mgr
        .ring_transporters
        .get(destination_region_id)
        .is_none()
    {
        tracing::warn!(
            destination_region_id,
            "selectDestination: destination transporter not loaded"
        );
        return;
    }

    let now = space_mgr.ring_transporters.now();
    if let Some(src) = space_mgr.ring_transporters.get_mut(source_region_id) {
        src.enter_send_wait(destination_region_id, entity_id, now);
    }
    {
        let dst_state = space_mgr
            .ring_transporters
            .get(destination_region_id)
            .map(|d| d.state);
        if dst_state != Some(State::Idle) {
            tracing::warn!(
                destination_region_id,
                ?dst_state,
                "selectDestination: destination busy — aborting"
            );
            // Roll the source back. `reset_to_idle` rather than writing
            // `state`/`remote_region_id` by hand: `enter_send_wait` one
            // statement ago armed the 60s SendWait stall deadline, and an
            // ad-hoc field write would leave it armed on an Idle ring, where
            // it would later abort an unrelated healthy trip.
            if let Some(src) = space_mgr.ring_transporters.get_mut(source_region_id) {
                src.reset_to_idle();
            }
            return;
        }
        if let Some(dst) = space_mgr.ring_transporters.get_mut(destination_region_id) {
            dst.remote_wait(source_region_id, now);
        }
    }

    // Python: clear `ringSourceId` and remember `destinationRingId` so the
    // destination's `playerLoaded` callback can route the player. The dest id
    // is cleared in `mark_player_loaded` once the destination ring picks the
    // player up (matching the Python `playerLoaded` lifecycle).
    if let Some(player) = space_mgr.get_entity_mut(entity_id) {
        player.ring_source_id = None;
        player.destination_ring_id = Some(destination_region_id);
    }

    let auto_start = space_mgr
        .ring_transporters
        .get(source_region_id)
        .is_some_and(|t| t.should_auto_start());
    if auto_start {
        kick_off_warmup(
            source_region_id,
            destination_region_id,
            tx,
            space_mgr,
            engine,
        )
        .await;
    }
}

/// Cross-world ring-transport deferred-load hook.
///
/// Called from `BaseToCellMsg::AdvanceRingDestination` after the player
/// finishes loading on the destination world. Same-world rings advance
/// the destination FSM synchronously inside `Effect::TeleportPlayer` —
/// cross-world has to wait for the world re-entry round-trip, so the
/// destination ring sits in `RemoteLoadWait` until this fires. Once
/// the player is recorded as loaded, `try_advance_after_load` (already
/// called by `mark_player_loaded`) walks the FSM through `RemoteWarmup
/// → Cooldown → Idle` exactly like the same-world path.
///
/// If the ring has already left `RemoteLoadWait` — because
/// `REMOTE_LOAD_WAIT_TIMEOUT` fired while the client was still loading —
/// `try_advance_after_load`'s readiness gate would silently drop this
/// arrival, and with it the `FireTeleportIn` chain event that carries
/// arrival mission credit. Release the late traveller directly instead.
pub async fn handle_remote_player_loaded(
    region_id: i32,
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let ring_state = space_mgr.ring_transporters.get(region_id).map(|t| t.state);
    if ring_state == Some(State::RemoteLoadWait) {
        mark_player_loaded(region_id, entity_id, tx, space_mgr, engine).await;
        return;
    }

    tracing::warn!(
        region_id,
        entity_id,
        state = ?ring_state,
        reason = "late_ring_arrival",
        "ring: player finished loading after the destination ring left RemoteLoadWait \
         (stall timeout, or the ring was never armed) — releasing them directly so they \
         are not left hidden and movement-locked, and firing teleport_in so arrival \
         mission credit is not lost"
    );
    if let Some(player) = space_mgr.get_entity_mut(entity_id) {
        player.destination_ring_id = None;
    }
    let effects = vec![
        Effect::ShowPlayer { entity_id },
        Effect::UnlockMovement { entity_id },
        Effect::FireTeleportIn {
            entity_id,
            region_id,
        },
    ];
    dispatch_effects(effects, tx, space_mgr, engine).await;
}

/// Hook called from the existing region-trigger path when a player crosses a
/// generic region boundary. If the region's `point_set_id` matches a known
/// ring pad, forward the enter/exit to the FSM.
pub async fn handle_region_trigger(
    point_set_id: i32,
    entering: bool,
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let region_id = match space_mgr
        .ring_point_set_to_region
        .get(&point_set_id)
        .copied()
    {
        Some(id) => id,
        None => return,
    };

    if let Some(t) = space_mgr.ring_transporters.get_mut(region_id) {
        t.region_triggered(entering, entity_id);
    }

    let auto_start = space_mgr
        .ring_transporters
        .get(region_id)
        .is_some_and(|t| t.should_auto_start());
    if auto_start {
        let dst_id = space_mgr
            .ring_transporters
            .get(region_id)
            .and_then(|t| t.remote_region_id);
        if let Some(dst_id) = dst_id {
            kick_off_warmup(region_id, dst_id, tx, space_mgr, engine).await;
        }
    }
}

/// Drive the source from SendWait → SendWarmup, the destination from RecvWait
/// → RecvWarmup, and dispatch the source's start-up effects (PlaySequence,
/// onTeleportOut, LockMovement). Shared between the destination-selection
/// path and the player-walks-on-pad path.
async fn kick_off_warmup(
    source_region_id: i32,
    destination_region_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let now = space_mgr.ring_transporters.now();
    let effects = match space_mgr.ring_transporters.get_mut(source_region_id) {
        Some(t) => t.start_sending(now),
        None => return,
    };
    // Back-pointer check, not just `state == RecvWait`. Without it a
    // destination reserved by a DIFFERENT source (whose own RecvWait has not
    // yet expired) would be dragged into this trip and would then receive
    // passengers it was not holding a slot for. Correctness here must not
    // depend on the 5s margin between SEND_WAIT_TIMEOUT and
    // RECV_WAIT_TIMEOUT.
    let dst_link = space_mgr
        .ring_transporters
        .get(destination_region_id)
        .map(|d| (d.state, d.remote_region_id));
    match dst_link {
        Some((State::RecvWait, Some(back))) if back == source_region_id => {
            if let Some(dst) = space_mgr.ring_transporters.get_mut(destination_region_id) {
                dst.remote_send(now);
            }
        }
        Some((State::RecvWait, back)) => {
            tracing::warn!(
                source_region_id,
                destination_region_id,
                destination_back_pointer = back,
                reason = "peer_backpointer_mismatch",
                "ring warmup: destination is reserved for a different source — not advancing it; \
                 this trip will release its passengers when SendWarmup's warmup deadline finds \
                 the destination unprepared"
            );
        }
        _ => {}
    }
    dispatch_effects(effects, tx, space_mgr, engine).await;
}
