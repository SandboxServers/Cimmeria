//! Tests for the MCP-side evidence gate.

use rmcp::{model::*, ErrorData as McpError};
use serde_json::{json, Value};

use super::stamp_response;

fn complete(blocks: Vec<ContentBlock>) -> Result<CallToolResponse, McpError> {
    Ok(CallToolResponse::Complete(CallToolResult::success(blocks)))
}

fn text_of(out: &Result<CallToolResponse, McpError>, i: usize) -> String {
    match out {
        Ok(CallToolResponse::Complete(r)) => match &r.content[i] {
            ContentBlock::Text(t) => t.text.clone(),
            _ => panic!("block {i} is not text"),
        },
        _ => panic!("not a complete result"),
    }
}

#[test]
fn stamp_response_rewrites_json_text_only() {
    let mut out = complete(vec![
        ContentBlock::text(json!({"output": 1}).to_string()),
        ContentBlock::text("plain words"),
    ]);
    let verdict = stamp_response("x_unrouted_tool", &mut out);
    assert_eq!(verdict.as_deref(), Some("unjudged"));
    let stamped: Value = serde_json::from_str(&text_of(&out, 0)).unwrap();
    assert_eq!(stamped["output"], 1);
    assert_eq!(stamped["evidence"]["verdict"], "unjudged");
    assert_eq!(stamped["verdict"], "unjudged");
    assert_eq!(text_of(&out, 1), "plain words");
}

#[test]
fn stamp_response_skips_errors_and_non_objects() {
    let mut out: Result<CallToolResponse, McpError> = Err(McpError::internal_error(
        "boom",
        Some(json!({"effect_ok": true})),
    ));
    assert_eq!(stamp_response("x_unrouted_tool", &mut out), None);
    let mut out = complete(vec![ContentBlock::text("[1, 2]")]);
    assert_eq!(stamp_response("x_unrouted_tool", &mut out), None);
    assert_eq!(text_of(&out, 0), "[1, 2]");
}
