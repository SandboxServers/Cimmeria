//! `sendDuelChallenge`, the SGWPlayer base method 0xD9, and its decoder.
//!
//! `SGWPlayer.def:509-513` declares it `Exposed` on the base with
//! `(WSTRING aPlayerName, INT8 aSquadDuel)`. The base-method wire id is
//! `0xC0 + 25` (`docs/protocol/sgwplayer-base-method-dispatch-table.md`).

use crate::cell::cell_methods::organization::{ArgReader, OrgDecodeError};

/// `sendDuelChallenge(WSTRING aPlayerName, INT8 aSquadDuel)`.
pub const SEND_DUEL_CHALLENGE: u8 = 0xD9;

/// One decoded `sendDuelChallenge`. The name is exactly what the player
/// typed; the base resolves it against the online index (D-SS13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuelChallengeCall {
    pub player_name: String,
    /// Nonzero asks for a squad duel. Squad duels are refused (SS-D1).
    pub squad_duel: i8,
}

impl DuelChallengeCall {
    /// `true` when the client asked for a squad duel.
    pub fn is_squad(&self) -> bool {
        self.squad_duel != 0
    }
}

/// Decode `sendDuelChallenge`. The argument reader is the organization one
/// (bounded `WSTRING`, trailing bytes rejected), so its error type is shared.
pub fn decode_send_duel_challenge(payload: &[u8]) -> Result<DuelChallengeCall, OrgDecodeError> {
    let mut r = ArgReader::new(payload);
    let player_name = r.wstring("aPlayerName")?;
    let squad_duel = r.u8("aSquadDuel")? as i8;
    r.finish()?;
    Ok(DuelChallengeCall {
        player_name,
        squad_duel,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `WSTRING "Bo"`.
    const WS_BO: [u8; 8] = [2, 0, 0, 0, 0x42, 0, 0x6F, 0];

    #[test]
    fn id_is_base_index_25() {
        assert_eq!(SEND_DUEL_CHALLENGE, 0xC0 + 25);
    }

    /// Name first, then one squad byte.
    #[test]
    fn decodes_name_then_squad_byte() {
        let payload = [&WS_BO[..], &[0]].concat();
        assert_eq!(
            decode_send_duel_challenge(&payload),
            Ok(DuelChallengeCall {
                player_name: "Bo".into(),
                squad_duel: 0
            })
        );
        let squad = [&WS_BO[..], &[1]].concat();
        assert!(decode_send_duel_challenge(&squad).unwrap().is_squad());
    }

    /// A missing squad byte, a trailing byte, or a forged name length does
    /// not decode.
    #[test]
    fn rejects_truncated_trailing_and_forged_length() {
        assert_eq!(
            decode_send_duel_challenge(&WS_BO).unwrap_err().reason(),
            "truncated"
        );
        let trailing = [&WS_BO[..], &[0, 0]].concat();
        assert_eq!(
            decode_send_duel_challenge(&trailing).unwrap_err().reason(),
            "trailing_bytes"
        );
        let forged = [0xFF, 0xFF, 0xFF, 0xFF, 0x42, 0, 0];
        assert_eq!(
            decode_send_duel_challenge(&forged).unwrap_err().reason(),
            "truncated"
        );
    }
}
