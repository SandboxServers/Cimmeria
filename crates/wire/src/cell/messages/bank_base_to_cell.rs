//! `BankBaseToCell`: bank and vault traffic from the base to the cell,
//! carried by `BaseToCellMsg::Bank`.
//!
//! One nested enum, like [`super::BankCellToBase`] in the other direction.
//! Every variant carries the player's `player_id` and `entity_id` as the
//! cell sent them, so the cell can tell a stale entity from a live one.

/// Bank messages sent from BaseApp to CellApp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BankBaseToCell {
    /// The answer to `BankCellToBase::ExpansionQuote` when the vault can
    /// still grow (BV-05): record the offer on the vault session and show
    /// the Expand dialog. The base sends nothing at the ceiling.
    OfferExpansion {
        /// The player's entity id.
        entity_id: u32,
        /// `sgw_player.player_id`; the cell drops the offer for an entity
        /// that is no longer this character.
        player_id: i32,
        /// The Expand dialog's speaker, echoed from the quote.
        speaker_id: u32,
        /// `sgw_player.bank_slots` when the base read it. The purchase is
        /// keyed on it.
        from_slots: i16,
        /// `bank_expansion_price.price_naquadah` of the step to
        /// `from_slots + 10`, shown to the player in chat.
        price: i32,
    },
}

impl BankBaseToCell {
    /// Short stable name for the `kind` log field.
    pub fn kind(&self) -> &'static str {
        match self {
            BankBaseToCell::OfferExpansion { .. } => "offer_expansion",
        }
    }
}
