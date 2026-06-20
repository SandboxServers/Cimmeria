//! AoI / space-lifecycle dispatch arms — the deferred-buffer gate logic plus
//! the routing into the [`super::aoi`] packet emitters.
//!
//! Each function is the body of one `CellToBaseMsg` arm carved out of the
//! [`super::handle_cell_message`] match so the gate-then-emit logic can grow
//! without bloating the dispatch shell. The deferred-AoI buffering decision
//! (while a witness is pre-`onClientReady`) lives here next to the emit call
//! it gates.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;

use crate::cell::messages::{CellToBaseMsg, NpcAoIData};

use super::super::super::deferred_aoi;
use super::super::super::ConnectedClientState;
use super::{aoi, DispatchCtx};

/// Route the AoI / space-lifecycle family of `CellToBaseMsg` variants.
///
/// Called by [`super::handle_cell_message`] once the outer match has narrowed
/// `msg` to this family; any non-AoI variant is `unreachable!` by construction.
pub(super) async fn route(msg: CellToBaseMsg, ctx: &DispatchCtx<'_>) {
    match msg {
        CellToBaseMsg::SpaceData {
            space_id,
            world_name,
        } => space_data(world_name, space_id),
        CellToBaseMsg::EntityCreated {
            entity_id,
            space_id,
            position,
        } => entity_created(entity_id, space_id, position),
        CellToBaseMsg::EnteredAoI {
            witness_id,
            entity_id,
            space_id: _,
            class_id,
            position,
            direction,
            level,
            npc_data,
        } => {
            entered_aoi(
                witness_id,
                entity_id,
                class_id,
                position,
                direction,
                level,
                npc_data,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::LeftAoI {
            witness_id,
            entity_id,
        } => {
            left_aoi(
                witness_id,
                entity_id,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::EntityMoved {
            witness_id,
            entity_id,
            space_id: _,
            position,
            direction,
            velocity,
        } => {
            entity_moved(
                witness_id,
                entity_id,
                position,
                direction,
                velocity,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            entity_method_call(
                entity_id,
                method_index,
                args,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::EntityMethodCallBatch { entity_id, calls } => {
            entity_method_call_batch(
                entity_id,
                calls,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::WitnessEntityMethod {
            witness_id,
            entity_id,
            method_index,
            args,
            entity_is_player,
        } => {
            witness_entity_method(
                witness_id,
                entity_id,
                method_index,
                args,
                entity_is_player,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::EntityInvisible {
            witness_id,
            entity_id,
        } => {
            entity_invisible(
                witness_id,
                entity_id,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        other => unreachable!("aoi_dispatch::route got non-AoI variant: {other:?}"),
    }
}

/// `CellToBaseMsg::SpaceData` — register the world-name → space-id mapping.
pub(super) fn space_data(world_name: String, space_id: u32) {
    super::super::space_registry::register_space(world_name, space_id);
}

/// `CellToBaseMsg::EntityCreated` — debug-trace only; the AoI tick drives the
/// actual client-visible CREATE_ENTITY.
pub(super) fn entity_created(entity_id: u32, space_id: u32, position: [f32; 3]) {
    tracing::debug!(
        entity_id,
        space_id,
        ?position,
        "CellService: entity created"
    );
}

/// `CellToBaseMsg::EnteredAoI` — buffer the CREATE_ENTITY + cascade while the
/// witness is pre-`onClientReady`, otherwise emit immediately.
pub(super) async fn entered_aoi(
    witness_id: u32,
    entity_id: u32,
    class_id: u8,
    position: [f32; 3],
    direction: [f32; 3],
    level: u32,
    npc_data: Option<NpcAoIData>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // Gate: while the witness is pre-`onClientReady`, the client
    // hasn't loaded terrain yet — buffer the CREATE_ENTITY +
    // cascade so it fires AFTER the client is ready to ACK.
    // See `crate::base::deferred_aoi`.
    let witness_addr = entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&witness_id).copied());
    if let Some(addr) = witness_addr {
        if deferred_aoi::should_defer(connected, addr) {
            deferred_aoi::push_deferred(
                connected,
                addr,
                deferred_aoi::DeferredAoiMsg::EnteredAoI {
                    entity_id,
                    class_id,
                    position,
                    direction,
                    level,
                    npc_data,
                },
            );
            return;
        }
    } else {
        // No addr mapping for this witness yet. `entered_aoi` below will
        // call `send_to_witness_reliable`, which silently no-ops with no
        // resolvable address — the CREATE_ENTITY + cascade are lost. The
        // cell has ALREADY marked this entity in the witness set, so the
        // idempotent AoI tick will NOT re-introduce it: the entity stays
        // invisible to this witness for the whole session and only heals
        // on relog (which resets the witness set). This is the suspected
        // mechanism behind "static NPC missing until relog" — surface it
        // loudly. See docs/architecture/negative-logging-convention.md.
        tracing::warn!(
            target: "aoi.entered_no_witness_addr",
            witness_id,
            entity_id,
            class_id,
            reason = "witness_addr_unmapped",
            "EnteredAoI for unmapped witness — CREATE_ENTITY + cascade will be dropped; entity invisible to witness until relog"
        );
    }
    aoi::entered_aoi(
        witness_id,
        entity_id,
        class_id,
        position,
        direction,
        level,
        npc_data,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::LeftAoI` — buffer the LEFT event while pre-`onClientReady`
/// so it pairs with the buffered ENTERED on flush, otherwise emit.
pub(super) async fn left_aoi(
    witness_id: u32,
    entity_id: u32,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // Gate: while the witness is pre-`onClientReady`, buffer
    // the LEFT event so it pairs correctly with the buffered
    // ENTERED event on flush. If the entity enters AND leaves
    // before the client is ready, the client never sees either
    // (both buffered, both dispatched at flush).
    let witness_addr = entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&witness_id).copied());
    if let Some(addr) = witness_addr {
        if deferred_aoi::should_defer(connected, addr) {
            deferred_aoi::push_deferred(
                connected,
                addr,
                deferred_aoi::DeferredAoiMsg::LeftAoI { entity_id },
            );
            return;
        }
    }
    aoi::left_aoi(witness_id, entity_id, transport, connected, entity_to_addr).await;
}

/// `CellToBaseMsg::EntityMoved` — drop pre-`onClientReady` position updates
/// (the moving entity's CREATE is still buffered), otherwise emit.
pub(super) async fn entity_moved(
    witness_id: u32,
    entity_id: u32,
    position: [f32; 3],
    direction: [f32; 3],
    velocity: [f32; 3],
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // Position updates are unreliable and superseded by the next
    // frame. While the witness is pre-`onClientReady` the client
    // doesn't have the moving entity yet (its CREATE_ENTITY is
    // buffered), so this update would target an unknown id and
    // be dropped client-side anyway. Drop pre-ready instead of
    // buffering — saves the buffer space and matches what the
    // client will do with it. See `crate::base::deferred_aoi`
    // module docs.
    let witness_addr = entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&witness_id).copied());
    if let Some(addr) = witness_addr {
        if deferred_aoi::should_defer(connected, addr) {
            tracing::trace!(
                %addr,
                witness_id,
                entity_id,
                "Dropping EntityMoved while witness is pre-onClientReady"
            );
            return;
        }
    }
    aoi::entity_moved(
        witness_id,
        entity_id,
        position,
        direction,
        velocity,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::EntityMethodCall` — wire-log, buffer pre-`onClientReady`,
/// otherwise emit.
pub(super) async fn entity_method_call(
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // Logged before the deferred-buffer check so pre-
    // onClientReady buffered calls still appear once on the
    // wire log; deferred-replay does NOT re-emit.
    crate::wire_log::log_outbound_entity_method(entity_id, entity_id, method_index, &args);
    // Gate on the TARGET entity's session — entity-method calls
    // are dispatched to the entity_id's owning client (see
    // `aoi::entity_method_call`'s lookup). If that client is
    // still pre-`onClientReady`, buffer the method.
    let target_addr = entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&entity_id).copied());
    if let Some(addr) = target_addr {
        if deferred_aoi::should_defer(connected, addr) {
            deferred_aoi::push_deferred(
                connected,
                addr,
                deferred_aoi::DeferredAoiMsg::EntityMethodCall {
                    entity_id,
                    method_index,
                    args,
                },
            );
            return;
        }
    }
    aoi::entity_method_call(
        entity_id,
        method_index,
        args,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::EntityMethodCallBatch` — per-method wire-log, buffer each
/// call individually when pre-`onClientReady`, otherwise emit the batch.
pub(super) async fn entity_method_call_batch(
    entity_id: u32,
    calls: Vec<(u16, Vec<u8>)>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // Same wire-log + deferred-buffer pattern as EntityMethodCall,
    // applied per-method so the SigNoz `wire.out` stream sees every
    // method exactly once even when they ride the same packet.
    for (method_index, args) in &calls {
        crate::wire_log::log_outbound_entity_method(entity_id, entity_id, *method_index, args);
    }
    let target_addr = entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&entity_id).copied());
    if let Some(addr) = target_addr {
        if deferred_aoi::should_defer(connected, addr) {
            // Buffer each call individually so the existing
            // single-method flush path replays them; we lose the
            // batching benefit during deferred replay but
            // preserve the at-most-once delivery guarantee.
            for (method_index, args) in calls {
                deferred_aoi::push_deferred(
                    connected,
                    addr,
                    deferred_aoi::DeferredAoiMsg::EntityMethodCall {
                        entity_id,
                        method_index,
                        args,
                    },
                );
            }
            return;
        }
    }
    aoi::entity_method_call_batch(entity_id, calls, transport, connected, entity_to_addr).await;
}

/// `CellToBaseMsg::WitnessEntityMethod` — wire-log the observer→observee
/// fanout, then emit to the witness.
pub(super) async fn witness_entity_method(
    witness_id: u32,
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
    entity_is_player: bool,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // observer (witness_id) and observee (entity_id) differ
    // on the AoI fanout — both are recorded so SigNoz can
    // answer "what did entity X broadcast to its witnesses?"
    crate::wire_log::log_outbound_entity_method(witness_id, entity_id, method_index, &args);
    aoi::witness_entity_method(
        witness_id,
        entity_id,
        method_index,
        args,
        entity_is_player,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::EntityInvisible` — emit the invisibility packet to the
/// witness.
pub(super) async fn entity_invisible(
    witness_id: u32,
    entity_id: u32,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    aoi::entity_invisible(witness_id, entity_id, transport, connected, entity_to_addr).await;
}
