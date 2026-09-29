//! Governor tests. One file per concern:
//!
//! - `classify` — the must-keep table and the precedence rules.
//! - `rollup` — exact counts, rates, top-N and numeric stats.
//! - `collapse` — duplicate collapse and its repeat events.
//! - `flood` — must-keep events survive floods through the real channel.
//! - `volume` — the measured one-hour mix shrinks by at least 90% with
//!   every must-keep event intact and every event accounted for.

mod classify;
mod collapse;
mod flood;
mod rollup;
mod volume;

use std::collections::BTreeMap;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use serde_json::Value;

use super::{Governor, GovernorConfig};
use crate::events::ClientNativeEvent;

/// A stamped event, as the producer hands it to the governor.
pub(super) fn ev(
    target: &str,
    level: &str,
    ts_ms: i64,
    fields: &[(&str, Value)],
) -> ClientNativeEvent {
    ClientNativeEvent {
        ts_ms,
        seq: 0,
        target: target.to_string(),
        level: level.to_string(),
        fields: fields
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect::<BTreeMap<_, _>>(),
    }
}

/// A governor with the default config and its own sequence counter.
pub(super) fn governor() -> Governor {
    Governor::new(
        GovernorConfig::default(),
        Arc::new(AtomicU64::new(1_000_000)),
    )
}

/// Admit and return what came out.
pub(super) fn admit(g: &mut Governor, e: ClientNativeEvent) -> Vec<ClientNativeEvent> {
    let mut out = Vec::new();
    g.admit(e, &mut out);
    out
}

/// Rollup events in `out` for `target`.
pub(super) fn rollups_for<'a>(
    out: &'a [ClientNativeEvent],
    target: &'a str,
) -> impl Iterator<Item = &'a ClientNativeEvent> + 'a {
    out.iter().filter(move |e| {
        e.target == super::ROLLUP_TARGET
            && e.fields.get("rollup_target").and_then(Value::as_str) == Some(target)
    })
}
