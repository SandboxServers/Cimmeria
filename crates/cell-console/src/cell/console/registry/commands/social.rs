//! Social (`console/social.rs`): the GM broadcast and the chat mutes.
//! Duels (`console/duel.rs`): the registry read and the GM abort (SS-U2).

use super::{spec, Spec, Target};

// `min` is 0 on purpose: a bare `.announce` must reach
// `social::parse_announce`, which answers with the usage line and logs
// `chat.gm_broadcast_rejected reason=no_text`. The generic argc check would
// refuse it first with a generic line and no chat event. `.mute` and
// `.unmute` do the same, for `chat.gm_mute_refused reason=usage`.
pub(super) const SPECS: &[Spec] = &[
    spec(
        "announce",
        0,
        usize::MAX,
        Target::None,
        "Broadcast a server line to every online player, or to your space with `space` first ([space] text)",
    ),
    spec(
        "duel_status",
        0,
        1,
        Target::None,
        "Show a player's duel or duel challenge, or your own with no name ([name])",
    ),
    // `min` is 0 for the same reason as `announce`: a bare `.duel_end`
    // reaches `duel::parse_duel_end`'s usage line and logs
    // `duel.gm_rejected reason=no_name`.
    spec(
        "duel_end",
        0,
        1,
        Target::None,
        "End a player's duel or duel challenge; both players are told \"Duel aborted\" (name)",
    ),
    spec(
        "mute",
        0,
        usize::MAX,
        Target::None,
        "Stop an online player's chat and tells for some minutes (name minutes [reason])",
    ),
    spec(
        "unmute",
        0,
        usize::MAX,
        Target::None,
        "Lift an online player's chat mute (name)",
    ),
];
