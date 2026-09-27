//! Duel client methods and the duel feedback texts.
//!
//! Only `onDuelChallenge` [143] has a serializer. `onDuelEntitiesSet`,
//! `Remove` and `Clear` (151-153) are SS-D2's, and the server must not send
//! 151 or 153 before then (D-SS25, audit A-41).
//!
//! # Feedback texts
//!
//! The strings are the client's own duel monikers from `texts.sql`
//! (872-878), sent as literal `onPlayerCommunication` feedback lines. SS-E1
//! found no client path that renders a duel moniker by id (`onErrorCode`
//! is ruled out), so the text travels, not the id
//! (`docs/reverse-engineering/findings/duel-wire-formats.md`, D-Q6). The
//! lines with no client moniker are Cimmeria's own wording.

pub use super::player::ON_DUEL_CHALLENGE;

/// Moniker 872.
pub const TEXT_CHALLENGE_SELF: &str = "You can not challenge yourself to a duel";
/// Moniker 873: the challenger is already in a duel or a pending challenge.
pub const TEXT_ALREADY_IN_DUEL: &str = "You are already involved in a duel";
/// Moniker 877: different space, or beyond the challenge range (D-SS19).
pub const TEXT_NOT_CLOSE_ENOUGH: &str = "You are not close enough to send a duel request";
/// Moniker 878: a declined or expired challenge, told to both sides.
pub const TEXT_DUEL_ABORTED: &str = "Duel aborted";

/// Squad duels are refused. Moniker 874 ("You cannot start a squad duel
/// when not in a squad") would be false for a squad member, so the line is
/// Cimmeria's own.
pub const TEXT_SQUAD_DUEL_UNSUPPORTED: &str = "Squad duels are not available.";
/// The typed name matched no online character.
pub const TEXT_TARGET_NOT_ONLINE: &str = "That player is not online.";
/// The typed name matched more than one online character (D-SS13).
pub const TEXT_TARGET_AMBIGUOUS: &str =
    "More than one player matches that name. Type the exact name.";
/// The target ignores the challenger (D-SS15).
pub const TEXT_TARGET_IGNORING: &str = "That player is not accepting your duel challenges.";
/// The target is already in a duel or a pending challenge (D-SS21).
pub const TEXT_TARGET_BUSY: &str = "That player is already involved in a duel.";
/// The per-pair cooldown after a decline or expiry is running (D-SS21).
pub const TEXT_PAIR_COOLDOWN: &str = "You cannot challenge that player again yet.";
/// The challenge went out: the challenger's acknowledgement.
pub const TEXT_CHALLENGE_SENT: &str = "Duel challenge sent.";
/// A response with no pending challenge addressed to the caller.
pub const TEXT_NO_PENDING_CHALLENGE: &str = "You have no duel challenge to answer.";
/// Accept: both sides, before the countdown.
pub const TEXT_DUEL_ACCEPTED: &str = "Duel accepted. The duel starts in 5 seconds.";

/// `onDuelChallenge(INT32 aEntityId, ARRAY<INT32> aSquadList)` [143]
/// (`SGWPlayer.def:1372-1375`): the challenger's entity id, then a `u32`
/// element count and the squad members' entity ids.
pub fn build_on_duel_challenge(challenger_entity_id: i32, squad: &[i32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + squad.len() * 4);
    out.extend_from_slice(&challenger_entity_id.to_le_bytes());
    let n = u32::try_from(squad.len()).expect("ARRAY element count exceeds u32");
    out.extend_from_slice(&n.to_le_bytes());
    for id in squad {
        out.extend_from_slice(&id.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Byte-exact: the entity id, then an empty array (count 0). Eight
    /// bytes; a one-byte or two-byte count would shift nothing here but
    /// would break the client's array read.
    #[test]
    fn on_duel_challenge_empty_squad_is_byte_exact() {
        assert_eq!(
            build_on_duel_challenge(0x0102_0304, &[]),
            vec![0x04, 0x03, 0x02, 0x01, 0, 0, 0, 0]
        );
        assert_eq!(ON_DUEL_CHALLENGE, 143);
    }

    #[test]
    fn on_duel_challenge_with_squad_is_byte_exact() {
        assert_eq!(
            build_on_duel_challenge(7, &[9, -1]),
            vec![7, 0, 0, 0, 2, 0, 0, 0, 9, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF]
        );
    }

    /// The moniker strings are the client's `texts.sql` rows, verbatim.
    #[test]
    fn moniker_texts_match_the_seed() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../db/resources/Texts/Seed/texts.sql"
        );
        let seed = std::fs::read_to_string(path).expect("read texts.sql");
        for (id, text) in [
            (872, TEXT_CHALLENGE_SELF),
            (873, TEXT_ALREADY_IN_DUEL),
            (877, TEXT_NOT_CLOSE_ENOUGH),
            (878, TEXT_DUEL_ABORTED),
        ] {
            let row = format!("VALUES ({id}, 0, 1033, '{text}', '')");
            assert!(seed.contains(&row), "texts.sql has no row {row}");
        }
    }
}
