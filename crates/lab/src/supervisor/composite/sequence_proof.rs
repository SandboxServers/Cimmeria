//! The checks `client_ui_sequence` is judged by (AX-46). Each action adds
//! the checks its own result can prove: a failed step denies the sequence, a
//! wait that found its window confirms it, a drag that moved confirms it and
//! a typed edit box must hold the text. Bare clicks and keys add nothing, so
//! a sequence of only those is `posted_only`.

use serde_json::{json, Value};

use super::sequence::Action;
use crate::proof::{Check, EvVerdict, Evidence, Source, Why};

/// The checks one action adds, given its result. `index` is the action's
/// position from 0 and is carried in each check's `before`. `moved` is set
/// only for a drag: whether it moved the item.
pub(super) fn action_checks(
    index: usize,
    action: &Action,
    result: &Result<Value, String>,
    moved: Option<bool>,
) -> Vec<Check> {
    let at = json!({ "action": index });
    let v = match result {
        Err(e) => {
            let c = Check::new(
                &format!("action_{index}"),
                Source::Client,
                Some(false),
                true,
            )
            .with_before(at)
            .with_after(json!(e))
            .with_why(Why::NoEffect);
            return vec![c];
        }
        Ok(v) => v,
    };
    match action {
        Action::WaitWindow { window, gone, .. } => {
            let after = json!({ "window": window, "gone": gone, "ms": v["ms"] });
            let c = Check::new(&format!("window_{index}"), Source::Client, Some(true), true);
            vec![c.with_before(at).with_after(after)]
        }
        Action::Drag { .. } if moved == Some(true) => {
            let c = Check::new(&format!("drag_{index}"), Source::Client, Some(true), true);
            vec![c.with_before(at).with_after(v.clone())]
        }
        Action::Type { into: Some(_), .. } => typed_check(index, v).into_iter().collect(),
        _ => Vec::new(),
    }
}

/// The `text_entered` check of a typing step (the one `type_into` judged its
/// edit box by), renamed `typed_<index>`. `None` when the result has none.
fn typed_check(index: usize, result: &Value) -> Option<Check> {
    let ev = Evidence::from_value(&result["evidence"])?;
    let mut c = ev.checks.into_iter().find(|c| c.name == "text_entered")?;
    c.name = format!("typed_{index}");
    c.before = Some(json!({ "action": index }));
    Some(c)
}

/// The sequence's evidence from every action's checks: judged when one of
/// them is required, else `posted_only`, since nothing proved an effect.
pub(super) fn sequence_evidence(checks: Vec<Check>) -> Evidence {
    if checks.iter().any(|c| c.required) {
        Evidence::judge(checks)
    } else {
        Evidence::from_verdict(EvVerdict::Inconclusive, Why::PostedOnly, checks)
    }
}

#[cfg(test)]
#[path = "sequence_proof_tests.rs"]
mod tests;
