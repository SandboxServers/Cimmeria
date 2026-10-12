//! The `lab record` JSON line, the failure body and the exit-code mapping of a
//! failed tool call (D-LR16).

use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;
use serde_json::Value;

use crate::ascii_json::to_ascii_json;
use crate::daemon::mcp::{hide_lease_ids, McpError};

/// A `--json` line stays under this many characters (a failure's error is cut to fit).
pub const BODY_CAP: usize = 400;

/// A JSON object as ordered pairs.
pub type Body = Vec<(String, Value)>;

/// `{ok:false, exit, error}`, with lease ids hidden in the error.
pub fn failure(exit: u8, message: &str) -> Body {
    vec![
        ("ok".to_owned(), Value::Bool(false)),
        ("exit".to_owned(), Value::from(exit)),
        ("error".to_owned(), Value::String(hide_lease_ids(message))),
    ]
}

/// A tool's answer as a body (an answer that is not an object gives an empty body).
pub fn body_from_reply(reply: &Value) -> Body {
    match reply.as_object() {
        Some(map) => map.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        None => Vec::new(),
    }
}

/// One value of a body.
pub fn body_get<'a>(body: &'a Body, key: &str) -> Option<&'a Value> {
    body.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// The body's `error` text, or an empty string.
pub fn error_text(body: &Body) -> String {
    body_get(body, "error")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

/// Ordered pairs as a JSON object, keys in the order given.
struct Pairs<'a>(&'a [(&'a str, Value)]);

impl Serialize for Pairs<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in self.0 {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

/// One compact JSON line: null and empty-string top-level values dropped (zeros
/// and `false` kept). When the body has a string `error`, the error is cut until
/// the line is under [`BODY_CAP`] characters.
pub fn compact_json(body: &Body) -> String {
    let mut kept: Vec<(&str, Value)> = body
        .iter()
        .filter(|(_, v)| !(v.is_null() || v.as_str() == Some("")))
        .map(|(k, v)| (k.as_str(), v.clone()))
        .collect();
    let mut line = line_of(&kept);
    let error_at = kept
        .iter()
        .position(|(k, v)| *k == "error" && v.is_string());
    if let Some(at) = error_at {
        let error = kept[at].1.as_str().unwrap_or("").to_owned();
        let mut keep = error.chars().count();
        while line.chars().count() >= BODY_CAP && keep > 0 {
            let over = line.chars().count() + 1 - BODY_CAP;
            keep = keep.saturating_sub(over.max(1));
            kept[at].1 = Value::String(error.chars().take(keep).collect());
            line = line_of(&kept);
        }
    }
    line
}

fn line_of(pairs: &[(&str, Value)]) -> String {
    to_ascii_json(&Pairs(pairs))
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("")
}

/// `{ok:false, exit, error}` text from a tool error, as `(exit, body)`. The
/// exit must be 1..=255; anything else becomes 1.
fn parse_refusal(text: &str) -> Option<(u8, Body)> {
    let value: Value = serde_json::from_str(text).ok()?;
    let map = value.as_object()?;
    if map.get("ok") != Some(&Value::Bool(false)) {
        return None;
    }
    let exit = map.get("exit")?;
    let error = map.get("error")?;
    let code = exit
        .as_i64()
        .or_else(|| exit.as_str().and_then(|s| s.trim().parse().ok()))?;
    let code = u8::try_from(code).ok().filter(|c| *c >= 1).unwrap_or(1);
    let message = match error {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    Some((code, failure(code, &message)))
}

/// A failed call as `(exit, body)`. A tool error whose text is the refusal JSON
/// keeps its body and its exit (the daemon answers 4 itself for a busy
/// instance); any other tool error is exit 1; an empty-reply protocol error
/// (`<tool> returned ...`) is exit 1; anything else means the daemon did not
/// answer, exit 3.
pub fn call_error(tool: &str, err: &McpError) -> (u8, Body) {
    let message = match err {
        McpError::Tool(text) => {
            let text = hide_lease_ids(text);
            return match parse_refusal(&text) {
                Some(parsed) => parsed,
                None => (1, failure(1, &text)),
            };
        }
        McpError::Protocol(text) => {
            let first = first_line(text);
            if first.starts_with(&format!("{tool} returned")) {
                return (1, failure(1, first));
            }
            format!("the lab daemon did not answer: {first}")
        }
        McpError::Unreachable(text) => {
            format!("the lab daemon did not answer: {}", first_line(text))
        }
        McpError::Http(code, text) => {
            format!(
                "the lab daemon did not answer: HTTP {code}: {}",
                first_line(text)
            )
        }
    };
    (3, failure(3, &message))
}
