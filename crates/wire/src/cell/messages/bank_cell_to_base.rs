//! `BankCellToBase`: bank and vault traffic from the cell to the base,
//! carried by `CellToBaseMsg::Bank`.
//!
//! One nested enum, so later bank packets add a variant here instead of in
//! `cell_to_base.rs` (the organizations pattern). Every variant carries the
//! actor's ids **from the cell's own session state** (`CellEntity`), never
//! from a client payload, and never a privilege bit: the cell has already
//! passed the GM gate on the server-side `access_level`.

/// Whose vault a GM tool is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BankSubject {
    /// The caller's own character (`sgw_player.player_id` from the cell).
    Player(i32),
    /// A character named by the GM, matched exactly against
    /// `sgw_player.player_name` by the base, online or not.
    Name(String),
}

/// Bank messages sent from CellApp to BaseApp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BankCellToBase {
    /// `.bankdump [player]` (bank-vault BV-04): list a character's personal
    /// vault (container 17) to the GM, read-only. The base reads
    /// `sgw_inventory` and `sgw_player.bank_slots`, answers on the GM's
    /// feedback channel, and logs `gm_action action=bankdump`.
    GmDump {
        /// The GM's entity id; the listing is addressed to it.
        entity_id: u32,
        /// The GM's `account.account_id`, `None` if the cell has none.
        account_id: Option<u32>,
        /// The GM's `sgw_player.player_id`, `None` if the cell has none.
        player_id: Option<i32>,
        /// Whose vault to list.
        subject: BankSubject,
    },
}

impl BankCellToBase {
    /// Short stable name for the `kind` log field.
    pub fn kind(&self) -> &'static str {
        match self {
            BankCellToBase::GmDump { .. } => "gm_dump",
        }
    }
}
