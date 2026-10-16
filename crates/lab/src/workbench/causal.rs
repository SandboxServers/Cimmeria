//! Conservative eight-stage one-click trace over existing lab observations.

use serde::Serialize;
use serde_json::Value;

use crate::supervisor::events::store::StoredEvent;
use crate::workbench::clicks::ClientClick;

const MAX_WINDOW_MS: i64 = 30_000;
const MAX_EVENTS: usize = 8192;
const MERCURY_MASK: u64 = 0x0fff_ffff;

/// The eight observation boundaries in a one-click trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Stage {
    UiEvent,
    ClientSend,
    ServerReceive,
    ServerState,
    ServerSend,
    ClientReceive,
    ClientApply,
    VisibleResult,
}

impl Stage {
    const ALL: [Self; 8] = [
        Self::UiEvent,
        Self::ClientSend,
        Self::ServerReceive,
        Self::ServerState,
        Self::ServerSend,
        Self::ClientReceive,
        Self::ClientApply,
        Self::VisibleResult,
    ];
}

/// A source row with an explicit clock interval, retaining its raw fields.
#[derive(Debug, Clone, Serialize)]
pub struct Evidence {
    pub seq: u64,
    pub stage: Stage,
    pub source: String,
    pub kind: String,
    pub at_ms: i64,
    pub clock_domain: String,
    /// `None` means no defensible finite bound is known.
    pub uncertainty_ms: Option<u64>,
    pub fields: Value,
}

impl Evidence {
    fn from_client(row: &StoredEvent) -> Option<Self> {
        let stage = match row.kind.as_str() {
            "ui.click" => Stage::UiEvent,
            "net.out" | "ability.sent" | "ability.sent_seq" => Stage::ClientSend,
            "mercury.entity_method" | "ability.recv" => Stage::ClientReceive,
            "ability.applied" => Stage::ClientApply,
            "ability.shown" => Stage::VisibleResult,
            // CME is a raised game event, not proof a client handler applied it.
            "cme.event" => return None,
            _ => return None,
        };
        Some(Self {
            seq: row.seq,
            stage,
            source: "client_event_ring".into(),
            kind: row.kind.clone(),
            at_ms: row.ts_ms,
            clock_domain: if row.kind == "ui.click" {
                "host_ingest"
            } else {
                "client_wall"
            }
            .into(),
            uncertainty_ms: (row.kind != "ui.click").then_some(0),
            fields: row.fields.clone(),
        })
    }
}

/// One stage's best supported reading; ambiguity is never silently resolved.
#[derive(Debug, Clone, Serialize)]
pub struct StageResult {
    pub stage: Stage,
    pub status: &'static str,
    pub evidence: Option<Evidence>,
    pub reason: Option<&'static str>,
    pub competing_candidates: usize,
}

/// A latency interval, rather than a point estimate across uncertain clocks.
#[derive(Debug, Clone, Serialize)]
pub struct Latency {
    pub lower_ms: u64,
    pub upper_ms: u64,
    pub uncertainty_ms: u64,
}

/// Exportable evidence and explicit uncertainty for a requested click.
#[derive(Debug, Clone, Serialize)]
pub struct CausalTrace {
    pub schema_version: u32,
    pub click_id: String,
    pub stages: Vec<StageResult>,
    pub first_unobserved_stage: Option<Stage>,
    pub competing_candidates: usize,
    pub click_to_visible_ms: Option<Latency>,
    pub capture_gap: bool,
    /// Ring-order context only, never promoted to a causal stage.
    pub unjoined_context_events: Vec<Value>,
}

fn key<'a>(row: &'a Evidence, name: &str) -> Option<&'a Value> {
    row.fields
        .get(name)
        .or_else(|| {
            (name == "click_id" && row.stage == Stage::UiEvent)
                .then(|| row.fields.get("event_id"))
                .flatten()
        })
        .filter(|v| !v.is_null())
}

fn same(a: &Evidence, b: &Evidence, name: &str) -> bool {
    key(a, name).zip(key(b, name)).is_some_and(|(x, y)| x == y)
}

fn conflict(a: &Evidence, b: &Evidence, name: &str) -> bool {
    key(a, name).zip(key(b, name)).is_some_and(|(x, y)| x != y)
}

fn sequence_matches(a: &Evidence, b: &Evidence) -> bool {
    if !same(a, b, "server_session_id") {
        return false;
    }
    let Some(seq) = key(b, "mercury_seq").and_then(Value::as_u64) else {
        return false;
    };
    let Some(first) = key(a, "mercury_seq_first").and_then(Value::as_u64) else {
        return false;
    };
    let Some(last) = key(a, "mercury_seq_last").and_then(Value::as_u64) else {
        return false;
    };
    ((seq.wrapping_sub(first)) & MERCURY_MASK) <= ((last.wrapping_sub(first)) & MERCURY_MASK)
}

fn scope_proven(a: &Evidence, b: &Evidence) -> bool {
    same(a, b, "server_session_id")
        || ["client_pid", "process_created_ticks", "bridge_epoch"]
            .iter()
            .all(|field| same(a, b, field))
}

fn linked(anchor: &Evidence, candidate: &Evidence) -> bool {
    for field in [
        "client_pid",
        "server_session_id",
        "entity_id",
        "target_id",
        "click_id",
        "press_id",
        "send_id",
        "cast_id",
    ] {
        if conflict(anchor, candidate, field) {
            return false;
        }
    }
    // A click epoch/id is unique. Other IDs can reset at a process or
    // session boundary, and require independent scope proof.
    if same(anchor, candidate, "click_id") {
        return true;
    }
    if !scope_proven(anchor, candidate) {
        return false;
    }
    if ["press_id", "send_id", "cast_id"]
        .iter()
        .any(|field| same(anchor, candidate, field))
        || sequence_matches(anchor, candidate)
        || sequence_matches(candidate, anchor)
    {
        return true;
    }
    same(anchor, candidate, "method")
        && same(anchor, candidate, "arguments")
        && (same(anchor, candidate, "entity_id") || same(anchor, candidate, "server_session_id"))
}

fn plausible_time(click: &Evidence, candidate: &Evidence) -> bool {
    if click.clock_domain != candidate.clock_domain {
        // A unique click identity can cross clocks. Other identifiers can be
        // reused; without a measured clock projection they cannot establish
        // a bounded causal window across sources.
        return same(click, candidate, "click_id");
    }
    let (Some(a), Some(b)) = (click.uncertainty_ms, candidate.uncertainty_ms) else {
        return true;
    };
    let slack = i64::try_from(a.saturating_add(b)).unwrap_or(i64::MAX);
    candidate.at_ms >= click.at_ms.saturating_sub(slack)
        && candidate.at_ms
            <= click
                .at_ms
                .saturating_add(MAX_WINDOW_MS)
                .saturating_add(slack)
}

fn latency(start: &Evidence, end: &Evidence) -> Option<Latency> {
    if start.clock_domain != end.clock_domain {
        return None;
    }
    let uncertainty = start.uncertainty_ms?.saturating_add(end.uncertainty_ms?);
    let delta = end.at_ms.saturating_sub(start.at_ms);
    let uncertainty_i64 = i64::try_from(uncertainty).ok()?;
    if delta < -uncertainty_i64 {
        return None;
    }
    Some(Latency {
        lower_ms: u64::try_from(delta.saturating_sub(uncertainty_i64).max(0)).ok()?,
        upper_ms: u64::try_from(delta.saturating_add(uncertainty_i64)).ok()?,
        uncertainty_ms: uncertainty,
    })
}

/// Assemble one click from bounded client rows and any independently sourced
/// server evidence. `server` must come from a trusted, session-bound reader.
pub fn assemble(
    click_id: &str,
    client: &[StoredEvent],
    server: &[Evidence],
    capture_gap: bool,
) -> CausalTrace {
    let rows: Vec<Evidence> = client
        .iter()
        .take(MAX_EVENTS)
        .filter(|row| row.kind != "ui.click" || ClientClick::from_stored(row).is_some())
        .filter_map(Evidence::from_client)
        .chain(server.iter().take(MAX_EVENTS).cloned())
        .collect();
    let mut trace = assemble_evidence(click_id, &rows, capture_gap);
    if trace.stages[0].evidence.is_some() {
        trace.unjoined_context_events = client
            .iter()
            .filter(|row| row.kind != "ui.click")
            .take(64)
            .map(StoredEvent::to_json)
            .collect();
    }
    trace
}

/// Assemble already classified, trusted observations. This also supports
/// offline fixtures while live server capture remains unbound.
pub fn assemble_evidence(click_id: &str, evidence: &[Evidence], capture_gap: bool) -> CausalTrace {
    let mut rows: Vec<_> = evidence.iter().take(MAX_EVENTS * 2).cloned().collect();
    rows.sort_by_key(|row| row.seq);
    let clicks: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.stage == Stage::UiEvent
                && key(row, "event_id").and_then(Value::as_str) == Some(click_id)
        })
        .cloned()
        .collect();
    let mut stages = Vec::with_capacity(8);
    let click = (clicks.len() == 1).then(|| clicks[0].clone());
    stages.push(StageResult {
        stage: Stage::UiEvent,
        status: match clicks.len() {
            0 => "unknown",
            1 => "observed",
            _ => "ambiguous",
        },
        evidence: click.clone(),
        reason: match clicks.len() {
            0 => Some("click_not_in_bounded_history"),
            1 => None,
            _ => Some("duplicate_click_id"),
        },
        competing_candidates: clicks.len(),
    });
    let mut anchor = click.clone();
    for stage in Stage::ALL.into_iter().skip(1) {
        let candidates: Vec<_> = rows
            .iter()
            .filter(|row| row.stage == stage)
            .filter(|row| {
                click
                    .as_ref()
                    .is_some_and(|c| c.clock_domain != row.clock_domain || plausible_time(c, row))
            })
            .filter(|row| {
                anchor
                    .as_ref()
                    .is_some_and(|a| linked(a, row) && plausible_time(a, row))
            })
            .cloned()
            .collect();
        let count = candidates.len();
        let evidence = (count == 1).then(|| candidates[0].clone());
        let visible_ok = stage != Stage::VisibleResult
            || evidence
                .as_ref()
                .is_some_and(|row| key(row, "status").and_then(Value::as_str) == Some("ok"));
        if visible_ok {
            if let Some(row) = &evidence {
                anchor = Some(row.clone());
            }
        }
        stages.push(StageResult {
            stage,
            status: if count == 1 && visible_ok {
                "observed"
            } else if count > 1 {
                "ambiguous"
            } else {
                "unknown"
            },
            evidence,
            reason: match count {
                0 => Some(if anchor.is_some() {
                    "no_correlated_observation"
                } else {
                    "click_unbound"
                }),
                1 if !visible_ok => Some("visible_handler_failed_or_unverified"),
                1 => None,
                _ => Some("multiple_correlated_observations"),
            },
            competing_candidates: count,
        });
    }
    let visible = stages.last().and_then(|stage| {
        (stage.status == "observed")
            .then_some(stage.evidence.as_ref())
            .flatten()
    });
    let click_to_visible_ms = click.as_ref().zip(visible).and_then(|(a, b)| latency(a, b));
    let first_unobserved_stage = stages
        .iter()
        .find(|stage| stage.status != "observed")
        .map(|stage| stage.stage);
    let competing_candidates = stages
        .iter()
        .map(|stage| stage.competing_candidates)
        .max()
        .unwrap_or(0);
    CausalTrace {
        schema_version: 1,
        click_id: click_id.into(),
        stages,
        first_unobserved_stage,
        competing_candidates,
        click_to_visible_ms,
        capture_gap,
        unjoined_context_events: Vec::new(),
    }
}

#[cfg(test)]
#[path = "causal_tests.rs"]
mod tests;
