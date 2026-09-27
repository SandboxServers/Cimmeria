//! `MailGmCellToBase`: the GM mail tools (SS-U1), carried by
//! `CellToBaseMsg::MailGm`.
//!
//! A nested enum in its own file (the organizations pattern, work-packets.md
//! § Messages), so the GM tools never touch `MailOp`, which the mail packets
//! SS-M1 to SS-M3 own. Every variant is sent by the cell only after its
//! `.`-console gate has confirmed the caller's server-side access level is
//! GameMaster or higher, and carries the GM's ids from the cell's own
//! `CellEntity`, never from a client payload.

/// The GM who ran the command, as the cell knows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MailGmActor {
    /// The GM's entity id, for the feedback line.
    pub entity_id: u32,
    /// The GM's `sgw_player.player_id`.
    pub player_id: i32,
    /// The GM's `account.account_id`, `None` if the cell has none.
    pub account_id: Option<u32>,
}

/// GM mail traffic from the cell to the base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MailGmCellToBase {
    /// `.mail [to <name>] [cash <n>] [item <typeId> [qty]] [cod <n>] [<subject>]`.
    ///
    /// The cell has parsed and range-checked the arguments
    /// (`cash >= 0`, `qty >= 1`, `cod > 0`, COD only with an item and
    /// without gift cash). The base resolves `to`, mints the cash and the
    /// item, and writes the mail: without COD through `send_system_mail`,
    /// with COD as a mail from the GM's own character, so the payment comes
    /// back to the GM.
    Send {
        actor: MailGmActor,
        /// The recipient's name as typed; `None` sends to the GM.
        to: Option<String>,
        /// Gift naquadah, minted.
        cash: i64,
        /// `(type_id, quantity)`, minted.
        item: Option<(i32, i32)>,
        /// The COD price, if any.
        cod: Option<i32>,
        subject: String,
    },
    /// `.mailbox [name]`: a summary of one mailbox, the GM's own by default.
    Mailbox {
        actor: MailGmActor,
        name: Option<String>,
    },
}

impl MailGmCellToBase {
    /// Short stable name for the `kind` log field.
    pub fn kind(&self) -> &'static str {
        match self {
            MailGmCellToBase::Send { .. } => "send",
            MailGmCellToBase::Mailbox { .. } => "mailbox",
        }
    }

    /// The GM who sent it.
    pub fn actor(&self) -> MailGmActor {
        match self {
            MailGmCellToBase::Send { actor, .. } | MailGmCellToBase::Mailbox { actor, .. } => {
                *actor
            }
        }
    }
}
