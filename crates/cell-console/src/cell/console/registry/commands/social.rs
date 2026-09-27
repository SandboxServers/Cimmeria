//! Social (`console/social.rs`): the GM broadcast and the chat mutes.
//! Duels (`console/duel.rs`): the registry read and the GM abort (SS-U2).
//! Mail (`console/mail.rs`): `.mail`, `.mailbox`, `.mail_expire` (SS-U1).

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
    // `min` 0 and no `max`: every option is optional and the subject is
    // free text; `mail::parse_mail` checks the words itself.
    spec(
        "mail",
        0,
        usize::MAX,
        Target::None,
        "Mail yourself or a player minted cash and an item, no postage ([to name] [cash n] [item typeId [qty]] [cod n] [subject])",
    ),
    spec(
        "mailbox",
        0,
        1,
        Target::None,
        "Show a mailbox: open and archived mail, what is in escrow, the next expiry ([name])",
    ),
    // `min` 0 so a bare `.mail_expire` reaches its own usage line and logs
    // `mail.gm_rejected reason=no_mail_id`.
    spec(
        "mail_expire",
        0,
        1,
        Target::None,
        "Expire a mail now so the expiry sweep takes it; not available until mail expiry lands (mailId)",
    ),
];
