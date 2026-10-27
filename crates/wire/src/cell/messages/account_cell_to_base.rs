//! `AccountCellToBase`: the GM `.account` commands from the cell to the
//! base, carried by `CellToBaseMsg::Account`.
//!
//! The cell has no DB pool, so it forwards the command and the base reads
//! the identity tables and answers on the GM's feedback channel. The ids
//! come from the cell's own session state, never from a client payload.
//! Codes are never carried back or logged.

/// The GM `.account` commands (client-tls-oidc C10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountCellToBase {
    /// `.account identities <account>`: list the account's sign-in methods.
    Identities {
        /// The GM's entity id; the answer is addressed to it.
        entity_id: u32,
        /// The GM's `account.account_id`, `None` if the cell has none.
        account_id: Option<u32>,
        /// The target account name the GM typed.
        account_name: String,
    },
    /// `.account unlink <account> <identity_id>`: remove one sign-in method.
    Unlink {
        /// The GM's entity id; the answer is addressed to it.
        entity_id: u32,
        /// The GM's `account.account_id`, `None` if the cell has none.
        account_id: Option<u32>,
        /// The target account name the GM typed.
        account_name: String,
        /// The identity row to remove, as the `identities` listing shows it.
        identity_id: i64,
    },
    /// `.account linkcode <account>`: issue a one-time link code.
    LinkCode {
        /// The GM's entity id; the answer is addressed to it.
        entity_id: u32,
        /// The GM's `account.account_id`, `None` if the cell has none.
        account_id: Option<u32>,
        /// The target account name the GM typed.
        account_name: String,
    },
}
