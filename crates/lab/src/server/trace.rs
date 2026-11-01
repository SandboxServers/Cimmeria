//! The line-trace tool (`client_trace_line`, C14): one engine collision check
//! in the client. The DLL enforces the same flag allowlist; this tool refuses
//! early so a bad flag never reaches the bridge.

use rmcp::{
    handler::server::wrapper::Parameters, model::*, schemars, tool, tool_router,
    ErrorData as McpError,
};
use serde_json::json;

use super::LabServer;

/// Level geometry: the default trace flags.
const DEFAULT_TRACE_FLAGS: u32 = 0x2086;

/// The only trace flags the DLL has proven live (C14, L2).
const ALLOWED_TRACE_FLAGS: [u32; 2] = [0x2086, 0x20bf];

/// The flags to send: the default when none is given, else only an allowed value.
fn trace_flags(flags: Option<u32>) -> Result<u32, &'static str> {
    let f = flags.unwrap_or(DEFAULT_TRACE_FLAGS);
    if ALLOWED_TRACE_FLAGS.contains(&f) {
        Ok(f)
    } else {
        Err("trace_line: flags must be 0x2086 or 0x20bf")
    }
}

/// The tool's error body for a failed bridge call. An old DLL without
/// `trace_line` (`-32601`) gets a hint instead of the raw bridge text.
fn trace_failure(e: &str) -> String {
    if e.contains("-32601") {
        json!({ "error": "trace_line needs a lab DLL with trace_line (AX-55)" }).to_string()
    } else {
        json!({ "error": e }).to_string()
    }
}

/// Args for `client_trace_line`. `instance` and `lease_id` are routed by the
/// handler, so they need no field here.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TraceArgs {
    /// Start point `[x, y, z]`, client UE3 units.
    pub start: [f64; 3],
    /// End point `[x, y, z]`, client UE3 units.
    pub end: [f64; 3],
    /// `0x2086` (default, level geometry) or `0x20bf` (also pawns).
    #[serde(default)]
    pub flags: Option<u32>,
}

#[tool_router(router = trace_router, vis = "pub(super)")]
impl LabServer {
    #[tool(
        description = "Trace a line in the client world (UE3 units, client space) through the engine's own collision check. flags 0x2086 (level geometry, the default) or 0x20bf (also pawns). Returns hit, time, location, normal, actor and item (a level hit has item >= 0). A read: no evidence."
    )]
    async fn client_trace_line(
        &self,
        Parameters(a): Parameters<TraceArgs>,
    ) -> Result<CallToolResult, McpError> {
        let flags = trace_flags(a.flags).map_err(|m| McpError::invalid_params(m, None))?;
        let params = json!({ "start": a.start, "end": a.end, "flags": flags });
        let out = self.supervisor.bridge_call("trace_line", params).await;
        Ok(match out {
            Ok(v) => CallToolResult::success(vec![ContentBlock::text(v.to_string())]),
            Err(e) => CallToolResult::error(vec![ContentBlock::text(trace_failure(&e))]),
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    #[test]
    fn flags_default_and_allowed_values_pass() {
        assert_eq!(trace_flags(None), Ok(0x2086));
        assert_eq!(trace_flags(Some(0x20bf)), Ok(0x20bf));
    }

    #[test]
    fn other_flags_are_refused_before_the_bridge() {
        for v in [0, 0x20be, 0x3fff, 0x2087] {
            assert_eq!(
                trace_flags(Some(v)),
                Err("trace_line: flags must be 0x2086 or 0x20bf"),
                "flags {v:#x}"
            );
        }
    }

    #[test]
    fn old_dll_gets_the_upgrade_hint() {
        let body: Value = serde_json::from_str(&trace_failure(
            "bridge: bridge error -32601: method not found",
        ))
        .unwrap();
        assert_eq!(
            body,
            json!({ "error": "trace_line needs a lab DLL with trace_line (AX-55)" })
        );
    }

    #[test]
    fn other_failures_keep_their_text() {
        let body: Value = serde_json::from_str(&trace_failure("boom")).unwrap();
        assert_eq!(body, json!({ "error": "boom" }));
    }
}
