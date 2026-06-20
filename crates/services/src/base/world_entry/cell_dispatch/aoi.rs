//! AoI packet emitters — the cell-side dispatcher hands these one
//! `CellToBaseMsg` per witness/entity update; each handler builds the wire
//! packet and routes it to the matching client.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::channel_bundle::{ChannelBundle, IDBASE_NPC_DEFAULT, IDBASE_SGW_PLAYER};
use cimmeria_mercury::transport::Transport;

use crate::cell::messages::NpcAoIData;
use crate::mercury::{
    build_avatar_update, build_create_entity_base, build_create_entity_cascade,
    build_entity_invisible, build_entity_leave, build_entity_method_packet,
    compose_create_entity_base_body, compose_create_entity_cascade_body,
};

use super::super::super::deferred_aoi::{self, DeferredAoiMsg};
use super::super::super::helpers::{
    send_bundle_to_witness_reliable, send_to_witness, send_to_witness_reliable,
};
use super::super::super::ConnectedClientState;

/// Drain a session's deferred-AoI buffer and dispatch each held message
/// through the normal AoI handlers.
///
/// Called from `handle_on_client_ready` once the client signals it's
/// ready to receive entity-state traffic. `witness_id` is the player's
/// own entity_id (the buffer's owner — used as the AoI witness for the
/// re-dispatched `EnteredAoI` / `LeftAoI` events). `EntityMethodCall`
/// entries carry their own target entity_id.
///
/// # Bundle batching
///
/// `EnteredAoI` is the burst-shape failure mode — a Castle_CellBlock
/// instance with 28 NPCs would pre-bundle emit 56 reliable packets
/// (2 per NPC: CREATE_ENTITY/UPDATE_AVATAR pair, then cascade).
/// Instead this function bundles into TWO logical client frames:
///
/// - **Phase-1 bundle**: every NPC's `CREATE_ENTITY + UPDATE_AVATAR`
///   body. Safe to combine cross-entity: CREATE_ENTITY(A) puts entity A
///   in transaction for the bundle, but CREATE_ENTITY(B) targets a
///   different entity and is unaffected.
/// - **Phase-2 bundle**: every NPC's `createOnClient()` property cascade
///   body. Safe to combine cross-entity for the same reason. Critically
///   sent as a SEPARATE bundle from phase-1 — same-entity messages
///   after CREATE_ENTITY in the same bundle hit the client's
///   HOLD-FOR-TRANSACTION path and are silently dropped (see the
///   `cimmeria_mercury::channel_bundle` module doc and the deliberate
///   two-bundle split in `base/world_entry/map_loaded.rs`).
///
/// For 28 NPCs: phase-1 ≈ 28×37 = 1 KB (1 fragment under
/// `FRAGMENT_BODY_SIZE`=1300); phase-2 ≈ 28×442 = 12 KB (10 fragments
/// at 1300 bytes each) ≈ **~11 reliable packets total**, down from 56.
/// The regression guard at
/// [`flush_deferred_aoi_bundles_28_npc_burst_under_packet_budget`]
/// pins this at `≤15` to leave headroom for cascade-payload growth.
/// The deferred-send queue still backstops the case where bundle
/// finalize emits more than the remaining TX-window capacity.
///
/// `LeftAoI` and `EntityMethodCall` stay on the per-message
/// `send_to_witness_reliable` path. They preserve their **encounter
/// order** in the buffer relative to each other so the cell's intended
/// sequencing survives the flush — an `EntityMethodCall(X)` followed by
/// `LeftAoI(X)` must NOT be reordered to `LeftAoI(X)` then
/// `EntityMethodCall(X)`, or the method targets a destroyed entity.
/// Both classes flush **after** the two bundles to guarantee any
/// `LeftAoI(X)` runs after the matching `EnteredAoI(X)`'s cascade.
pub(crate) async fn flush_deferred_aoi(
    witness_id: u32,
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let buffered = deferred_aoi::drain_deferred(connected, addr);
    if buffered.is_empty() {
        return;
    }
    tracing::info!(
        %addr,
        witness_id,
        count = buffered.len(),
        "Flushing deferred-AoI buffer after onClientReady"
    );

    // Pre-aggregate EnteredAoI events into two cross-entity bundles.
    // Tracked separately so phase-1 emits before phase-2 (the client must
    // process each NPC's CREATE_ENTITY transaction before its cascade).
    let mut phase1 = ChannelBundle::new(true);
    let mut phase2 = ChannelBundle::new(true);
    let mut entered_count = 0usize;

    // Per-message tail — LeftAoI / EntityMethodCall keep their existing
    // one-packet-each shape and MUST preserve encounter order relative to
    // each other (a buffered EntityMethodCall(X) followed by LeftAoI(X)
    // would otherwise reorder to LeftAoI(X) then EntityMethodCall(X) and
    // the method would target a destroyed entity). Single enum + push in
    // iteration order is what holds this invariant.
    enum TailMsg {
        LeftAoI(u32),
        EntityMethodCall(u32, u16, Vec<u8>),
    }
    let mut tail: Vec<TailMsg> = Vec::new();

    for msg in buffered {
        match msg {
            DeferredAoiMsg::EnteredAoI {
                entity_id,
                class_id,
                position,
                direction,
                level,
                npc_data,
            } => {
                phase1.append_raw_message(&compose_create_entity_base_body(
                    entity_id, class_id, position, direction,
                ));
                phase2.append_raw_message(&compose_create_entity_cascade_body(
                    entity_id,
                    class_id,
                    level,
                    npc_data.as_ref(),
                ));
                entered_count += 1;
            }
            DeferredAoiMsg::LeftAoI { entity_id } => tail.push(TailMsg::LeftAoI(entity_id)),
            DeferredAoiMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => tail.push(TailMsg::EntityMethodCall(entity_id, method_index, args)),
        }
    }

    if !phase1.is_empty() {
        tracing::debug!(
            witness_id,
            entered = entered_count,
            phase1_bytes = phase1.body_len(),
            phase1_packets = phase1.estimated_packet_count(),
            "AoI flush: phase-1 bundle (CREATE_ENTITY + UPDATE_AVATAR per NPC)"
        );
        send_bundle_to_witness_reliable(transport, connected, entity_to_addr, witness_id, phase1)
            .await;
    }
    if !phase2.is_empty() {
        tracing::debug!(
            witness_id,
            entered = entered_count,
            phase2_bytes = phase2.body_len(),
            phase2_packets = phase2.estimated_packet_count(),
            "AoI flush: phase-2 bundle (createOnClient() cascade per NPC)"
        );
        send_bundle_to_witness_reliable(transport, connected, entity_to_addr, witness_id, phase2)
            .await;
    }

    for msg in tail {
        match msg {
            TailMsg::LeftAoI(entity_id) => {
                left_aoi(witness_id, entity_id, transport, connected, entity_to_addr).await;
            }
            TailMsg::EntityMethodCall(entity_id, method_index, args) => {
                entity_method_call(
                    entity_id,
                    method_index,
                    args,
                    transport,
                    connected,
                    entity_to_addr,
                )
                .await;
            }
        }
    }
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
    transport: &Arc<dyn Transport>,
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
    // Packet 2: createOnClient() property cascade (CellApp round-trip) — RELIABLE
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        witness_id,
        |key, version, seq, acks| {
            build_create_entity_cascade(
                key,
                seq,
                acks,
                entity_id,
                class_id,
                level,
                npc_data.as_ref(),
                version,
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
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    tracing::debug!(witness_id, entity_id, "AoI: entity left witness range");
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        witness_id,
        |key, version, seq, acks| build_entity_leave(key, seq, acks, entity_id, version),
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
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    tracing::trace!(witness_id, entity_id, "AoI: entity position update");
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
        entity_id,
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
    send_bundle_to_witness_reliable(transport, connected, entity_to_addr, entity_id, bundle).await;
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
        entity_id,
        method_index,
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
    send_to_witness_reliable(
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
        witness_id,
        entity_id,
        method_index,
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
    send_to_witness_reliable(
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
    tracing::debug!(witness_id, entity_id, "Send ENTITY_INVISIBLE to witness");
    // RELIABLE — visibility-state change. Loss leaves the entity rendered
    // when it should be hidden.
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        witness_id,
        |key, version, seq, acks| build_entity_invisible(key, seq, acks, entity_id, version),
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{test_default_connected_client_state, TestTransport};

    /// Domain A (fan-out byte test): one AoI event (entity 200 leaves) routed
    /// to two witnesses lands as exactly two packets — one per witness — each
    /// addressed to that witness's `entity_to_addr` mapping, with byte-exact
    /// (and, since the witnesses share default key+seq state, identical)
    /// payloads.
    ///
    /// This is the witness_id → addr → bytes mapping that was uninspectable
    /// before the `Transport` trait: it catches witness-list amplification
    /// (N±1 sends) via the `len`/`send_count_to` assertions, and wrong-
    /// recipient routes via the per-index addr assertions.
    #[tokio::test]
    async fn left_aoi_fans_out_one_packet_per_witness_to_each_addr() {
        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();

        let entity_id = 200u32; // the entity leaving AoI
        let witness_a = 100u32;
        let witness_b = 101u32;
        let addr_a: SocketAddr = "127.0.0.1:50100".parse().unwrap();
        let addr_b: SocketAddr = "127.0.0.1:50101".parse().unwrap();

        let entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>> =
            Arc::new(Mutex::new(HashMap::from([
                (witness_a, addr_a),
                (witness_b, addr_b),
            ])));
        let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
            Arc::new(Mutex::new(HashMap::from([
                (addr_a, test_default_connected_client_state()),
                (addr_b, test_default_connected_client_state()),
            ])));

        // Cell fans out one LeftAoI per witness; the base routes each.
        left_aoi(
            witness_a,
            entity_id,
            &dyn_transport,
            &connected,
            &entity_to_addr,
        )
        .await;
        left_aoi(
            witness_b,
            entity_id,
            &dyn_transport,
            &connected,
            &entity_to_addr,
        )
        .await;

        let sent = transport.drain();
        assert_eq!(
            sent.len(),
            2,
            "exactly one send per witness, no amplification"
        );
        assert_eq!(
            transport.send_count_to(addr_a),
            0,
            "drain consumed the records"
        );
        assert_eq!(sent[0].0, addr_a, "witness A's packet routed to A's addr");
        assert_eq!(sent[1].0, addr_b, "witness B's packet routed to B's addr");

        // Both witnesses start at next_seq=0 with the all-zero key, so the
        // leave packet is byte-identical for each.
        let expected = build_entity_leave(
            &[0u8; 32],
            0,
            &[],
            entity_id,
            cimmeria_mercury::encryption::EncryptionVersion::V1,
        );
        assert_eq!(sent[0].1, expected, "witness A leave bytes (seq 0)");
        assert_eq!(sent[1].1, expected, "witness B leave bytes (seq 0)");
    }
}
