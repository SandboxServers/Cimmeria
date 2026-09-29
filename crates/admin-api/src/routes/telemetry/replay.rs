//! Replaying uploaded events through `tracing`, so every sink (SigNoz, the
//! on-disk logs, the admin WebSocket) receives them.
//!
//! # Where the rows land
//!
//! The client-side replays (`client.native`, `launcher.client_log`,
//! `launcher.debug_log`, `launcher.session_meta`) are routed to the
//! `cimmeria-client` SigNoz service by the server's log filters
//! (`cimmeria_server::otel::CLIENT_TARGETS`). The server's own account of
//! an upload (`launcher.ingest`, `launcher.bundle`) stays in
//! `cimmeria-server`.
//!
//! # Attributes every replayed row carries
//!
//! The OTLP bridge turns each event's own fields into log attributes (it
//! does not flatten span fields), so everything a SigNoz query should
//! filter on is a field on the event itself:
//!
//! | Field | Source |
//! |---|---|
//! | `session_id` | the token's `sid` claim |
//! | `install_id` | the token's `sub` claim |
//! | `cimmeria.session_kind` | `"lab"` or `"player"`, from the signed `kind` claim |
//! | `lab` | `true` for a lab session |
//! | `ts_ms`, `seq` | the uploader's clock and sequence number |
//!
//! A `client.native` row adds the DLL's event name (`client_target`, also
//! the log body), its level string (`client_level`), a few correlation keys
//! lifted out of the DLL's `fields` bag when present (see
//! [`LiftedFields`]), and the whole bag as JSON in `fields`.

use serde_json::{Map, Value};

use crate::routes::dev_session::TokenClaims;

use super::dto::{ClientNativeEvent, TelemetryEvent};

/// One NDJSON line that did not parse as a [`TelemetryEvent`]. The whole
/// chunk is refused at the first one, as before this module existed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayError {
    /// 1-based line number in the decompressed chunk.
    pub line: u64,
    pub err: String,
}

/// Counts from one [`replay_ndjson`] call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReplayCounts {
    /// Non-blank lines seen.
    pub parsed: u64,
    /// Lines replayed. Equal to `parsed` unless the session budget
    /// suppressed some (see `session_budget`).
    pub accepted: u64,
    /// Lines parsed but not replayed because the session was over its
    /// budget. Always 0 from [`replay_ndjson`].
    pub suppressed: u64,
}

/// Replay every event in a decompressed upload chunk (NDJSON, one
/// `TelemetryEvent` per line) under `claims`. Blank lines are skipped.
///
/// Public so the server's ingest round-trip test can drive the exact path
/// `/api/telemetry/upload-chunk` takes, past the HTTP and gzip layers.
pub fn replay_ndjson(claims: &TokenClaims, ndjson: &str) -> Result<ReplayCounts, ReplayError> {
    replay_ndjson_gated(claims, ndjson, |_| true)
}

/// [`replay_ndjson`] with a gate: an event for which `admit` returns
/// `false` is parsed and counted as suppressed but not replayed. A bad
/// line refuses the whole chunk. The whole chunk is parsed before `admit`
/// sees any event, so a refused chunk replays nothing and spends none of
/// the session's budget: its retry is judged afresh.
pub(super) fn replay_ndjson_gated(
    claims: &TokenClaims,
    ndjson: &str,
    mut admit: impl FnMut(&TelemetryEvent) -> bool,
) -> Result<ReplayCounts, ReplayError> {
    let mut events = Vec::new();
    for (idx, line) in ndjson.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let ev: TelemetryEvent = serde_json::from_str(line).map_err(|e| ReplayError {
            line: idx as u64 + 1,
            err: e.to_string(),
        })?;
        events.push(ev);
    }
    let mut counts = ReplayCounts {
        parsed: events.len() as u64,
        ..ReplayCounts::default()
    };
    for ev in events {
        if admit(&ev) {
            replay_event(claims, ev);
            counts.accepted += 1;
        } else {
            counts.suppressed += 1;
        }
    }
    Ok(counts)
}

pub(super) fn replay_event(claims: &TokenClaims, ev: TelemetryEvent) {
    // Every event carries the session's id, install and kind so SigNoz
    // queries can slice by session or by player without joining across
    // rows. Field naming matches the dev_session mint event so a session's
    // mint and uploads correlate naturally.
    let kind = claims.session_kind();
    let lab = claims.is_lab();
    match ev {
        TelemetryEvent::ClientLog(e) => {
            tracing::info!(
                target: "launcher.client_log",
                session_id = %claims.sid,
                install_id = %claims.sub,
                cimmeria.session_kind = kind,
                lab,
                ts_ms = e.ts_ms,
                seq = e.seq,
                source_file = %e.source_file,
                level = %e.level,
                category = %e.category,
                packet_no = ?e.packet_no,
                message = %e.message,
            );
        }
        TelemetryEvent::DebugLog(e) => {
            tracing::info!(
                target: "launcher.debug_log",
                session_id = %claims.sid,
                install_id = %claims.sub,
                cimmeria.session_kind = kind,
                lab,
                ts_ms = e.ts_ms,
                seq = e.seq,
                source_file = %e.source_file,
                level = %e.level,
                message = %e.message,
            );
        }
        TelemetryEvent::KeyDump(e) => {
            // KeyDump entries are encryption key material observed by
            // the client. Useful for offline pcap decryption — but we
            // intentionally do NOT log the key body at info; debug
            // only, so the default sinks don't carry it to disk, and the
            // target is `off` in every OTLP filter.
            tracing::debug!(
                target: "launcher.key_dump",
                session_id = %claims.sid,
                install_id = %claims.sub,
                ts_ms = e.ts_ms,
                seq = e.seq,
                source_file = %e.source_file,
                key_b64 = %e.key_b64,
            );
        }
        TelemetryEvent::SessionMeta(e) => {
            tracing::info!(
                target: "launcher.session_meta",
                session_id = %claims.sid,
                install_id = %claims.sub,
                cimmeria.session_kind = kind,
                lab,
                ts_ms = e.ts_ms,
                seq = e.seq,
                kind = %e.kind,
                fields = %serde_json::Value::Object(e.fields),
            );
        }
        TelemetryEvent::ClientNative(e) => {
            replay_client_native(claims, e);
        }
    }
}

/// Correlation keys lifted out of a DLL event's `fields` bag into
/// attributes of their own, so SigNoz can filter on them without parsing
/// the `fields` JSON. `None` (key absent or the wrong JSON type) omits the
/// attribute: `tracing` records nothing for a `None` field, which is the
/// right shape for "only when known" and never a sentinel.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct LiftedFields {
    /// The game account, when a hook knows it.
    pub account_id: Option<i64>,
    /// The player entity, when a hook knows it.
    pub player_id: Option<i64>,
    /// A Mercury method index (`client.dispatch.method_dropped`).
    pub method_index: Option<i64>,
    /// The map being loaded (`client.streaming.*`).
    pub level_name: Option<String>,
    /// The DLL's own build (`client.dll.attached`).
    pub dll_version: Option<String>,
    /// Whether the SGW.exe build fingerprint matched
    /// (`client.hooks.fingerprint`).
    pub fingerprint_usable: Option<bool>,
    /// The target a governor rollup summarizes (`client.telemetry.rollup`).
    pub rollup_target: Option<String>,
    /// How many events that rollup summarizes. Lifted only alongside
    /// `rollup_target`, so a generic `count` field is never mistaken for
    /// one; `sum(rollup_count)` by `rollup_target` recovers the totals.
    pub rollup_count: Option<i64>,
}

impl LiftedFields {
    pub(super) fn from_fields(fields: &Map<String, Value>) -> Self {
        let int = |k: &str| fields.get(k).and_then(Value::as_i64);
        let text = |k: &str| fields.get(k).and_then(Value::as_str).map(str::to_string);
        Self {
            account_id: int("account_id"),
            player_id: int("player_id"),
            method_index: int("method_index"),
            level_name: text("level_name"),
            dll_version: text("dll_version"),
            fingerprint_usable: fields.get("usable").and_then(Value::as_bool),
            rollup_target: text("rollup_target"),
            rollup_count: text("rollup_target").and_then(|_| int("count")),
        }
    }
}

/// Replay one injected-DLL event through `tracing`.
///
/// - **Target is static** (`client.native`), which is what routes it to
///   `cimmeria-client`. The DLL's own event name (`client.lua.pcall`)
///   rides in `client_target` and is the log body, so the SigNoz list view
///   shows it.
/// - **`level` is honoured** if it is one of `trace`/`debug`/`info`/
///   `warn`/`error`; anything else is replayed at `info` so a typo in the
///   DLL never drops an event. The raw string is kept in `client_level`.
/// - **Identity** is the token's (`session_id`, `install_id`,
///   `cimmeria.session_kind`, `lab`), never the DLL's own claims.
pub(super) fn replay_client_native(claims: &TokenClaims, e: ClientNativeEvent) {
    let lifted = LiftedFields::from_fields(&e.fields);
    let kind = claims.session_kind();
    let lab = claims.is_lab();
    let name = e.target.as_str();
    let fields_json = Value::Object(e.fields);
    match e.level.as_str() {
        "trace" => tracing::trace!(
            target: "client.native",
            session_id = %claims.sid,
            install_id = %claims.sub,
            cimmeria.session_kind = kind,
            lab,
            ts_ms = e.ts_ms,
            seq = e.seq,
            client_target = name,
            client_level = %e.level,
            account_id = lifted.account_id,
            player_id = lifted.player_id,
            method_index = lifted.method_index,
            level_name = lifted.level_name.as_deref(),
            dll_version = lifted.dll_version.as_deref(),
            fingerprint_usable = lifted.fingerprint_usable,
            rollup_target = lifted.rollup_target.as_deref(),
            rollup_count = lifted.rollup_count,
            fields = %fields_json,
            "{name}"
        ),
        "debug" => tracing::debug!(
            target: "client.native",
            session_id = %claims.sid,
            install_id = %claims.sub,
            cimmeria.session_kind = kind,
            lab,
            ts_ms = e.ts_ms,
            seq = e.seq,
            client_target = name,
            client_level = %e.level,
            account_id = lifted.account_id,
            player_id = lifted.player_id,
            method_index = lifted.method_index,
            level_name = lifted.level_name.as_deref(),
            dll_version = lifted.dll_version.as_deref(),
            fingerprint_usable = lifted.fingerprint_usable,
            rollup_target = lifted.rollup_target.as_deref(),
            rollup_count = lifted.rollup_count,
            fields = %fields_json,
            "{name}"
        ),
        "warn" => tracing::warn!(
            target: "client.native",
            session_id = %claims.sid,
            install_id = %claims.sub,
            cimmeria.session_kind = kind,
            lab,
            ts_ms = e.ts_ms,
            seq = e.seq,
            client_target = name,
            client_level = %e.level,
            account_id = lifted.account_id,
            player_id = lifted.player_id,
            method_index = lifted.method_index,
            level_name = lifted.level_name.as_deref(),
            dll_version = lifted.dll_version.as_deref(),
            fingerprint_usable = lifted.fingerprint_usable,
            rollup_target = lifted.rollup_target.as_deref(),
            rollup_count = lifted.rollup_count,
            fields = %fields_json,
            "{name}"
        ),
        "error" => tracing::error!(
            target: "client.native",
            session_id = %claims.sid,
            install_id = %claims.sub,
            cimmeria.session_kind = kind,
            lab,
            ts_ms = e.ts_ms,
            seq = e.seq,
            client_target = name,
            client_level = %e.level,
            account_id = lifted.account_id,
            player_id = lifted.player_id,
            method_index = lifted.method_index,
            level_name = lifted.level_name.as_deref(),
            dll_version = lifted.dll_version.as_deref(),
            fingerprint_usable = lifted.fingerprint_usable,
            rollup_target = lifted.rollup_target.as_deref(),
            rollup_count = lifted.rollup_count,
            fields = %fields_json,
            "{name}"
        ),
        // `info` and any unrecognised value
        _ => tracing::info!(
            target: "client.native",
            session_id = %claims.sid,
            install_id = %claims.sub,
            cimmeria.session_kind = kind,
            lab,
            ts_ms = e.ts_ms,
            seq = e.seq,
            client_target = name,
            client_level = %e.level,
            account_id = lifted.account_id,
            player_id = lifted.player_id,
            method_index = lifted.method_index,
            level_name = lifted.level_name.as_deref(),
            dll_version = lifted.dll_version.as_deref(),
            fingerprint_usable = lifted.fingerprint_usable,
            rollup_target = lifted.rollup_target.as_deref(),
            rollup_count = lifted.rollup_count,
            fields = %fields_json,
            "{name}"
        ),
    }
}
