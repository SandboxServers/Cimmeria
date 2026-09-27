//! Crafting telemetry shared by every verb: the player identity each event
//! carries, and the two counters.
//!
//! Metric labels are enumerated strings only: `verb` is the cell method name
//! (`CraftVerb::method_name`), `outcome` is [`Outcome`], and `reason` is a
//! `CraftReject::reason`. Ids go on log events, never on a metric.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use crate::base::helpers::{BundleSendOutcome, WitnessSendOutcome};
use crate::base::session_identity::identity_for_entity;
use crate::base::ConnectedClientState;

/// `crafting_requests_total{verb, outcome}`: one per answered request.
pub const METRIC_REQUESTS: &str = "crafting_requests_total";
/// `crafting_rejections_total{verb, reason}`: one per refusal.
pub const METRIC_REJECTIONS: &str = "crafting_rejections_total";

/// How a crafting request was answered, the `outcome` label. Emitted
/// exactly once per request, when it is answered. What happens to an
/// accepted request later (an induction completing or failing) is not this
/// counter's business.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Carried out or queued.
    Accepted,
    /// Refused, by a rule or because the server could not decide it; the
    /// player got a line and nothing changed.
    Rejected,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Accepted => "accepted",
            Outcome::Rejected => "rejected",
        }
    }
}

/// Count one answered request.
pub fn record_request(verb: &'static str, outcome: Outcome) {
    cimmeria_observability::counter!(
        METRIC_REQUESTS,
        "verb" => verb,
        "outcome" => outcome.as_str(),
    );
}

/// Count one refusal.
pub fn record_rejection(verb: &'static str, reason: &'static str) {
    cimmeria_observability::counter!(
        METRIC_REJECTIONS,
        "verb" => verb,
        "reason" => reason,
    );
}

/// The `account_id` of the session that owns `entity_id`, for the event's
/// identity fields. `None` once the session is gone; the field is then
/// omitted rather than faked.
pub fn account_id_of(
    entity_id: u32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Option<u32> {
    identity_for_entity(connected, entity_to_addr, entity_id).account_id
}

/// Why a single-message send to the player did not go out, as the `reason`
/// of a send-failure WARN; `None` when it was sent.
pub fn witness_send_failure(outcome: &WitnessSendOutcome) -> Option<&'static str> {
    match outcome {
        WitnessSendOutcome::Sent { .. } => None,
        WitnessSendOutcome::AddrUnresolved => Some("entity_to_addr_miss"),
        WitnessSendOutcome::ClientDisconnected => Some("client_disconnected"),
        WitnessSendOutcome::SendError => Some("send_error"),
    }
}

/// [`witness_send_failure`] for a bundle send.
pub fn bundle_send_failure(outcome: &BundleSendOutcome) -> Option<&'static str> {
    match outcome {
        BundleSendOutcome::Sent { .. } => None,
        BundleSendOutcome::AddrUnresolved => Some("entity_to_addr_miss"),
        BundleSendOutcome::ClientDisconnected => Some("client_disconnected"),
        BundleSendOutcome::Empty => Some("empty_bundle"),
        BundleSendOutcome::SendError => Some("send_error"),
    }
}

/// The class of a database error, for the `error_class` field of a
/// `persist_failed` WARN (the full error goes in `error`).
pub fn sql_error_class(e: &sqlx::Error) -> &'static str {
    match e {
        sqlx::Error::RowNotFound => "row_not_found",
        sqlx::Error::Database(_) => "database",
        sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed => "pool",
        sqlx::Error::Io(_) | sqlx::Error::Tls(_) => "io",
        sqlx::Error::ColumnDecode { .. } | sqlx::Error::Decode(_) => "decode",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The label vocabulary the dashboards group by.
    #[test]
    fn outcome_labels_are_accepted_and_rejected() {
        assert_eq!(
            [Outcome::Accepted, Outcome::Rejected].map(Outcome::as_str),
            ["accepted", "rejected"]
        );
    }
}
