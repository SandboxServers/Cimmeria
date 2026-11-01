//! The evidence gate (C3): stamps a tool result with the verdict its evidence
//! supports, downgrades any claim the evidence does not back, and mirrors the
//! verdict at the top level. Errors and claim-free tools pass untouched.

use serde_json::{json, Map, Value};

use super::claims::{claim, Claim};
use super::{EvVerdict, Evidence};

/// Whether an `Unjudged` result loses `effect_ok` like an `Inconclusive` one.
/// AX-44 sets this on; until then unjudged results keep their legacy flag.
pub const STRICT: bool = false;

/// Stamp `v`, a tool result from `tool`, in place (C3 rules a-f).
pub fn stamp(tool: &str, v: &mut Value) {
    stamp_with(claim(tool), v);
}

/// The mirrored top-level `verdict` of a stamped result.
pub fn verdict_of(v: &Value) -> Option<&str> {
    v.get("verdict").and_then(Value::as_str)
}

/// [`stamp`] with the claim given, so tests can exercise each claim kind.
fn stamp_with(claim: Claim, v: &mut Value) {
    let Some(obj) = v.as_object_mut() else {
        return;
    };
    if obj.contains_key("error") || claim == Claim::NoClaim {
        return;
    }
    let ev = match obj.get("evidence").and_then(Evidence::from_value) {
        Some(ev) => ev,
        None => {
            let ev = match claim {
                Claim::Judged => Evidence::no_evidence(),
                _ => Evidence::unjudged(),
            };
            obj.insert("evidence".into(), ev.to_value());
            ev
        }
    };
    let verdict = ev.verdict;
    let mut claimed = Map::new();
    if verdict != EvVerdict::Confirmed {
        for key in ["counts_as_native_pass", "native_pass"] {
            if obj.get(key) == Some(&json!(true)) {
                obj.insert(key.into(), json!(false));
                claimed.insert(key.into(), json!(true));
            }
        }
    }
    match (verdict, obj.get("effect_ok").cloned()) {
        (EvVerdict::Denied, Some(Value::Bool(true))) => {
            obj.insert("effect_ok".into(), json!(false));
            claimed.insert("effect_ok".into(), json!(true));
        }
        (EvVerdict::Inconclusive, Some(old)) => {
            obj.remove("effect_ok");
            claimed.insert("effect_ok".into(), old);
        }
        (EvVerdict::Unjudged, Some(old)) if STRICT => {
            obj.remove("effect_ok");
            claimed.insert("effect_ok".into(), old);
        }
        _ => {}
    }
    if !claimed.is_empty() {
        obj.insert("claimed".into(), Value::Object(claimed));
    }
    let name = serde_json::to_value(verdict).unwrap_or(Value::Null);
    obj.insert("verdict".into(), name);
}

#[cfg(test)]
#[path = "gate_tests.rs"]
mod tests;
