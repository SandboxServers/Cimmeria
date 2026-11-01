//! The C2 reason codes. A judged result says why it was judged that way,
//! and `hint` tells the reader what to do next.

use serde::{Deserialize, Serialize};

/// Why a result has its verdict (contract C2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Why {
    Confirmed,
    NotDelivered,
    Ignored,
    NoEffect,
    OutOfRange,
    NoHover,
    StaleRead,
    NoPostRead,
    ServerUnreachable,
    ServerDisagrees,
    NotYetJudged,
    NoEvidence,
    CoolingDown,
    Delivered,
    PostedOnly,
    Timeout,
    Stalled,
    AlreadyTrue,
}

impl Why {
    /// Every code, in contract table order.
    pub const ALL: [Why; 18] = [
        Why::Confirmed,
        Why::NotDelivered,
        Why::Ignored,
        Why::NoEffect,
        Why::OutOfRange,
        Why::NoHover,
        Why::StaleRead,
        Why::NoPostRead,
        Why::ServerUnreachable,
        Why::ServerDisagrees,
        Why::NotYetJudged,
        Why::NoEvidence,
        Why::CoolingDown,
        Why::Delivered,
        Why::PostedOnly,
        Why::Timeout,
        Why::Stalled,
        Why::AlreadyTrue,
    ];

    /// The wire name, the same as the serde `snake_case` form.
    pub fn as_str(self) -> &'static str {
        match self {
            Why::Confirmed => "confirmed",
            Why::NotDelivered => "not_delivered",
            Why::Ignored => "ignored",
            Why::NoEffect => "no_effect",
            Why::OutOfRange => "out_of_range",
            Why::NoHover => "no_hover",
            Why::StaleRead => "stale_read",
            Why::NoPostRead => "no_post_read",
            Why::ServerUnreachable => "server_unreachable",
            Why::ServerDisagrees => "server_disagrees",
            Why::NotYetJudged => "not_yet_judged",
            Why::NoEvidence => "no_evidence",
            Why::CoolingDown => "cooling_down",
            Why::Delivered => "delivered",
            Why::PostedOnly => "posted_only",
            Why::Timeout => "timeout",
            Why::Stalled => "stalled",
            Why::AlreadyTrue => "already_true",
        }
    }

    /// The hint text a reader sees next to the code.
    pub fn hint(self) -> &'static str {
        match self {
            Why::Confirmed => "The result was read back after the action and matches.",
            Why::NotDelivered => {
                "The game never read the input (its input counters did not move). Check focus, then retry once."
            }
            Why::Ignored => {
                "The game read the input but nothing changed. Something is blocking it (a dialog, a cast, a stun, UI focus): read the UI state."
            }
            Why::NoEffect => "The action ran but the expected change did not happen.",
            Why::OutOfRange => {
                "The target is out of range. Move closer and retry (interact range is about 5 m)."
            }
            Why::NoHover => {
                "The cursor was not over the target when it clicked. Find the entity again and retry."
            }
            Why::StaleRead => {
                "The read-back came from the same frame as the action, so it proves nothing. Read again."
            }
            Why::NoPostRead => {
                "Nothing was read back after the action, so the result cannot be judged. Read the state you expected to change."
            }
            Why::ServerUnreachable => {
                "The server could not be asked, so nothing corroborates the client. Report INCONCLUSIVE."
            }
            Why::ServerDisagrees => {
                "The client shows the change but the server does not. Report it as a bug, not a pass."
            }
            Why::NotYetJudged => {
                "This tool does not judge its result yet. Treat the result as a claim, not a fact."
            }
            Why::NoEvidence => {
                "This tool should judge its result but returned no evidence. Report INCONCLUSIVE."
            }
            Why::CoolingDown => {
                "The same call failed moments ago. Wait retry_after_ms or change the arguments."
            }
            Why::Delivered => {
                "The game received the input. Whether it had an effect is not judged here."
            }
            Why::PostedOnly => {
                "The input was posted to the game window. Nothing can read back whether the game used it: read the state you expected to change."
            }
            Why::Timeout => "The action did not finish before its deadline.",
            Why::Stalled => "The action stopped making progress. Read stop for what was tried.",
            Why::AlreadyTrue => {
                "The expected state already held before the action, so reading it back proves nothing. Start from a state where it does not hold."
            }
        }
    }
}
