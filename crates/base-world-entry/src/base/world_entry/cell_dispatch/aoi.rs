//! AoI packet emitters — the cell-side dispatcher hands these one
//! `CellToBaseMsg` per witness/entity update; each handler builds the wire
//! packet and routes it to the matching client.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::channel_bundle::{ChannelBundle, IDBASE_NPC_DEFAULT, IDBASE_SGW_PLAYER};
use cimmeria_mercury::transport::Transport;

use crate::cell::messages::{NpcAoIData, PlayerAoIData};
use crate::mercury::{
    build_avatar_update, build_create_entity_base, build_create_entity_cascade,
    build_entity_invisible, build_entity_leave, build_entity_method_packet,
    build_player_ghost_cascade,
};

use super::super::super::helpers::{
    send_bundle_to_witness_reliable, send_to_witness, send_to_witness_reliable, WitnessSendOutcome,
};
use super::super::super::session_identity::player_name_for_entity;
use super::super::super::ConnectedClientState;
use super::{method_delivery, player_ghost};

/// Where an AoI line's names come from (Rule 6). The base knows a player's
/// name from its session and an NPC's only from the `name_id` its
/// `EnteredAoI` carried; anything else is left off the line.
///
/// Every lookup runs inside a log macro, so only when that line is on, and
/// never while the caller holds the `connected` lock.
pub(super) struct AoiNames<'a> {
    pub(super) connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub(super) entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
    pub(super) npc_name_id: Option<i32>,
}

impl AoiNames<'_> {
    /// A player's character name (the witness, or a player observee).
    pub(super) fn player(&self, entity_id: u32) -> Option<&'static str> {
        player_name_for_entity(self.connected, self.entity_to_addr, entity_id)
    }

    /// An NPC observee's `name_id` text, with no session lock: for the
    /// per-event DEBUG lines (hot-path rule). A player observee is left off.
    pub(super) fn npc(&self) -> Option<&'static str> {
        self.npc_name_id
            .and_then(|n| cimmeria_entity::name_intern::intern_opt(cimmeria_names::book().text(n)))
    }

    /// The observee's name: an NPC's `name_id` text, else a player's name.
    pub(super) fn observee(&self, entity_id: u32) -> Option<&'static str> {
        match self.npc_name_id {
            Some(n) => cimmeria_entity::name_intern::intern_opt(cimmeria_names::book().text(n)),
            None => self.player(entity_id),
        }
    }
}

/// Emit the success-side (`aoi.create_emit`, DEBUG) or failure-side
/// (`aoi.create_send_failed`, WARN) observability seam for one packet of
/// the entity-introduction pair.
///
/// `phase` is `"create_base"` (CREATE_ENTITY + UPDATE_AVATAR) or
/// `"cascade"` (createOnClient property cascade). This is the visibility
/// the invisible-static-NPC (Castle_CellBlock GuardBody corpse) bug needs:
/// the entity-create/cascade packets ride Mercury and were previously
/// unlogged, and `entered_aoi`'s two reliable sends discarded their
/// outcomes — so an addr-resolution miss or a swallowed enqueue failure on
/// either packet was invisible. See
/// `docs/architecture/negative-logging-convention.md` (failure side) and
/// `docs/architecture/instrumentation-discipline.md` (success side).
fn log_create_emit(
    witness_id: u32,
    entity_id: u32,
    class_id: u8,
    phase: &'static str,
    outcome: WitnessSendOutcome,
    names: &AoiNames<'_>,
) {
    match outcome {
        WitnessSendOutcome::Sent {
            seq,
            bytes,
            packets,
            ..
        } => {
            tracing::debug!(
                target: "aoi.create_emit",
                event = "create_emit",
                witness_id, // nt:id-only per AoI event; naming it would take a session lock (hot-path rule)
                entity_id,
                entity_name = names.npc(),
                class_id,
                class_name = cimmeria_wire::names::class_name(class_id),
                phase,
                addr_resolved = true,
                // Encrypted bytes on the wire, summed over the datagrams: a
                // cascade too big for one datagram goes out as fragments.
                bytes,
                datagrams = packets,
                seq,
                "AoI create emit: {phase} packet delivered to witness"
            );
        }
        failed => {
            // reason is one of: entity_to_addr_miss / client_disconnected /
            // send_error. The helper already logged its own line; this seam
            // is the AoI-create-specific WARN that names the phase + entity
            // so the invisible-corpse repro can be pinned to CREATE vs CASCADE.
            tracing::warn!(
                target: "aoi.create_send_failed",
                event = "create_send_failed",
                witness_id,
                witness_name = names.player(witness_id),
                entity_id,
                entity_name = names.observee(entity_id),
                class_id,
                class_name = cimmeria_wire::names::class_name(class_id),
                phase,
                addr_resolved = failed.addr_resolved(),
                reason = failed.failure_reason().unwrap_or("unknown"),
                "AoI create emit: {phase} packet NOT delivered to witness -- \
                 entity may be invisible until relog"
            );
        }
    }
}

/// Failure side of the leave and invisible sends (`aoi.create_send_failed`,
/// WARN, `phase` = `"leave"` or `"invisible"`). Both are single reliable
/// packets, so a non-`Sent` outcome means the witness keeps the entity
/// visible. Success stays silent; the caller's own DEBUG row covers it.
fn log_visibility_send_failure(
    witness_id: u32,
    entity_id: u32,
    phase: &'static str,
    outcome: WitnessSendOutcome,
    names: &AoiNames<'_>,
) {
    if matches!(outcome, WitnessSendOutcome::Sent { .. }) {
        return;
    }
    tracing::warn!(
        target: "aoi.create_send_failed",
        event = "create_send_failed",
        witness_id,
        witness_name = names.player(witness_id),
        entity_id,
        entity_name = names.observee(entity_id),
        phase,
        addr_resolved = outcome.addr_resolved(),
        reason = outcome.failure_reason().unwrap_or("unknown"),
        "AoI leave or invisible packet NOT delivered to witness -- \
         entity may stay visible until relog"
    );
}

/// `CellToBaseMsg::EnteredAoI` — entity entered a witness's range.
/// Emits CREATE_ENTITY + UPDATE_AVATAR (phase 1, BaseApp immediate) followed
/// by the createOnClient() property cascade (phase 2, CellApp round-trip).
pub(super) async fn entered_aoi(
    witness_id: u32,
    entity_id: u32,
    class_id: u8,
    position: [f32; 3],
    direction: [f32; 3],
    level: u32,
    npc_data: Option<NpcAoIData>,
    player_data: Option<PlayerAoIData>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let names = AoiNames {
        connected,
        entity_to_addr,
        npc_name_id: npc_data.as_ref().and_then(|n| n.name_id),
    };
    tracing::debug!(
        witness_id, // nt:id-only per AoI event; naming it would take a session lock (hot-path rule)
        entity_id,
        entity_name = names.npc(),
        class_id,
        class_name = cimmeria_wire::names::class_name(class_id),
        level,
        is_player = player_data.is_some(),
        "AoI: entity entered witness range"
    );
    // A player observee's cascade needs the identity half that lives on its
    // base session (name, appearance, ...). Resolved before the sends so no
    // `connected` lock is held across them.
    let ghost_identity = player_data.as_ref().and_then(|_| {
        player_ghost::resolve_identity(connected, entity_to_addr, witness_id, entity_id)
    });
    // Packet 1: CREATE_ENTITY + UPDATE_AVATAR (BaseApp immediate) — RELIABLE.
    // NPC spawn into player AoI; loss = NPC permanently invisible.
    let base_outcome = send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        witness_id,
        |key, version, seq, acks| {
            build_create_entity_base(
                key, seq, acks, entity_id, class_id, position, direction, version,
            )
        },
    )
    .await;
    log_create_emit(
        witness_id,
        entity_id,
        class_id,
        "create_base",
        base_outcome,
        &names,
    );
    // Packet 2: createOnClient() property cascade (CellApp round-trip) — RELIABLE
    let cascade_outcome = send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        witness_id,
        |key, version, seq, acks| match (&player_data, &ghost_identity) {
            (Some(live), Some(identity)) => build_player_ghost_cascade(
                key,
                seq,
                acks,
                entity_id,
                &identity.cascade(live),
                version,
            ),
            _ => build_create_entity_cascade(
                key,
                seq,
                acks,
                entity_id,
                class_id,
                level,
                npc_data.as_ref(),
                version,
            ),
        },
    )
    .await;
    log_create_emit(
        witness_id,
        entity_id,
        class_id,
        "cascade",
        cascade_outcome,
        &names,
    );
}

/// `CellToBaseMsg::LeftAoI` — entity left a witness's range.
/// Emits ENTITY_INVISIBLE + LEAVE_AOI in a single packet.
pub(super) async fn left_aoi(
    witness_id: u32,
    entity_id: u32,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    tracing::debug!(
        witness_id, // nt:id-only per AoI event; naming it would take a session lock (hot-path rule)
        entity_id,  // nt:id-only per AoI event; naming it would take a session lock (hot-path rule)
        "AoI: entity left witness range"
    );
    let outcome = send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        witness_id,
        |key, version, seq, acks| build_entity_leave(key, seq, acks, entity_id, version),
    )
    .await;
    let names = AoiNames {
        connected,
        entity_to_addr,
        npc_name_id: None,
    };
    log_visibility_send_failure(witness_id, entity_id, "leave", outcome, &names);
}

/// `CellToBaseMsg::EntityMoved` — per-tick position relay for a ghost
/// entity already in the witness's AoI.
pub(super) async fn entity_moved(
    witness_id: u32,
    entity_id: u32,
    position: [f32; 3],
    direction: [f32; 3],
    velocity: [f32; 3],
    npc_moved_since_last: Option<bool>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // Full row to world_entry.log; the 1-in-N `wire.out.avatar_update`
    // sample (with the position, velocity and facing sent) to SigNoz. NA25.
    crate::firehose::log_entity_moved(
        &crate::firehose::AOI_POSITION_SAMPLER,
        &crate::firehose::EntityMovedRow {
            witness_id,
            entity_id,
            position,
            direction,
            velocity,
            npc_moved_since_last,
        },
    );
    // UNRELIABLE — avatar position updates are continuous and self-correcting;
    // the next position frame supersedes any lost one within a tick or two.
    // Stays on the no-Channel-tracking path.
    send_to_witness(
        transport,
        connected,
        entity_to_addr,
        witness_id,
        |key, version, seq, acks| {
            build_avatar_update(
                key, seq, acks, entity_id, position, velocity, direction, version,
            )
        },
    )
    .await;
}

/// `CellToBaseMsg::EntityMethodCall` — server→client entity method call to
/// the entity's owning client.
/// Batched variant of [`entity_method_call`] — packs N method calls into a
/// single Mercury packet body via [`ChannelBundle`]. All methods target the
/// same player entity; bundle is RELIABLE (matches the per-call variant's
/// channel semantics). See [`crate::cell::messages::CellToBaseMsg::EntityMethodCallBatch`]
/// for the motivating bug (world-entry region-hint flood, PR #410).
pub(super) async fn entity_method_call_batch(
    entity_id: u32,
    calls: Vec<(u16, Vec<u8>)>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    if calls.is_empty() {
        return;
    }
    tracing::debug!(
        entity_id, // nt:id-only per method batch; naming it would take a session lock (hot-path rule)
        batch_size = calls.len(),
        "CellService->client entity method call batch"
    );
    let mut bundle = ChannelBundle::new(true);
    for (method_index, args) in &calls {
        bundle.append_entity_method(*method_index, IDBASE_SGW_PLAYER, entity_id, args);
    }
    // entity_id is the routing key (the player entity owning the session)
    // AND the witness — entity-method calls are dispatched to the entity's
    // own client, identical to the per-call variant.
    let outcome =
        send_bundle_to_witness_reliable(transport, connected, entity_to_addr, entity_id, bundle)
            .await;
    method_delivery::log_batch_outcome(outcome, entity_id, &calls, connected, entity_to_addr);
}

pub(super) async fn entity_method_call(
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    tracing::debug!(
        entity_id, // nt:id-only per method call; naming it would take a session lock (hot-path rule)
        method_index,
        method_name = cimmeria_wire::names::player_client_method(method_index),
        args_len = args.len(),
        "CellService->client entity method call"
    );
    // RELIABLE — entity method calls are state-change traffic (interaction
    // triggers, quest updates, mission state, dialog opens, content engine
    // events). Loss = permanent damage.
    //
    // `entity_id` here is the entity the cell wants to address and is also
    // the routing key in `entity_to_addr` — that map only holds player
    // entries, so `entity_id` is always a SGWPlayer. Use that idbase.
    let outcome = send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, version, seq, acks| {
            build_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_index,
                IDBASE_SGW_PLAYER,
                &args,
                version,
            )
        },
    )
    .await;
    method_delivery::log_method_outcome(
        outcome,
        entity_id,
        entity_id,
        true,
        method_index,
        &args,
        connected,
        entity_to_addr,
    );
}

/// `CellToBaseMsg::WitnessEntityMethod` — broadcast a server-driven entity
/// method to a specific witness (one client per call site).
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
    tracing::debug!(
        witness_id, // nt:id-only per AoI event; naming it would take a session lock (hot-path rule)
        entity_id,  // nt:id-only per AoI event; naming it would take a session lock (hot-path rule)
        method_index,
        method_name = if entity_is_player {
            cimmeria_wire::names::player_client_method(method_index)
        } else {
            cimmeria_wire::names::any_entity_client_method(method_index)
        },
        entity_is_player,
        "Broadcast entity method to witness"
    );
    // RELIABLE — witness-broadcast entity methods are state-change traffic,
    // same shape as the owning-client `entity_method_call` above.
    //
    // `entity_id` here is the *target* (ghost) entity in the witness's AoI,
    // NOT the witness itself. The idbase selects how the method index is
    // wire-encoded: player ghosts use `IDBASE_SGW_PLAYER` (61), NPC ghosts
    // use `IDBASE_NPC_DEFAULT` (62). Method indices ≥61 encode differently
    // under each idbase — a player ghost firing a high-index method
    // (onEffectResults, onStateFieldUpdate, onSequence) corrupts on the wire
    // if forced through the NPC idbase. The cell side stamps `entity_is_player`
    // when it builds the message so this selection is correct per observee.
    let idbase = if entity_is_player {
        IDBASE_SGW_PLAYER
    } else {
        IDBASE_NPC_DEFAULT
    };
    let outcome = send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        witness_id,
        |key, version, seq, acks| {
            build_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_index,
                idbase,
                &args,
                version,
            )
        },
    )
    .await;
    method_delivery::log_method_outcome(
        outcome,
        witness_id,
        entity_id,
        entity_is_player,
        method_index,
        &args,
        connected,
        entity_to_addr,
    );
}

/// `CellToBaseMsg::EntityInvisible` — temporary visual hide that keeps the
/// entity in the client's AoI bookkeeping (used for ring-transport teleport-
/// out fades).
pub(super) async fn entity_invisible(
    witness_id: u32,
    entity_id: u32,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    tracing::debug!(
        witness_id, // nt:id-only per AoI event; naming it would take a session lock (hot-path rule)
        entity_id,  // nt:id-only per AoI event; naming it would take a session lock (hot-path rule)
        "Send ENTITY_INVISIBLE to witness"
    );
    // RELIABLE — visibility-state change. Loss leaves the entity rendered
    // when it should be hidden.
    let outcome = send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        witness_id,
        |key, version, seq, acks| build_entity_invisible(key, seq, acks, entity_id, version),
    )
    .await;
    let names = AoiNames {
        connected,
        entity_to_addr,
        npc_name_id: None,
    };
    log_visibility_send_failure(witness_id, entity_id, "invisible", outcome, &names);
}

#[cfg(test)]
#[path = "aoi_tests.rs"]
mod tests;
