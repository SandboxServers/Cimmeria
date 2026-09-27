//! `DuelBaseToCell`: duel traffic from the base to the cell, carried by
//! `BaseToCellMsg::Duel`.
//!
//! The challenge arrives on the base (`sendDuelChallenge`, 0xD9), which
//! runs the rate limit, the online lookup (D-SS13) and the Ignore check
//! (D-SS15), then forwards it here. The cell owns the duel registry. Every
//! variant carries both players' `player_id` and `entity_id` from the base's
//! own session map (`ConnectedClientState`), never from the client payload.

/// Duel messages sent from BaseApp to CellApp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DuelBaseToCell {
    /// A challenge that passed the base's checks. The cell checks self,
    /// space, range, busy and the per-pair cooldown, then prompts the target.
    Challenge {
        player_id: i32,
        entity_id: u32,
        account_id: u32,
        target_player_id: i32,
        target_entity_id: u32,
    },
}

impl DuelBaseToCell {
    /// The acting player's `(player_id, entity_id)`.
    pub fn actor(&self) -> (i32, u32) {
        match *self {
            DuelBaseToCell::Challenge {
                player_id,
                entity_id,
                ..
            } => (player_id, entity_id),
        }
    }

    /// Short stable name for the `kind` log field.
    pub fn kind(&self) -> &'static str {
        match self {
            DuelBaseToCell::Challenge { .. } => "challenge",
        }
    }
}
