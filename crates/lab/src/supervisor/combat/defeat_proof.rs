//! Evidence for `client_die_and_respawn`: a respawn needs both reads.
//!
//! `alive` is the health read `defeat.rs` takes today. The `GameBeing + 0x158`
//! dead bit is not read here (L6): it is only seen on an NPC so far.

use crate::proof::{Check, Source, Why};

/// The two required Client checks behind a respawn. `alive` is `None` when the
/// health read did not run, which leaves the verdict inconclusive.
pub fn defeat_checks(window_closed: bool, alive: Option<bool>) -> Vec<Check> {
    let alive_check = Check::new("alive", Source::Client, alive, true);
    let alive_check = match alive {
        None => alive_check.with_why(Why::NoPostRead),
        Some(_) => alive_check,
    };
    vec![
        Check::new(
            "defeat_window_closed",
            Source::Client,
            Some(window_closed),
            true,
        ),
        alive_check,
    ]
}

/// The one required Client check behind a `respawn: "none"` result: the defeat window was seen open.
pub fn defeated_checks() -> Vec<Check> {
    vec![Check::new(
        "defeat_window_open",
        Source::Client,
        Some(true),
        true,
    )]
}

#[cfg(test)]
#[path = "defeat_proof_tests.rs"]
mod tests;
