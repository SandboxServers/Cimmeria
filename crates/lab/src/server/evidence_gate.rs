//! The evidence gate on the MCP path (C3): stamps the JSON results of a
//! completed tool call before they are logged and shaped.

use rmcp::{model::*, ErrorData as McpError};
use serde_json::Value;

use crate::proof::gate;

/// Stamp each JSON object text block and the structured content of a
/// completed call. Returns the mirrored verdict of the first stamped block.
/// Errors, images, plain text and other results are left as they are.
pub(super) fn stamp_response(
    tool: &str,
    out: &mut Result<CallToolResponse, McpError>,
) -> Option<String> {
    let Ok(CallToolResponse::Complete(r)) = out else {
        return None;
    };
    let mut verdict = None;
    for block in &mut r.content {
        let ContentBlock::Text(t) = block else {
            continue;
        };
        let Ok(mut v) = serde_json::from_str::<Value>(&t.text) else {
            continue;
        };
        if !v.is_object() {
            continue;
        }
        gate::stamp(tool, &mut v);
        if verdict.is_none() && v.get("evidence").is_some() {
            verdict = gate::verdict_of(&v).map(String::from);
        }
        if let Ok(s) = serde_json::to_string(&v) {
            t.text = s;
        }
    }
    if let Some(sc) = r.structured_content.as_mut() {
        if sc.is_object() {
            gate::stamp(tool, sc);
        }
    }
    verdict
}

#[cfg(test)]
#[path = "evidence_gate_tests.rs"]
mod tests;
