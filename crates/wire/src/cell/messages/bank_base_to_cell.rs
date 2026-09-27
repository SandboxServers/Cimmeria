//! `BankBaseToCell`: bank and vault traffic from the base to the cell,
//! carried by `BaseToCellMsg::Bank`.
//!
//! One nested enum, so later bank packets add a variant here instead of in
//! `base_to_cell.rs` (the organizations pattern). Every variant carries the
//! player's ids from the base's own session and database reads, never from
//! a client payload.

use cimmeria_entity::cell_entity::VaultScope;

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

    /// The answer to `BankCellToBase::OrgVaultOpen` when the player is a
    /// member (bank-vault BV-07). The base has already sent the vault's
    /// `onBagInfo` and contents; the cell records the Team or Command vault
    /// session and sends `onTeamVaultOpen` (107) or `onCommandVaultOpen`
    /// (108), after checking that the player still has that Banker pinned.
    OrgVaultGranted {
        entity_id: u32,
        /// The member's character, so the cell can tell a stale entity id
        /// (a gate travel since the request) from a live one.
        player_id: i32,
        scope: VaultScope,
        org_id: i32,
        /// The Banker the request named.
        banker_id: u32,
    },
}

impl BankBaseToCell {
    /// Short stable name for the `kind` log field.
    pub fn kind(&self) -> &'static str {
        match self {
            BankBaseToCell::OfferExpansion { .. } => "offer_expansion",
            BankBaseToCell::OrgVaultGranted { .. } => "org_vault_granted",
        }
    }
}
