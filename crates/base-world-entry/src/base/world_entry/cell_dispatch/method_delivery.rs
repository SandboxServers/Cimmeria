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

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_wire::cell::client_methods::being::{ON_EFFECT_RESULTS, ON_TIMER_UPDATE};
use cimmeria_wire::cell::client_methods::player::ON_ERROR_CODE;

use super::super::super::helpers::WitnessSendOutcome;
use super::super::super::session_identity;
use super::super::super::ConnectedClientState;
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

/// Log what became of one entity method addressed to `recipient_id`'s
/// client (`entity_id` is the entity the method is about; the same id for
/// an owner send).
pub(super) fn log_method_outcome(
    outcome: WitnessSendOutcome,
    recipient_id: u32,
    entity_id: u32,
    method_index: u16,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    match outcome {
        WitnessSendOutcome::Sent { seq, .. } => {
            if is_ability_method(method_index) {
                let who =
                    session_identity::identity_for_entity(connected, entity_to_addr, recipient_id);
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
}
