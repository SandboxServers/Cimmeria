//! Tool-call audit trail.
//!
//! Every tool invocation emits exactly one `tracing` event on target
//! `lab.tool_call` with `{ tool, args, caller, outcome, duration_ms }`, plus
//! `error` (capped) when the call failed. By owner decision (issue #687) this
//! single event *is* the whole audit trail for the endpoint, so it must fire
//! once per call, on both the success and error paths. A failed call logs at
//! WARN; a successful one at INFO.

use std::time::Instant;

use serde_json::Value;

/// Outcome label for a successful call.
pub const OUTCOME_OK: &str = "ok";
/// Outcome label for a failed call (denied, bad SQL, channel down, …).
pub const OUTCOME_ERROR: &str = "error";
/// Longest `error` string kept on an audit row, in characters.
pub const ERROR_CAP: usize = 200;

/// Resolve the caller address bound by the auth middleware for this request,
/// or a placeholder if the task-local isn't set (e.g. a call path that didn't
/// pass through the middleware — should not happen in production).
fn caller() -> String {
    crate::auth::CALLER
        .try_with(|c| c.clone())
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Truncate an error message to [`ERROR_CAP`] characters.
fn cap_error(error: &str) -> String {
    error.chars().take(ERROR_CAP).collect()
}

/// Emit the one audit event for a tool call.
///
/// `args` is rendered compactly (single-line JSON) so the whole call fits on
/// one greppable log line. `error` is the failure text for an
/// [`OUTCOME_ERROR`] call and `None` otherwise; `started` is taken when the
/// tool call began, so `duration_ms` covers the whole call.
pub fn emit(tool: &str, args: &Value, outcome: &str, error: Option<&str>, started: Instant) {
    let args_compact =
        serde_json::to_string(args).unwrap_or_else(|_| "<unserializable>".to_string());
    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let error = error.map(cap_error);
    let error = error.as_deref();
    if outcome == OUTCOME_ERROR {
        tracing::warn!(
            target: "lab.tool_call",
            tool,
            args = %args_compact,
            caller = %caller(),
            outcome,
            duration_ms,
            error,
            "lab MCP tool call"
        );
    } else {
        tracing::info!(
            target: "lab.tool_call",
            tool,
            args = %args_compact,
            caller = %caller(),
            outcome,
            duration_ms,
            "lab MCP tool call"
        );
    }
}
