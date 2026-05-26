//! AoI packet emitters — the cell-side dispatcher hands these one
//! `CellToBaseMsg` per witness/entity update; each handler builds the wire
//! packet and routes it to the matching client.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::net::UdpSocket;

use crate::cell::messages::NpcAoIData;
use crate::mercury::{
    build_avatar_update, build_create_entity_base, build_create_entity_cascade,
    build_entity_invisible, build_entity_leave, build_entity_method_packet,
};

use super::super::super::helpers::{send_to_witness, send_to_witness_reliable};
use super::super::super::ConnectedClientState;

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
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    tracing::debug!(
        witness_id,
        entity_id,
        class_id,
        level,
        "AoI: entity entered witness range"
    );
    // Packet 1: CREATE_ENTITY + UPDATE_AVATAR (BaseApp immediate) — RELIABLE.
    // NPC spawn into player AoI; loss = NPC permanently invisible.
    send_to_witness_reliable(
        socket,
        connected,
        entity_to_addr,
        witness_id,
        |key, seq, acks| {
            build_create_entity_base(key, seq, acks, entity_id, class_id, position, direction)
        },
    )
    .await;
    // Packet 2: createOnClient() property cascade (CellApp round-trip) — RELIABLE
    send_to_witness_reliable(
        socket,
        connected,
        entity_to_addr,
        witness_id,
        |key, seq, acks| {
            build_create_entity_cascade(
                key,
                seq,
                acks,
                entity_id,
                class_id,
                level,
                npc_data.as_ref(),
            )
        },
    )
    .await;
}

/// `CellToBaseMsg::LeftAoI` — entity left a witness's range.
/// Emits ENTITY_INVISIBLE + LEAVE_AOI in a single packet.
pub(super) async fn left_aoi(
    witness_id: u32,
    entity_id: u32,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    tracing::debug!(witness_id, entity_id, "AoI: entity left witness range");
    send_to_witness_reliable(
        socket,
        connected,
        entity_to_addr,
        witness_id,
        |key, seq, acks| build_entity_leave(key, seq, acks, entity_id),
    )
    .await;
}

/// `CellToBaseMsg::EntityMoved` — per-tick position relay for a ghost
/// entity already in the witness's AoI.
pub(super) async fn entity_moved(
    witness_id: u32,
    entity_id: u32,
    position: [f32; 3],
    direction: [f32; 3],
    velocity: [f32; 3],
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    tracing::trace!(witness_id, entity_id, "AoI: entity position update");
    // UNRELIABLE — avatar position updates are continuous and self-correcting;
    // the next position frame supersedes any lost one within a tick or two.
    // Stays on the no-Channel-tracking path.
    //
    // `let _ =`: the per-tick driver re-emits the next frame within ~100 ms
    // even when one update lands in an empty witness map. `send_to_witness`
    // logs the missing-addr warn internally (T1-1 / #304) so no caller-side
    // log is needed; we just discard the `Err("no_client_addr")` /
    // `Err("client_disconnected")` distinction here because both have the
    // same recovery (next tick).
    let _ = send_to_witness(
        socket,
        connected,
        entity_to_addr,
        witness_id,
        entity_id,
        "POSITION",
        |key, seq, acks| {
            build_avatar_update(key, seq, acks, entity_id, position, velocity, direction)
        },
    )
    .await;
}

/// `CellToBaseMsg::EntityMethodCall` — server→client entity method call to
/// the entity's owning client.
pub(super) async fn entity_method_call(
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    tracing::debug!(
        entity_id,
        method_index,
        args_len = args.len(),
        "CellService->client entity method call"
    );
    // RELIABLE — entity method calls are state-change traffic (interaction
    // triggers, quest updates, mission state, dialog opens, content engine
    // events). Loss = permanent damage.
    send_to_witness_reliable(
        socket,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| build_entity_method_packet(key, seq, acks, entity_id, method_index, &args),
    )
    .await;
}

/// `CellToBaseMsg::WitnessEntityMethod` — broadcast a server-driven entity
/// method to a specific witness (one client per call site).
pub(super) async fn witness_entity_method(
    witness_id: u32,
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    tracing::debug!(
        witness_id,
        entity_id,
        method_index,
        "Broadcast entity method to witness"
    );
    // RELIABLE — witness-broadcast entity methods are state-change traffic,
    // same shape as the owning-client `entity_method_call` above.
    send_to_witness_reliable(
        socket,
        connected,
        entity_to_addr,
        witness_id,
        |key, seq, acks| build_entity_method_packet(key, seq, acks, entity_id, method_index, &args),
    )
    .await;
}

/// `CellToBaseMsg::EntityInvisible` — temporary visual hide that keeps the
/// entity in the client's AoI bookkeeping (used for ring-transport teleport-
/// out fades).
pub(super) async fn entity_invisible(
    witness_id: u32,
    entity_id: u32,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    tracing::debug!(witness_id, entity_id, "Send ENTITY_INVISIBLE to witness");
    // RELIABLE — visibility-state change. Loss leaves the entity rendered
    // when it should be hidden.
    send_to_witness_reliable(
        socket,
        connected,
        entity_to_addr,
        witness_id,
        |key, seq, acks| build_entity_invisible(key, seq, acks, entity_id),
    )
    .await;
}
