//! JSON and text rendering of leases for the tools and the logs (N6 split of
//! `lease/mod.rs`, which stays the owner of the book and its rules).

use serde_json::{json, Value};

use super::{Ended, Lease};

/// An RFC 3339 UTC timestamp, to the second, of `ms` since the epoch.
pub fn rfc3339(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_default()
}

pub(super) fn holder_text(l: &Lease, now: i64) -> String {
    format!(
        "{} ({}) since {}, expires {} (in {} s)",
        l.owner,
        l.purpose,
        rfc3339(l.acquired_ms),
        rfc3339(l.expires_ms),
        (l.expires_ms - now).max(0) / 1000
    )
}

pub(super) fn lease_json(l: &Lease, now: i64) -> Value {
    json!({
        "owner": l.owner,
        "purpose": l.purpose,
        "since": rfc3339(l.acquired_ms),
        "renewed_at": rfc3339(l.renewed_ms),
        "expires_at": rfc3339(l.expires_ms),
        "remaining_s": (l.expires_ms - now).max(0) / 1000,
        "ttl_s": l.ttl_s,
        "took_over_from": l.took_over_from.as_ref().map(ended_json),
    })
}

pub(super) fn ended_json(e: &Ended) -> Value {
    json!({
        "owner": e.owner,
        "purpose": e.purpose,
        "since": rfc3339(e.acquired_ms),
        "ended_at": rfc3339(e.ended_ms),
        "how": e.how,
        "by": e.by,
        "reason": e.reason,
    })
}

/// What [`LeaseBook::acquire`] returns to the caller: the id is the
/// capability, so it is only ever handed to the acquirer.
pub fn grant_json(l: &Lease) -> Value {
    json!({
        "lease_id": l.lease_id,
        "owner": l.owner,
        "purpose": l.purpose,
        "ttl_s": l.ttl_s,
        "expires_at": rfc3339(l.expires_ms),
        "took_over_from": l.took_over_from.as_ref().map(ended_json),
        "note": "Pass lease_id to every tool that drives the client; each such call renews the lease. lab_lease_release when you are done.",
    })
}
