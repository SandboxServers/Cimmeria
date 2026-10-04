//! The base half of the ability wire-send ledger (AB-T4): what became of an
//! entity method the cell queued.
//!
//! The cell's `abilities.wire` `wire_sent` row means the cell-to-base queue
//! accepted the message (`delivery = queued_to_base`), not that a client got
//! it. The base can still drop it here: the recipient's session ended (no
//! `entity_to_addr` entry, or gone from `connected` mid-send), or the socket
//! send failed. Each drop writes one `client_send_dropped` row naming the
//! method, the recipient and the entity the method is about, so a cast's
//! accounting shows the drop instead of a silent gap
//! (`docs/architecture/negative-logging-convention.md`).
//!
//! A teardown race is DEBUG (the helper's own `departed_witnesses` line
//! already WARNs when the miss is not a teardown); a socket error is WARN.
//! Ability methods (`onEffectResults`, `onTimerUpdate`, `onErrorCode`) also
//! log `client_sent` with the Mercury seq when they go out; that is one
//! integer match per method call, and no row at all for any other method.
//! The row carries the payload fields that join it to the cell's `wire_sent`
//! row, which names the cast (`method_join`).
//!
//! Two more paths hold or bundle methods. Before the client is ready (or
//! behind a buffered introduction) a method is buffered
//! (`client_send_buffered`, or `client_send_dropped` with
//! `deferred_buffer_full`); the flush logs `deferred_flushed` with counts,
//! and a session that ends with methods still buffered logs
//! `deferred_discarded` (`deferred_aoi::log_discarded_on_teardown`). An
//! `EntityMethodCallBatch` logs one `batch_sent` row per batch, or a
//! `client_send_dropped` row with `method = batch` and its reason.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_wire::cell::client_methods::being::{ON_EFFECT_RESULTS, ON_TIMER_UPDATE};
use cimmeria_wire::cell::client_methods::player::ON_ERROR_CODE;

use super::super::super::deferred_aoi::DeferOutcome;
use super::super::super::helpers::{BundleSendOutcome, WitnessSendOutcome};
use super::super::super::session_identity;
use super::super::super::ConnectedClientState;
use super::method_join;
use crate::wire_log::client_names::outbound_method_name;

/// The methods whose successful send is logged too (`client_sent`): the
/// ability messages a cast's accounting joins on. Kept small so routine
/// traffic (stat ticks, movement) pays nothing.
fn is_ability_method(method_index: u16) -> bool {
    matches!(
        method_index,
        ON_EFFECT_RESULTS | ON_TIMER_UPDATE | ON_ERROR_CODE
    )
}

/// Log an entity method the base held in the session's deferred buffer
/// instead of sending (`why`: `client_not_ready`, or `held_behind_create`
/// for a witness method queued behind a buffered introduction), or the drop
/// when the buffer could not take it.
pub(super) fn log_method_deferred(
    outcome: DeferOutcome,
    recipient_id: u32,
    entity_id: u32,
    method_index: u16,
    why: &'static str,
) {
    match outcome {
        DeferOutcome::Buffered { depth } => tracing::debug!(
            target: "base.entity_method",
            event = "client_send_buffered",
            method = outbound_method_name(method_index),
            method_index,
            recipient_id,
            entity_id,
            depth,
            reason = why,
            "entity method buffered until the client is ready; the flush or discard is logged"
        ),
        DeferOutcome::BufferFull => tracing::warn!(
            target: "base.entity_method",
            event = "client_send_dropped",
            method = outbound_method_name(method_index),
            method_index,
            recipient_id,
            entity_id,
            reason = "deferred_buffer_full",
            "entity method dropped at the base: the pre-ready buffer is full, so the client \
             never sees it"
        ),
        DeferOutcome::SessionGone => tracing::debug!(
            target: "base.entity_method",
            event = "client_send_dropped",
            method = outbound_method_name(method_index),
            method_index,
            recipient_id,
            entity_id,
            reason = "client_disconnected",
            "entity method dropped at the base: the recipient's session has ended"
        ),
    }
}

/// Log a flush of the deferred buffer: how many entity methods it held and
/// how many it dispatched. The difference is the methods the lifecycle pass
/// cancelled (an entity that entered and left inside the hold); each
/// dispatched one then logs its own send outcome.
pub(super) fn log_deferred_flush(
    addr: SocketAddr,
    witness_id: u32,
    trigger: &'static str,
    buffered: &(usize, String),
    dispatched: usize,
) {
    if buffered.0 == 0 {
        return;
    }
    let discarded = buffered.0.saturating_sub(dispatched);
    tracing::debug!(
        target: "base.entity_method",
        event = "deferred_flushed",
        %addr,
        recipient_id = witness_id,
        methods = buffered.0,
        method_counts = buffered.1.as_str(),
        dispatched,
        discarded,
        reason = if discarded > 0 { "entity_left_during_hold" } else { "none" },
        trigger,
        "buffered entity methods flushed to the client's send path"
    );
}

/// Log one `EntityMethodCallBatch` send: a row per batch with its method
/// counts, and the drop with its reason when the bundle did not go out.
pub(super) fn log_batch_outcome(
    outcome: BundleSendOutcome,
    entity_id: u32,
    calls: &[(u16, Vec<u8>)],
) {
    let mut counts: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
    for (m, _) in calls {
        *counts.entry(*m).or_default() += 1;
    }
    let method_counts = counts
        .iter()
        .map(|(m, n)| format!("{m}:{n}"))
        .collect::<Vec<_>>()
        .join(",");
    match outcome {
        BundleSendOutcome::Sent {
            base_seq, packets, ..
        } => tracing::debug!(
            target: "base.entity_method",
            event = "batch_sent",
            entity_id,
            calls = calls.len(),
            method_counts = method_counts.as_str(),
            base_seq,
            packets,
            "entity method batch sent to the client"
        ),
        BundleSendOutcome::SendError => tracing::warn!(
            target: "base.entity_method",
            event = "client_send_dropped",
            method = "batch",
            entity_id,
            recipient_id = entity_id,
            calls = calls.len(),
            method_counts = method_counts.as_str(),
            reason = "send_error",
            "entity method batch dropped at the base: a fragment send failed"
        ),
        failed => tracing::debug!(
            target: "base.entity_method",
            event = "client_send_dropped",
            method = "batch",
            entity_id,
            recipient_id = entity_id,
            calls = calls.len(),
            method_counts = method_counts.as_str(),
            reason = failed.failure_reason().unwrap_or("unknown"),
            "entity method batch dropped at the base: the recipient's session has ended"
        ),
    }
}

/// Log what became of one entity method addressed to `recipient_id`'s
/// client (`entity_id` is the entity the method is about; the same id for
/// an owner send). `args` is the method's payload: a `client_sent` row
/// carries the fields that join it to the cell's `wire_sent` row, and so to
/// its cast (`method_join`).
pub(super) fn log_method_outcome(
    outcome: WitnessSendOutcome,
    recipient_id: u32,
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    match outcome {
        WitnessSendOutcome::Sent { seq, .. } => {
            if is_ability_method(method_index) {
                let who =
                    session_identity::identity_for_entity(connected, entity_to_addr, recipient_id);
                let j = method_join::join_fields(method_index, args);
                tracing::debug!(
                    target: "abilities.wire",
                    event = "client_sent",
                    stage = "wire",
                    method = outbound_method_name(method_index),
                    method_index,
                    recipient_id,
                    entity_id,
                    account_id = who.account_id,
                    player_id = who.player_id,
                    seq,
                    timer_id = j.timer_id,
                    timer_type_code = j.timer_type_code,
                    secondary_id = j.secondary_id,
                    complete_at = j.complete_at,
                    ability_id = j.ability_id,
                    effect_id = j.effect_id,
                    target_id = j.target_id,
                    system_id = j.system_id,
                    instance_id = j.instance_id,
                    error_code = j.error_code,
                    "entity method sent to the client"
                );
            }
        }
        failed => {
            // The session is usually gone by now, so these are mostly absent;
            // the cell's `wire_sent` row carries the ids under the same
            // `entity_id` and `method`.
            let who =
                session_identity::identity_for_entity(connected, entity_to_addr, recipient_id);
            let reason = failed.failure_reason().unwrap_or("unknown");
            if matches!(failed, WitnessSendOutcome::SendError) {
                tracing::warn!(
                    target: "base.entity_method",
                    event = "client_send_dropped",
                    method = outbound_method_name(method_index),
                    method_index,
                    recipient_id,
                    entity_id,
                    account_id = who.account_id,
                    player_id = who.player_id,
                    reason,
                    "entity method dropped at the base: the socket send failed, so the client \
                     never sees it"
                );
            } else {
                tracing::debug!(
                    target: "base.entity_method",
                    event = "client_send_dropped",
                    method = outbound_method_name(method_index),
                    method_index,
                    recipient_id,
                    entity_id,
                    account_id = who.account_id,
                    player_id = who.player_id,
                    reason,
                    "entity method dropped at the base: the recipient's session has ended"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use cimmeria_mercury::transport::Transport;

    use super::super::aoi;
    use super::*;
    use crate::test_support::{
        test_default_connected_client_state, Captured, LogCapture, LogCaptureGuard, TestTransport,
    };

    type Connected = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;
    type EntityToAddr = Arc<Mutex<HashMap<u32, SocketAddr>>>;

    const PLAYER: u32 = 100;
    const NPC: u32 = 900;

    fn maps(with_addr: bool, with_session: bool) -> (Connected, EntityToAddr) {
        let addr: SocketAddr = "127.0.0.1:50100".parse().unwrap();
        let e2a = if with_addr {
            HashMap::from([(PLAYER, addr)])
        } else {
            HashMap::new()
        };
        let conn = if with_session {
            HashMap::from([(addr, test_default_connected_client_state())])
        } else {
            HashMap::new()
        };
        (Arc::new(Mutex::new(conn)), Arc::new(Mutex::new(e2a)))
    }

    fn drops(logs: &LogCaptureGuard) -> Vec<Captured> {
        logs.all()
            .into_iter()
            .filter(|c| {
                c.target == "base.entity_method" && c.has_field("event", "client_send_dropped")
            })
            .collect()
    }

    /// AB-T4 (Copilot on #1175): the cell queued an `onEffectResults` for a
    /// player whose session has ended (no `entity_to_addr` entry). The base
    /// drops it and says so, naming the method, recipient and reason. Fails
    /// on revert: before, only the helper's generic addr-miss line logged,
    /// with no method.
    #[tokio::test]
    async fn an_owner_method_for_a_departed_player_logs_the_drop() {
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
        let (connected, e2a) = maps(false, false);
        let logs = LogCapture::install();

        aoi::entity_method_call(
            PLAYER,
            ON_EFFECT_RESULTS,
            vec![0; 21],
            &transport,
            &connected,
            &e2a,
        )
        .await;

        let rows = drops(&logs);
        assert_eq!(rows.len(), 1, "{:#?}", logs.all());
        assert!(rows[0].has_field("method", "onEffectResults"));
        assert!(rows[0].has_field("recipient_id", "100"));
        assert!(rows[0].has_field("reason", "entity_to_addr_miss"));
    }

    /// A witness whose session left `connected` mid-send: the drop row names
    /// the witness as recipient and the NPC the method is about.
    #[tokio::test]
    async fn a_witness_method_for_a_disconnected_client_logs_the_drop() {
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
        let (connected, e2a) = maps(true, false);
        let logs = LogCapture::install();

        aoi::witness_entity_method(
            PLAYER,
            NPC,
            ON_TIMER_UPDATE,
            vec![0; 21],
            false,
            &transport,
            &connected,
            &e2a,
        )
        .await;

        let rows = drops(&logs);
        assert_eq!(rows.len(), 1, "{:#?}", logs.all());
        assert!(rows[0].has_field("recipient_id", "100"));
        assert!(rows[0].has_field("entity_id", "900"));
        assert!(rows[0].has_field("reason", "client_disconnected"));
    }

    /// A delivered ability method logs `client_sent` with its seq; any other
    /// method delivered logs nothing here.
    #[tokio::test]
    async fn a_sent_ability_method_logs_client_sent_and_others_do_not() {
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
        let (connected, e2a) = maps(true, true);
        let logs = LogCapture::install();

        aoi::entity_method_call(
            PLAYER,
            ON_ERROR_CODE,
            vec![0; 7],
            &transport,
            &connected,
            &e2a,
        )
        .await;
        aoi::entity_method_call(PLAYER, 20, vec![0; 4], &transport, &connected, &e2a).await;

        let sent: Vec<_> = logs
            .all()
            .into_iter()
            .filter(|c| c.target == "abilities.wire" && c.has_field("event", "client_sent"))
            .collect();
        assert_eq!(sent.len(), 1, "{:#?}", logs.all());
        assert!(sent[0].has_field("method", "onErrorCode"));
        assert!(sent[0].fields.contains_key("seq"));
        assert!(drops(&logs).is_empty());
    }

    /// Colo smoke test, 2026-10-04: a Heal Focus cast's two `client_sent`
    /// `onTimerUpdate` rows carried no field to join them to the cast. They
    /// now carry the timer's id, type, secondary id and expiry, the fields
    /// (and names) the cell's `wire_sent` row logs next to its `cast_id`.
    /// Fails on revert: the row had only `method` and `seq`.
    #[tokio::test]
    async fn a_sent_timer_update_carries_the_fields_that_join_it_to_its_cast() {
        use cimmeria_entity::abilities::{serialize_timer_update, TIMER_ABILITY_WARMUP};
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
        let (connected, e2a) = maps(true, true);
        let logs = LogCapture::install();

        let args = serialize_timer_update(597, TIMER_ABILITY_WARMUP, PLAYER as i32, 0, 2.0, 88.5);
        aoi::entity_method_call(PLAYER, ON_TIMER_UPDATE, args, &transport, &connected, &e2a).await;

        let sent: Vec<_> = logs
            .all()
            .into_iter()
            .filter(|c| c.target == "abilities.wire" && c.has_field("event", "client_sent"))
            .collect();
        assert_eq!(sent.len(), 1, "{:#?}", logs.all());
        let row = &sent[0];
        assert!(row.has_field("timer_id", "597"), "{row:#?}");
        assert!(
            row.has_field("timer_type_code", &TIMER_ABILITY_WARMUP.to_string()),
            "{row:#?}"
        );
        assert!(row.has_field("secondary_id", "0"), "{row:#?}");
        assert!(row.has_field("complete_at", "88.5"), "{row:#?}");
        assert!(!row.fields.contains_key("effect_id"), "{row:#?}");
    }

    fn rows_with(logs: &LogCaptureGuard, event: &str) -> Vec<Captured> {
        logs.all()
            .into_iter()
            .filter(|c| c.target == "base.entity_method" && c.has_field("event", event))
            .collect()
    }

    /// A method for a client still loading is buffered (`client_send_buffered`,
    /// `client_not_ready`), and the `onClientReady` flush reports it
    /// (`deferred_flushed`, one method buffered and dispatched). Fails on
    /// revert of either row: the buffer used to be silent.
    #[tokio::test]
    async fn a_method_buffered_before_ready_logs_the_buffer_and_the_flush() {
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
        let (addr, connected, e2a) = super::super::tests_dispatch_arms::one_session(PLAYER, true);
        let logs = LogCapture::install();

        super::super::aoi_dispatch::entity_method_call(
            PLAYER,
            ON_EFFECT_RESULTS,
            vec![0; 21],
            &transport,
            &connected,
            &e2a,
        )
        .await;
        let buffered = rows_with(&logs, "client_send_buffered");
        assert_eq!(buffered.len(), 1, "{:#?}", logs.all());
        assert!(buffered[0].has_field("method", "onEffectResults"));
        assert!(buffered[0].has_field("reason", "client_not_ready"));

        connected
            .lock()
            .unwrap()
            .get_mut(&addr)
            .unwrap()
            .pending_client_ready = None;
        super::super::deferred_flush::flush_deferred_aoi(
            PLAYER, addr, "test", &transport, &connected, &e2a,
        )
        .await;
        let flushed = rows_with(&logs, "deferred_flushed");
        assert_eq!(flushed.len(), 1, "{:#?}", logs.all());
        assert!(flushed[0].has_field("methods", "1"));
        assert!(flushed[0].has_field("dispatched", "1"));
        assert!(flushed[0].has_field("method_counts", "14:1"));
    }

    /// The buffer at its cap drops the method with a WARN naming it.
    #[tokio::test]
    async fn a_method_refused_by_a_full_buffer_logs_the_drop() {
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
        let (addr, connected, e2a) = super::super::tests_dispatch_arms::one_session(PLAYER, true);
        while deferred_aoi_push_left(&connected, addr) {}
        let logs = LogCapture::install();

        super::super::aoi_dispatch::entity_method_call(
            PLAYER,
            ON_TIMER_UPDATE,
            vec![0; 21],
            &transport,
            &connected,
            &e2a,
        )
        .await;

        let rows = drops(&logs);
        assert_eq!(rows.len(), 1, "{:#?}", logs.all());
        assert_eq!(rows[0].level, tracing::Level::WARN);
        assert!(rows[0].has_field("reason", "deferred_buffer_full"));
        assert!(rows[0].has_field("method", "onTimerUpdate"));
    }

    /// Fill the buffer with leaves; `false` once it refuses.
    fn deferred_aoi_push_left(connected: &Connected, addr: SocketAddr) -> bool {
        use super::super::super::super::deferred_aoi::{push_deferred, DeferredAoiMsg};
        matches!(
            push_deferred(connected, addr, DeferredAoiMsg::LeftAoI { entity_id: 1 }),
            DeferOutcome::Buffered { .. }
        )
    }

    /// An `EntityMethodCallBatch` logs one row per batch with its counts, and
    /// a drop row with its reason when the recipient is gone.
    #[tokio::test]
    async fn a_batch_logs_one_row_sent_or_dropped() {
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
        let calls = || vec![(ON_ERROR_CODE, vec![0; 7]), (ON_ERROR_CODE, vec![0; 7])];
        let logs = LogCapture::install();

        let (connected, e2a) = maps(true, true);
        aoi::entity_method_call_batch(PLAYER, calls(), &transport, &connected, &e2a).await;
        let (gone_c, gone_e) = maps(false, false);
        aoi::entity_method_call_batch(PLAYER, calls(), &transport, &gone_c, &gone_e).await;

        let sent = rows_with(&logs, "batch_sent");
        assert_eq!(sent.len(), 1, "{:#?}", logs.all());
        assert!(sent[0].has_field("calls", "2"));
        assert!(sent[0].has_field("method_counts", "121:2"));
        let dropped = drops(&logs);
        assert_eq!(dropped.len(), 1, "{:#?}", logs.all());
        assert!(dropped[0].has_field("method", "batch"));
        assert!(dropped[0].has_field("reason", "entity_to_addr_miss"));
    }
}
