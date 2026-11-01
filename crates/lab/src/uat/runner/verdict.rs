//! How the runner reads a tool's `evidence` verdict (AX-12, contract C1):
//! a denied action fails like `effect_ok: false`; an inconclusive one is
//! noted and, without an independent passing proof, holds the row at
//! UNVERIFIED. Unjudged results change nothing.

use serde_json::Value;

use crate::proof::{EvVerdict, Evidence};
use crate::uat::evidence::{ClauseResult, Verdict};
use crate::uat::grade::{mark_unverified, Grade};

/// What the runner does with one action's result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ActionVerdict {
    Proceed,
    /// The tool's own evidence says the action did not happen.
    Denied(String),
    /// The tool could not tell; the note goes on the row.
    Inconclusive(String),
}

/// Read the `evidence` object of a tool result, when it has one.
pub(super) fn of_outcome(tool: &str, json: &Value) -> ActionVerdict {
    let Some(ev) = json.get("evidence").and_then(Evidence::from_value) else {
        return ActionVerdict::Proceed;
    };
    match ev.verdict {
        EvVerdict::Denied => {
            ActionVerdict::Denied(format!("{tool} denied: {}: {}", ev.why.as_str(), ev.hint))
        }
        EvVerdict::Inconclusive => {
            ActionVerdict::Inconclusive(format!("{tool}: inconclusive ({})", ev.why.as_str()))
        }
        EvVerdict::Confirmed | EvVerdict::Unjudged => ActionVerdict::Proceed,
    }
}

/// Apply the inconclusive notes to a graded row: one flag each, and
/// UNVERIFIED unless a required clause passed on its own read (that read,
/// not the action, proved the outcome).
pub(super) fn settle(g: &mut Grade, inconclusive: &[String], clauses: &[ClauseResult]) {
    if inconclusive.is_empty() {
        return;
    }
    g.flags
        .extend(inconclusive.iter().map(|n| format!("inconclusive: {n}")));
    if !clauses
        .iter()
        .any(|c| c.required && c.verdict == Verdict::Pass)
    {
        mark_unverified(g, inconclusive);
    }
}

#[cfg(test)]
#[path = "verdict_tests.rs"]
mod tests;
