//! Evidence for `client_minigame_play` (D-AX11). A real minigame is judged by
//! the server board, which must say `victory`. A placeholder game has no
//! published board, so the client window decides, and the server is a note.

use serde_json::json;

use crate::proof::{Check, Source, Why};

/// The checks behind a minigame result. `placeholder` is the caller's
/// `is_placeholder(game)`; `server_outcome` is the board's `outcome` word.
pub fn minigame_checks(
    game: &str,
    placeholder: bool,
    server_outcome: Option<&str>,
    client_window_hidden: bool,
) -> Vec<Check> {
    if placeholder {
        vec![
            Check::new(
                "client_window_hidden",
                Source::Client,
                Some(client_window_hidden),
                true,
            )
            .with_before(json!(game)),
            server_check(game, server_outcome, false),
        ]
    } else {
        vec![server_check(game, server_outcome, true)]
    }
}

/// The `server_outcome` check: `victory` passes, another word fails with
/// `no_effect`, and no board read is `server_unreachable`.
fn server_check(game: &str, server_outcome: Option<&str>, required: bool) -> Check {
    let check = Check::new(
        "server_outcome",
        Source::Server,
        server_outcome.map(|outcome| outcome == "victory"),
        required,
    )
    .with_before(json!(game));
    match server_outcome {
        None => check.with_why(Why::ServerUnreachable),
        Some("victory") => check.with_after(json!("victory")),
        Some(other) => check.with_after(json!(other)).with_why(Why::NoEffect),
    }
}

#[cfg(test)]
#[path = "proof_tests.rs"]
mod tests;
