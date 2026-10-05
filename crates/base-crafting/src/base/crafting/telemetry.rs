//! Crafting telemetry shared by every verb: the player identity each event
//! carries, the three counters, the induction job ids and the checked
//! client sends.
//!
//! Metric labels are enumerated strings only: `verb` is the cell method name
//! (`CraftVerb::method_name`), `outcome` is [`Outcome`] or [`JobEnd`], and
//! `reason` is a `CraftReject::reason`. Ids go on log events, never on a
//! metric. Every event logs under target `crafting` with an `event` field;
//! the catalog is the `crafting` row of `docs/architecture/observability.md`.

use cimmeria_entity::known_names;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_cell_catalog::crafting::loaded_crafting_catalog;
use cimmeria_wire::cell::client_methods::being::ON_TIMER_UPDATE;

use super::session::InductionEnv;
use crate::base::helpers::{send_to_witness_reliable, BundleSendOutcome, WitnessSendOutcome};
use crate::base::session_identity::identity_for_entity;
use crate::base::ConnectedClientState;
use crate::mercury::build_player_entity_method_packet;

/// `crafting_requests_total{verb, outcome}`: one per answered request.
pub const METRIC_REQUESTS: &str = "crafting_requests_total";
/// `crafting_rejections_total{verb, reason}`: one per refusal.
pub const METRIC_REJECTIONS: &str = "crafting_rejections_total";
/// `crafting_jobs_total{verb, outcome}`: one per induction job, when it ends.
pub const METRIC_JOBS: &str = "crafting_jobs_total";

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

/// `disciplines.name` for a log line (Rule 6), from the crafting catalog
/// once something has loaded it. `None` for an absent or unknown id, and
/// before the first load.
pub fn discipline_name(discipline_id: impl Into<Option<i32>>) -> Option<String> {
    let id = discipline_id.into()?;
    let catalog = loaded_crafting_catalog()?;
    let name = catalog.disciplines.get(&id)?.name.trim();
    (!name.is_empty()).then(|| name.to_owned())
}

/// A blueprint's name for a log line (Rule 6). `blueprints` has no name
/// column: the client shows a blueprint as the item it makes, so it is
/// named for its product. `None` for an absent or unknown id, a blueprint
/// with no product, and before the catalog's first load.
pub fn blueprint_name(blueprint_id: impl Into<Option<i32>>) -> Option<String> {
    let id = blueprint_id.into()?;
    let product = loaded_crafting_catalog()?.blueprints.get(&id)?.product_id;
    cimmeria_names::owned::item(product)
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

/// Who an induction job belongs to, its id and its verb. Every job event
/// carries the ids; `verb` labels the job's refusals and its count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobIds {
    /// Correlates `queued`, `induction_started`, `induction_expired`,
    /// `completed`, `queue_dropped` and `persist_failed` of one job.
    pub job_id: u64,
    /// The job's verb, the cell method name (`CraftVerb::method_name`).
    pub verb: &'static str,
    pub account_id: u32,
    pub player_id: i32,
    pub entity_id: u32,
    /// The GM who ran the command, when the transaction is a GM grant
    /// rather than the player's own induction. Carried on the transaction
    /// and client-sync events; omitted for a player's job.
    pub gm_entity_id: Option<u32>,
    /// The GM's character name, paired with `gm_entity_id` (Rule 6).
    pub gm_name: Option<&'static str>,
}

/// How an induction job ended: the `outcome` label of
/// `crafting_jobs_total`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobEnd {
    /// The job ran and its work committed.
    Completed,
    /// The job ran and its work was refused or rolled back.
    Failed,
    /// The job was dropped before it ran.
    Dropped,
}

impl JobEnd {
    pub fn label(self) -> &'static str {
        match self {
            JobEnd::Completed => "completed",
            JobEnd::Failed => "failed",
            JobEnd::Dropped => "dropped",
        }
    }
}

/// Count one ended job. The engine calls this exactly once per job:
/// when its work returns, or when it is dropped unrun. `verb` is the
/// job's own `&'static str`, so the label set stays enumerated.
pub fn count_job(verb: &'static str, end: JobEnd) {
    cimmeria_observability::counter!(
        METRIC_JOBS,
        "verb" => verb,
        "outcome" => end.label(),
    );
}

/// The SQLSTATE of a database error (`"40P01"` for a deadlock), or `""`.
pub fn sqlstate(e: &sqlx::Error) -> String {
    match e {
        sqlx::Error::Database(db) => db.code().map(|c| c.into_owned()).unwrap_or_default(),
        _ => String::new(),
    }
}

/// Send one entity method to the player's own client and log a failed
/// send as `client_sync_failed`: WARN when the packet could not go out
/// (no address, a socket error), DEBUG when the session had already gone
/// (logout race, nothing to correct). `what` names the update.
pub async fn send_to_player(
    env: &InductionEnv,
    ids: &JobIds,
    method: u16,
    args: &[u8],
    what: &'static str,
) -> bool {
    let entity_id = ids.entity_id;
    let outcome = send_to_witness_reliable(
        &env.transport,
        &env.connected,
        &env.entity_to_addr,
        entity_id,
        |key, version, seq, acks| {
            build_player_entity_method_packet(key, seq, acks, entity_id, method, args, version)
        },
    )
    .await;
    let Some(reason) = witness_send_failure(&outcome) else {
        return true;
    };
    if matches!(outcome, WitnessSendOutcome::ClientDisconnected) {
        let player_label = known_names::player_name(ids.player_id);
        tracing::debug!(
            target: "crafting",
            event = "client_sync_failed",
            job_id = ids.job_id, // nt:id-only induction job counter, unnamed
            account_id = ids.account_id,
            account_name = known_names::account_name(ids.account_id),
            player_id = ids.player_id,
            player_name = player_label,
            gm_entity_id = ids.gm_entity_id,
            gm_entity_name = ids.gm_name,
            entity_id,
            entity_name = player_label,
            what,
            method,
            reason,
            "crafting client update dropped: the session ended mid-send"
        );
        return false;
    }
    let player_label = known_names::player_name(ids.player_id);
    tracing::warn!(
        target: "crafting",
        event = "client_sync_failed",
        job_id = ids.job_id, // nt:id-only induction job counter, unnamed
        account_id = ids.account_id,
        account_name = known_names::account_name(ids.account_id),
        player_id = ids.player_id,
        player_name = player_label,
        gm_entity_id = ids.gm_entity_id,
        gm_entity_name = ids.gm_name,
        entity_id,
        entity_name = player_label,
        what,
        method,
        reason,
        "crafting client update not sent -- the client may show stale bags or no bar"
    );
    false
}

/// [`send_to_player`] for the induction bar.
pub async fn send_timer(env: &InductionEnv, ids: &JobIds, args: &[u8]) -> bool {
    send_to_player(env, ids, ON_TIMER_UPDATE, args, "induction_timer").await
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
