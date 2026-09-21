//! Deferred-AoI buffer flush — replays what [`deferred_aoi`] held back.
//!
//! Two windows buffer: pre-`onClientReady` world entry, and the first-login
//! cinematic hold. Both drain through [`dispatch_deferred`], which bundles
//! the entity introductions and replays the rest in encounter order via the
//! [`super::aoi`] emitters.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::channel_bundle::ChannelBundle;
use cimmeria_mercury::transport::Transport;

use crate::mercury::compose_create_entity_base_body;

use super::super::super::deferred_aoi::{self, DeferredAoiMsg};
use super::super::super::deferred_aoi_lifecycle::lifecycle_segments;
use super::super::super::helpers::{send_bundle_to_witness_reliable, BundleSendOutcome};
use super::super::super::ConnectedClientState;
use super::aoi::{entity_invisible, entity_method_call, left_aoi, witness_entity_method};
use super::player_ghost;

/// Bundle-path analogue of `aoi::log_create_emit` — emits the success
/// (`aoi.create_emit`, DEBUG) or failure (`aoi.create_send_failed`, WARN)
/// seam for the bundled introduction in [`dispatch_deferred`]. `entered` is the NPC count folded into the
/// bundle; `phase` distinguishes the phase-1 (`"create_base"`) and
/// phase-2 (`"cascade"`) bundles. Per-entity ids aren't available here —
/// the bundle carries N entities — so the seam reports the aggregate
/// count instead.
fn log_bundle_emit(
    witness_id: u32,
    entered: usize,
    phase: &'static str,
    outcome: BundleSendOutcome,
) {
    match outcome {
        BundleSendOutcome::Sent {
            base_seq,
            packets,
            bytes,
            ..
        } => {
            tracing::debug!(
                target: "aoi.create_emit",
                event = "create_emit",
                witness_id,
                entered,
                phase,
                addr_resolved = true,
                bytes,
                seq = base_seq,
                packets,
                "AoI create emit: {phase} bundle ({entered} NPC) delivered to witness"
            );
        }
        // Empty bundle is a benign no-op (the caller already gates on
        // `is_empty()`), so it never reaches here in practice; treat it as
        // non-failure to avoid a spurious WARN if that gate ever changes.
        BundleSendOutcome::Empty => {}
        failed => {
            tracing::warn!(
                target: "aoi.create_send_failed",
                event = "create_send_failed",
                witness_id,
                entered,
                phase,
                addr_resolved = failed.addr_resolved(),
                reason = failed.failure_reason().unwrap_or("unknown"),
                "AoI create emit: {phase} bundle ({entered} NPC) NOT delivered to witness -- \
                 entities may be invisible until relog"
            );
        }
    }
}

/// Drain a session's whole deferred-AoI buffer and dispatch each held
/// message through the normal AoI handlers.
///
/// Called from `handle_on_client_ready` once the client signals it's
/// ready to receive entity-state traffic, and from the cinematic hold
/// release. `trigger` names which, for the flush log line.
pub(crate) async fn flush_deferred_aoi(
    witness_id: u32,
    addr: SocketAddr,
    trigger: &'static str,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let buffered = deferred_aoi::drain_deferred(connected, addr);
    dispatch_deferred(
        witness_id,
        addr,
        trigger,
        buffered,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// Flush only the player-self method calls, leaving entity-scoped traffic
/// buffered for the cinematic hold release. `handle_on_client_ready` calls
/// this instead of [`flush_deferred_aoi`] when it starts a hold.
pub(crate) async fn flush_deferred_self_methods(
    witness_id: u32,
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let buffered = deferred_aoi::drain_deferred_self_methods(connected, addr);
    dispatch_deferred(
        witness_id,
        addr,
        "on_client_ready_self_methods",
        buffered,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// Dispatch drained deferred-AoI messages through the normal AoI handlers.
///
/// `witness_id` is the player's own entity_id (the buffer's owner — used as
/// the AoI witness for the re-dispatched `EnteredAoI` / `LeftAoI` /
/// `WitnessEntityMethod` / `EntityInvisible` events). `EntityMethodCall`
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
/// `LeftAoI`, `EntityMethodCall`, `WitnessEntityMethod` and
/// `EntityInvisible` stay on the per-message `send_to_witness_reliable` path. They preserve their **encounter
/// order** in the buffer relative to each other so the cell's intended
/// sequencing survives the flush — an `EntityMethodCall(X)` followed by
/// `LeftAoI(X)` must NOT be reordered to `LeftAoI(X)` then
/// `EntityMethodCall(X)`, or the method targets a destroyed entity.
/// The tail flushes **after** the two bundles to guarantee any
/// `LeftAoI(X)` — or held `WitnessEntityMethod(X)` — runs after the matching
/// `EnteredAoI(X)`'s cascade.
///
/// # Lifecycle segments
///
/// "Introductions first" is only correct while no entity both leaves and
/// enters inside one dispatch. [`lifecycle_segments`] cancels an enter against
/// the leave that undoes it and cuts the rest into segments that each keep
/// that property; every segment is then dispatched in the shape above. The
/// world-entry burst is introductions only, so it stays a single segment and
/// keeps its packet budget.
async fn dispatch_deferred(
    witness_id: u32,
    addr: SocketAddr,
    trigger: &'static str,
    buffered: Vec<DeferredAoiMsg>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    if buffered.is_empty() {
        return;
    }
    let buffered_count = buffered.len();
    let segments = lifecycle_segments(buffered);
    tracing::info!(
        %addr,
        witness_id,
        count = buffered_count,
        dispatched = segments.iter().map(Vec::len).sum::<usize>(),
        segments = segments.len(),
        trigger,
        "Flushing deferred-AoI buffer"
    );
    for segment in segments {
        dispatch_segment(witness_id, segment, transport, connected, entity_to_addr).await;
    }
}

/// Dispatch one lifecycle segment: its introductions as the two bundles, then
/// everything else in encounter order.
async fn dispatch_segment(
    witness_id: u32,
    buffered: Vec<DeferredAoiMsg>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
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
        WitnessEntityMethod(u32, u16, Vec<u8>, bool),
        EntityInvisible(u32),
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
                player_data,
            } => {
                phase1.append_raw_message(&compose_create_entity_base_body(
                    entity_id, class_id, position, direction,
                ));
                // Joined with the observee's session NOW, not when the cell
                // fired the event: this buffer can be seconds old and the
                // observee's cached appearance may have changed since.
                phase2.append_raw_message(&player_ghost::compose_cascade_body(
                    witness_id,
                    entity_id,
                    class_id,
                    level,
                    npc_data.as_ref(),
                    player_data.as_ref(),
                    connected,
                    entity_to_addr,
                ));
                entered_count += 1;
            }
            DeferredAoiMsg::LeftAoI { entity_id } => tail.push(TailMsg::LeftAoI(entity_id)),
            DeferredAoiMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => tail.push(TailMsg::EntityMethodCall(entity_id, method_index, args)),
            DeferredAoiMsg::WitnessEntityMethod {
                entity_id,
                method_index,
                args,
                entity_is_player,
            } => tail.push(TailMsg::WitnessEntityMethod(
                entity_id,
                method_index,
                args,
                entity_is_player,
            )),
            DeferredAoiMsg::EntityInvisible { entity_id } => {
                tail.push(TailMsg::EntityInvisible(entity_id))
            }
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
        let outcome = send_bundle_to_witness_reliable(
            transport,
            connected,
            entity_to_addr,
            witness_id,
            phase1,
        )
        .await;
        log_bundle_emit(witness_id, entered_count, "create_base", outcome);
    }
    if !phase2.is_empty() {
        tracing::debug!(
            witness_id,
            entered = entered_count,
            phase2_bytes = phase2.body_len(),
            phase2_packets = phase2.estimated_packet_count(),
            "AoI flush: phase-2 bundle (createOnClient() cascade per NPC)"
        );
        let outcome = send_bundle_to_witness_reliable(
            transport,
            connected,
            entity_to_addr,
            witness_id,
            phase2,
        )
        .await;
        log_bundle_emit(witness_id, entered_count, "cascade", outcome);
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
            TailMsg::WitnessEntityMethod(entity_id, method_index, args, entity_is_player) => {
                witness_entity_method(
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
            TailMsg::EntityInvisible(entity_id) => {
                entity_invisible(witness_id, entity_id, transport, connected, entity_to_addr).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{test_default_connected_client_state, TestTransport};

    const WITNESS: u32 = 100;
    /// A patrolling NPC the client already has when the hold begins.
    const PATROLLER: u32 = 7;

    fn entered(entity_id: u32) -> DeferredAoiMsg {
        DeferredAoiMsg::EnteredAoI {
            entity_id,
            class_id: 1,
            position: [0.0; 3],
            direction: [0.0; 3],
            level: 1,
            npc_data: None,
            player_data: None,
        }
    }

    /// Flush `buffered` on a fresh session and return what hit the wire, in
    /// send order. Every session starts at seq 0 with the same key, so the
    /// same message at the same position yields the same bytes.
    async fn flush_on_fresh_session(buffered: Vec<DeferredAoiMsg>) -> Vec<Vec<u8>> {
        let addr: SocketAddr = "127.0.0.1:54501".parse().unwrap();
        let typed_transport = Arc::new(TestTransport::new());
        let transport: Arc<dyn Transport> = typed_transport.clone();
        let mut state = test_default_connected_client_state();
        state.deferred_aoi_msgs = buffered;
        let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(WITNESS, addr)])));

        flush_deferred_aoi(
            WITNESS,
            addr,
            "cinematic_hold_release",
            &transport,
            &connected,
            &entity_to_addr,
        )
        .await;
        typed_transport.filter_to(addr)
    }

    /// The patroller walks out of range and back during a hold, so the buffer
    /// is `LeftAoI(7), EnteredAoI(7)`. Bundling introductions ahead of the
    /// tail sends the create first and the leave last, and the client ends
    /// the flush without an NPC that is standing in front of the player.
    /// Dispatching the raw buffer as one segment puts the leave packet last
    /// and fails the first assertion.
    #[tokio::test]
    async fn leave_then_reenter_reaches_the_wire_as_leave_then_create() {
        let leave_alone = flush_on_fresh_session(vec![DeferredAoiMsg::LeftAoI {
            entity_id: PATROLLER,
        }])
        .await;
        assert_eq!(leave_alone.len(), 1, "a leave is one packet");

        let sent = flush_on_fresh_session(vec![
            DeferredAoiMsg::LeftAoI {
                entity_id: PATROLLER,
            },
            entered(PATROLLER),
        ])
        .await;

        assert_eq!(
            sent.first(),
            leave_alone.first(),
            "the leave goes out first, at seq 0 — byte-identical to a lone leave"
        );
        assert_eq!(
            sent.len(),
            3,
            "then the re-introduction's phase-1 and phase-2 bundles"
        );
    }

    /// An NPC that came and went during the hold is never introduced: no
    /// create, no leave, nothing on the wire for it.
    #[tokio::test]
    async fn enter_then_leave_during_the_hold_sends_nothing() {
        let sent = flush_on_fresh_session(vec![
            entered(PATROLLER),
            DeferredAoiMsg::LeftAoI {
                entity_id: PATROLLER,
            },
        ])
        .await;
        assert!(sent.is_empty(), "{} packets sent", sent.len());
    }
}
