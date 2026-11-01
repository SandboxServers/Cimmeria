//! Evidence for `client_use_ability`: did the press get an answer?
//!
//! The result judges the press, not the ability. A refusal is an answer
//! (D-AX14): `debug-area.toml` DA-U22 presses Pistol Shot at a neutral NPC and
//! expects a refusal, so the row's clauses, not this check, decide whether
//! the answer was the right one.

use serde_json::json;

use crate::proof::{Check, Evidence, Source, Why};

/// The one required Client check, `press_answered`, from the `outcome.rs`
/// `verdict()` word and whether the client sent the press.
pub fn ability_evidence(verdict: &str, sent: bool) -> Evidence {
    let (ok, why) = match verdict {
        "effect_applied" | "refused" | "refused_client_side" | "refused_with_feedback" => {
            (Some(true), Why::Confirmed)
        }
        "cast_started" | "sent_no_reply" => (None, Why::NoPostRead),
        "nothing_observed" => (Some(false), Why::Ignored),
        _ => (None, Why::NoEvidence),
    };
    let check = Check::new("press_answered", Source::Client, ok, true)
        .with_after(json!({ "verdict": verdict, "sent": sent }))
        .with_why(why);
    Evidence::judge(vec![check])
}

#[cfg(test)]
#[path = "proof_tests.rs"]
mod tests;
