//! The quick-fail cooldown (C9): an identical call that just failed is refused
//! for a short, growing backoff, so an agent cannot hammer a broken action.
//! MCP path only; the UAT runner dispatches through the router directly.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use rmcp::{model::*, ErrorData as McpError};
use serde_json::{json, Map, Value};

use crate::lease::policy::OPEN;
use crate::proof::Why;

/// Arguments that shape or route a call without changing what it does.
const IGNORED_ARGS: [&str; 4] = ["lease_id", "instance", "verbose", "fields"];
/// A key with no call for this long is forgotten.
const IDLE_MS: u64 = 60_000;
/// At most this many keys are tracked; the oldest is evicted first.
const MAX_KEYS: usize = 256;
/// The longest backoff.
const CAP_MS: u64 = 8_000;

#[derive(Debug)]
struct Entry {
    failures: u32,
    last_ms: u64,
}

#[derive(Debug, Default)]
pub(super) struct Cooldown {
    keys: HashMap<u64, Entry>,
    /// Insertion order of `keys`, oldest first.
    order: VecDeque<u64>,
}

/// Sort object keys at every level so equal arguments serialise equally
/// whatever order the client sent them in.
fn canonical(v: &Value) -> Value {
    match v {
        Value::Object(m) => {
            let sorted: BTreeMap<&String, Value> =
                m.iter().map(|(k, v)| (k, canonical(v))).collect();
            Value::Object(
                sorted
                    .into_iter()
                    .map(|(k, v)| (k.clone(), v))
                    .collect::<Map<_, _>>(),
            )
        }
        Value::Array(a) => Value::Array(a.iter().map(canonical).collect()),
        other => other.clone(),
    }
}

/// The identity of a call: the tool plus its arguments minus the shaping ones.
pub(super) fn key(tool: &str, args: &Value) -> u64 {
    let mut args = args.clone();
    if let Some(m) = args.as_object_mut() {
        for k in IGNORED_ARGS {
            m.remove(k);
        }
    }
    // serde_json's map is ordered by insertion unless `preserve_order` is
    // off; serialising the BTreeMap-rebuilt value keeps this independent of
    // that feature.
    let text = serde_json::to_string(&canonical(&args)).unwrap_or_default();
    let mut h = DefaultHasher::new();
    tool.hash(&mut h);
    text.hash(&mut h);
    h.finish()
}

/// Tools that are never cooled down (C9): lease and lifecycle tools, UAT
/// runs, recovery, and every read-only tool in `lease::policy::OPEN`.
pub(super) fn exempt(tool: &str) -> bool {
    OPEN.contains(&tool)
        || tool.starts_with("lab_lease_")
        || tool.starts_with("lab_uat_")
        || matches!(
            tool,
            "lab_client_start"
                | "lab_client_stop"
                | "lab_client_restart"
                | "lab_client_status"
                | "lab_recover"
        )
}

/// Whether a finished call counts as a failure: an MCP error, `is_error`, or
/// a first JSON object whose `evidence.verdict` is `denied`.
pub(super) fn failed(out: &Result<CallToolResponse, McpError>) -> bool {
    let Ok(CallToolResponse::Complete(r)) = out else {
        return out.is_err();
    };
    if r.is_error == Some(true) {
        return true;
    }
    r.content
        .iter()
        .find_map(|b| match b {
            ContentBlock::Text(t) => serde_json::from_str::<Value>(&t.text)
                .ok()
                .filter(Value::is_object),
            _ => None,
        })
        .is_some_and(|v| v.pointer("/evidence/verdict").and_then(Value::as_str) == Some("denied"))
}

fn backoff_ms(failures: u32) -> u64 {
    let shift = failures.saturating_sub(1).min(16);
    (500u64 << shift).min(CAP_MS)
}

impl Cooldown {
    fn forget_idle(&mut self, now_ms: u64) {
        self.keys
            .retain(|_, e| now_ms.saturating_sub(e.last_ms) < IDLE_MS);
        let keys = &self.keys;
        self.order.retain(|k| keys.contains_key(k));
    }

    /// The refusal for `key` when a backoff is running, else `None`.
    pub(super) fn check(&mut self, key: u64, now_ms: u64) -> Option<Value> {
        self.forget_idle(now_ms);
        let e = self.keys.get(&key)?;
        let wait = backoff_ms(e.failures);
        let elapsed = now_ms.saturating_sub(e.last_ms);
        if elapsed >= wait {
            return None;
        }
        Some(json!({
            "error": "cooling down",
            "why": Why::CoolingDown.as_str(),
            "hint": Why::CoolingDown.hint(),
            "retry_after_ms": wait - elapsed,
            "failures": e.failures,
        }))
    }

    /// A failure counts up; a success clears the key.
    pub(super) fn record(&mut self, key: u64, failed: bool, now_ms: u64) {
        if !failed {
            if self.keys.remove(&key).is_some() {
                self.order.retain(|k| *k != key);
            }
            return;
        }
        match self.keys.get_mut(&key) {
            Some(e) => {
                e.failures = e.failures.saturating_add(1);
                e.last_ms = now_ms;
            }
            None => {
                self.keys.insert(
                    key,
                    Entry {
                        failures: 1,
                        last_ms: now_ms,
                    },
                );
                self.order.push_back(key);
                while self.order.len() > MAX_KEYS {
                    if let Some(old) = self.order.pop_front() {
                        self.keys.remove(&old);
                    }
                }
            }
        }
    }
}

fn state() -> &'static Mutex<(Cooldown, Instant)> {
    static STATE: OnceLock<Mutex<(Cooldown, Instant)>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new((Cooldown::default(), Instant::now())))
}

/// The handler's pre-call step: the key and, when a backoff is running, the
/// refusal result. `None` for exempt tools.
pub(super) fn gate(
    instance: &str,
    tool: &str,
    args: Option<&Map<String, Value>>,
) -> Option<(u64, Option<CallToolResult>)> {
    if exempt(tool) {
        return None;
    }
    // The state is process-wide, so the instance keeps one client's failure
    // from cooling another client's identical call.
    let mut h = DefaultHasher::new();
    instance.hash(&mut h);
    key(tool, &Value::Object(args.cloned().unwrap_or_default())).hash(&mut h);
    let k = h.finish();
    let mut g = state().lock().unwrap_or_else(|p| p.into_inner());
    let now = g.1.elapsed().as_millis() as u64;
    let refusal =
        g.0.check(k, now)
            .map(|v| CallToolResult::error(vec![ContentBlock::text(v.to_string())]));
    Some((k, refusal))
}

/// The handler's post-call step.
pub(super) fn note(key: u64, out: &Result<CallToolResponse, McpError>) {
    let mut g = state().lock().unwrap_or_else(|p| p.into_inner());
    let now = g.1.elapsed().as_millis() as u64;
    g.0.record(key, failed(out), now);
}

#[cfg(test)]
#[path = "cooldown_tests.rs"]
mod tests;
