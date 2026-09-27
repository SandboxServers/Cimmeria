//! `BankCellToBase`: bank and vault traffic from the cell to the base,
//! carried by `CellToBaseMsg::Bank`.
//!
//! One nested enum, so later bank packets add a variant here instead of in
//! `cell_to_base.rs` (the organizations pattern). Every variant carries the
//! actor's ids **from the cell's own session state** (`CellEntity`), never
//! from a client payload, and never a privilege bit: the cell has already
//! passed the GM gate on the server-side `access_level`.

use crate::cell::vault::VaultAccess;
use cimmeria_entity::cell_entity::ExpansionOffer;
use cimmeria_entity::cell_entity::VaultScope;

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
#[derive(Debug, Clone, PartialEq)]
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

    /// A personal vault just opened, at a Banker or with GM `.bank`
    /// (BV-05): may the player be offered the next expansion? The base
    /// reads `sgw_player.bank_slots` and the step's price, and answers with
    /// [`BankBaseToCell::OfferExpansion`] while `bank_slots < 100`.
    ///
    /// [`BankBaseToCell::OfferExpansion`]: super::BankBaseToCell::OfferExpansion
    ExpansionQuote {
        /// The player's entity id.
        entity_id: u32,
        /// `account.account_id`, `None` if the cell has none.
        account_id: Option<u32>,
        /// `sgw_player.player_id`.
        player_id: i32,
        /// Who speaks the Expand dialog: the Banker, or the player's own
        /// entity for a GM session.
        speaker_id: u32,
    },

    /// The player pressed the Expand dialog's button (BV-05). The base
    /// buys one step in one statement, or refuses with a reason.
    Expand {
        /// The player's entity id.
        entity_id: u32,
        /// `account.account_id`, `None` if the cell has none.
        account_id: Option<u32>,
        /// `sgw_player.player_id`.
        player_id: i32,
        /// The offer the dialog showed, taken (one-shot) from the vault
        /// session. The purchase only matches a row still at its size and a
        /// price row still at its price, so a second send for the same offer
        /// is a replay and charges nothing. `None`: the session holds no
        /// offer.
        offer: Option<ExpansionOffer>,
        /// The cell's fresh vault-session verdict, the one a bank move
        /// takes (`vault_access`). Only a `Personal` open verdict may buy.
        vault: VaultAccess,
        /// What asked for the purchase.
        trigger: ExpandTrigger,
    },

    /// A player clicked a Team or Command Banker (bank-vault BV-07). The
    /// cell does not know organization membership, so it asks the base: the
    /// base finds the player's Team or Command, checks membership under the
    /// organization lock, sends the vault's size and contents, and answers
    /// with `BankBaseToCell::OrgVaultGranted`, or refuses with a feedback
    /// line itself. The cell has passed the interact range gate and pinned
    /// the Banker.
    OrgVaultOpen {
        /// The player's entity id.
        entity_id: u32,
        /// `account.account_id` from the cell entity, for the log.
        account_id: Option<u32>,
        /// `sgw_player.player_id` from the cell entity; `None` refuses.
        player_id: Option<i32>,
        /// `Team` or `Command`, from the Banker's template.
        scope: VaultScope,
        /// The pinned Banker.
        banker_id: u32,
        /// Player to Banker distance when the click passed the gate.
        distance: Option<f32>,
        /// The space the player and the Banker are in.
        space_id: u32,
    },
}

/// What asked for a vault expansion, the `trigger` log field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpandTrigger {
    /// The Banker's Expand dialog. It must carry the offer the dialog showed.
    Dialog,
    /// GM `.bankexpand`. There is no dialog, so no offer: the base quotes
    /// the current size and price itself and buys at those.
    GmConsole,
}

impl ExpandTrigger {
    /// The stable `trigger` string.
    pub fn as_str(self) -> &'static str {
        match self {
            ExpandTrigger::Dialog => "dialog",
            ExpandTrigger::GmConsole => "gm_console",
        }
    }
}

impl BankCellToBase {
    /// Short stable name for the `kind` log field.
    pub fn kind(&self) -> &'static str {
        match self {
            BankCellToBase::GmDump { .. } => "gm_dump",
            BankCellToBase::ExpansionQuote { .. } => "expansion_quote",
            BankCellToBase::Expand { .. } => "expand",
            BankCellToBase::OrgVaultOpen { .. } => "org_vault_open",
        }
    }
}
