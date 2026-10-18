//! One exported telemetry row: a flat JSON object read from a JSONL line.

use std::path::Path;

/// A flat JSON object, keyed by tracing field name plus `timestamp`.
#[derive(Debug, Clone, PartialEq)]
pub struct Row(pub serde_json::Map<String, serde_json::Value>);

impl Row {
    /// The value of `key` when it is a JSON string.
    pub fn str(&self, key: &str) -> Option<&str> {
        self.0.get(key)?.as_str()
    }

    /// The value of `key` as a float: a JSON number, or a string that parses as one.
    pub fn f64(&self, key: &str) -> Option<f64> {
        match self.0.get(key)? {
            serde_json::Value::Number(n) => n.as_f64(),
            serde_json::Value::String(s) => s.trim().parse().ok(),
            _ => None,
        }
    }

    /// The value of `key` as an integer: a JSON integer, or a string that parses as one.
    pub fn i64(&self, key: &str) -> Option<i64> {
        match self.0.get(key)? {
            serde_json::Value::Number(n) => n.as_i64(),
            serde_json::Value::String(s) => s.trim().parse().ok(),
            _ => None,
        }
    }

    /// The value of `key` as text: a JSON string as is, a JSON number in its decimal form.
    pub fn text(&self, key: &str) -> Option<String> {
        match self.0.get(key)? {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        }
    }
}

/// Parses JSONL text: one JSON object per non-blank line. An error names its line as `line N: ...`.
///
/// A leading UTF-8 byte-order mark is ignored, since Windows tools add one to redirected output.
pub fn parse_jsonl(text: &str) -> Result<Vec<Row>, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|e| format!("line {number}: {e}"))?;
        match value {
            serde_json::Value::Object(map) => rows.push(Row(map)),
            _ => return Err(format!("line {number}: not a JSON object")),
        }
    }
    Ok(rows)
}

/// Reads and parses a JSONL file exported from ClickHouse.
pub fn read_jsonl(path: &Path) -> Result<Vec<Row>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_jsonl(&text)
}
