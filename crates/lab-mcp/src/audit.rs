//! Tool-call audit trail.
//!
//! Every tool invocation emits exactly one `tracing::info!` event on target
//! `lab.tool_call` with `{ tool, args, caller, outcome }`. By owner decision
//! (issue #687) this single event *is* the whole audit trail for the endpoint,
//! so it must fire once per call, on both the success and error paths.

use serde_json::Value;

/// Outcome label for a successful call.
pub const OUTCOME_OK: &str = "ok";
/// Outcome label for a failed call (denied, bad SQL, channel down, …).
pub const OUTCOME_ERROR: &str = "error";

/// Resolve the caller address bound by the auth middleware for this request,
/// or a placeholder if the task-local isn't set (e.g. a call path that didn't
/// pass through the middleware — should not happen in production).
fn caller() -> String {
    crate::auth::CALLER
        .try_with(|c| c.clone())
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Emit the one audit event for a tool call.
///
/// `args` is rendered compactly (single-line JSON) so the whole call fits on
/// one greppable log line.
pub fn emit(tool: &str, args: &Value, outcome: &str) {
    let args_compact =
        serde_json::to_string(args).unwrap_or_else(|_| "<unserializable>".to_string());
    tracing::info!(
        target: "lab.tool_call",
        tool,
        args = %args_compact,
        caller = %caller(),
        outcome,
        "lab MCP tool call"
    );
}
