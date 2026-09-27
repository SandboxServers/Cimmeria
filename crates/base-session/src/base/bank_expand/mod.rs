//! Buying vault space (bank-vault BV-05, D-BV02): the base half.
//!
//! The personal vault starts at 40 slots and grows in steps of 10 up to
//! 100, each step bought at a Banker for the price in
//! `resources.bank_expansion_price`. The cell owns the vault session and
//! the Expand dialog (`cimmeria-cell-interactions` `bank/expand.rs`); the
//! base owns `sgw_player.bank_slots` and the cash, so it answers two
//! messages:
//!
//! - [`handle_expansion_quote`] (`BankCellToBase::ExpansionQuote`): a vault
//!   opened. Below the ceiling, tell the cell the current size and the next
//!   step's price, so it can record the offer and show the dialog.
//! - [`handle_expand`] (`BankCellToBase::Expand`): the player pressed the
//!   button. Check the cell's fresh verdict and the offer, then buy in one
//!   statement ([`persist::persist_expansion`]). On success, re-declare the
//!   vault's size with `onBagInfo` (69), send the new balance with
//!   `onCashChanged` (75) and confirm in chat. Every refusal gets a chat line
//!   too, so the press is acknowledged even with the vault window closed.
//!
//! Telemetry (D-BV19, target `bank`): INFO `expand` with the size and the
//! cash before and after; WARN `expand_rejected` with a stable `reason`,
//! the size and the cash; DEBUG/WARN `expand_quote` for the offer. Every row
//! carries `account_id`, `player_id` and `entity_id`, plus `banker_id` or
//! `gm_override` from the verdict.

pub mod persist;
mod sends;

#[cfg(test)]
mod quote_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod wire_tests;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::messages::{BankBaseToCell, BaseToCellMsg};
use cimmeria_wire::cell::vault::{VaultAccess, VAULT_EXPAND_STEP};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::ConnectedClientState;
use persist::{
    persist_expansion, read_expansion_state, ExpandOutcome, ExpansionState, VAULT_CEILING,
};
pub use sends::vault_resize_bag_info_args;
use sends::Client;

/// The player the message is about, from the cell's session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpandCaller {
    pub entity_id: u32,
    pub account_id: Option<u32>,
    pub player_id: i32,
}

/// Why a purchase bought nothing. [`ExpandRefusal::reason`] is the stable
/// `reason=` of `expand_rejected`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpandRefusal {
    /// The cell's verdict refused the personal vault: the label of
    /// `VaultAccess::personal_vault_refusal` (`no_vault_session`,
    /// `banker_out_of_range`, `banker_gone`, `banker_other_space`,
    /// `vault_session_other_space`, `player_missing`, `vault_scope_mismatch`).
    Vault(&'static str),
    /// The session holds no offer: the dialog was answered after the vault
    /// was reopened, or twice.
    NoOffer,
    /// The offer's size is no longer the vault's: already bought.
    Replay,
    /// The vault is at 100 slots.
    AtCeiling,
    /// Not enough naquadah.
    InsufficientCash,
    /// No price row for the next step.
    PriceMissing,
    /// No `sgw_player` row.
    PlayerRowMissing,
    /// The base has no database pool.
    DbUnavailable,
    /// A query failed.
    QueryFailed,
}

impl ExpandRefusal {
    /// The stable `reason=` string.
    pub fn reason(self) -> &'static str {
        match self {
            ExpandRefusal::Vault(label) => label,
            ExpandRefusal::NoOffer => "no_offer",
            ExpandRefusal::Replay => "replay",
            ExpandRefusal::AtCeiling => "at_ceiling",
            ExpandRefusal::InsufficientCash => "insufficient_cash",
            ExpandRefusal::PriceMissing => "price_missing",
            ExpandRefusal::PlayerRowMissing => "player_row_missing",
            ExpandRefusal::DbUnavailable => "db_unavailable",
            ExpandRefusal::QueryFailed => "query_failed",
        }
    }

    /// The chat line the player sees. `price` is the next step's price when
    /// it is known.
    pub fn feedback(self, price: Option<i32>) -> String {
        match self {
            ExpandRefusal::Vault("banker_out_of_range") => {
                "You are too far from the Banker. Your vault was not expanded.".to_string()
            }
            ExpandRefusal::Vault(_) | ExpandRefusal::NoOffer => {
                "Talk to a Banker again to expand your vault. Nothing was charged.".to_string()
            }
            ExpandRefusal::Replay => {
                "That expansion was already handled. Nothing more was charged.".to_string()
            }
            ExpandRefusal::AtCeiling => {
                format!("Your vault is already at its full size of {VAULT_CEILING} slots.")
            }
            ExpandRefusal::InsufficientCash => match price {
                Some(p) => {
                    format!("You need {p} naquadah to expand your vault. Nothing was charged.")
                }
                None => "You do not have enough naquadah to expand your vault.".to_string(),
            },
            ExpandRefusal::PriceMissing
            | ExpandRefusal::PlayerRowMissing
            | ExpandRefusal::DbUnavailable
            | ExpandRefusal::QueryFailed => {
                "Your vault could not be expanded right now. Nothing was charged.".to_string()
            }
        }
    }
}

/// What a refusal log knows about the vault and the cash.
#[derive(Debug, Clone, Copy, Default)]
struct Snapshot {
    bank_slots: Option<i16>,
    cash: Option<i32>,
    price: Option<i32>,
}

impl From<ExpansionState> for Snapshot {
    fn from(s: ExpansionState) -> Self {
        Snapshot {
            bank_slots: Some(s.bank_slots),
            cash: Some(s.naquadah),
            price: s.next_price,
        }
    }
}

/// `BankCellToBase::Expand`: buy one step, or refuse with a reason.
#[tracing::instrument(
    name = "bank.expand_purchase",
    level = "info",
    skip_all,
    fields(entity_id = caller.entity_id, player_id = caller.player_id)
)]
pub async fn handle_expand(
    caller: ExpandCaller,
    from_slots: Option<i16>,
    vault: VaultAccess,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let client = Client {
        caller,
        transport,
        connected,
        entity_to_addr,
    };
    let Some(pool) = db_pool.as_deref() else {
        reject(
            &client,
            ExpandRefusal::DbUnavailable,
            &vault,
            from_slots,
            Snapshot::default(),
            None,
        )
        .await;
        return;
    };

    // The verdict and the offer first: neither needs the database, but the
    // refusal log reads the vault and the cash so it is answerable alone.
    let early = match (vault.personal_vault_refusal(), from_slots) {
        (Some(label), _) => Some(ExpandRefusal::Vault(label)),
        (None, None) => Some(ExpandRefusal::NoOffer),
        (None, Some(_)) => None,
    };
    if let Some(refusal) = early {
        let snapshot = match read_expansion_state(pool, caller.player_id).await {
            Ok(state) => state.map(Snapshot::from).unwrap_or_default(),
            Err(_) => Snapshot::default(),
        };
        reject(&client, refusal, &vault, from_slots, snapshot, None).await;
        return;
    }
    let from = from_slots.unwrap_or_default();

    match persist_expansion(pool, caller.player_id, from).await {
        Ok(ExpandOutcome::Expanded {
            bank_slots_after,
            cash_after,
            price,
        }) => {
            tracing::info!(
                target: "bank",
                event = "expand",
                account_id = caller.account_id,
                player_id = caller.player_id,
                entity_id = caller.entity_id,
                bank_slots_before = bank_slots_after - VAULT_EXPAND_STEP,
                bank_slots_after,
                price,
                cash_before = cash_after + price,
                cash_after,
                banker_id = vault.banker_id(),
                gm_override = vault.gm_override(),
                distance = vault.distance(),
                "expand: vault expanded"
            );
            client.send_vault_size(bank_slots_after).await;
            client.send_cash(cash_after).await;
            let line =
                format!("Your vault now has {bank_slots_after} slots. You paid {price} naquadah.");
            client.send_line(&line).await;
        }
        Ok(outcome) => {
            let (refusal, snapshot) = refusal_of(outcome);
            reject(&client, refusal, &vault, from_slots, snapshot, None).await;
        }
        Err(e) => {
            let error = e.to_string();
            reject(
                &client,
                ExpandRefusal::QueryFailed,
                &vault,
                from_slots,
                Snapshot::default(),
                Some(&error),
            )
            .await;
        }
    }
}

/// Map a non-purchase outcome to its refusal and what it read.
fn refusal_of(outcome: ExpandOutcome) -> (ExpandRefusal, Snapshot) {
    match outcome {
        ExpandOutcome::Replay { state } => (ExpandRefusal::Replay, state.into()),
        ExpandOutcome::AtCeiling { state } => (ExpandRefusal::AtCeiling, state.into()),
        ExpandOutcome::PriceMissing { state } => (ExpandRefusal::PriceMissing, state.into()),
        ExpandOutcome::InsufficientCash { state, price } => (
            ExpandRefusal::InsufficientCash,
            Snapshot {
                price: Some(price),
                ..state.into()
            },
        ),
        ExpandOutcome::PlayerMissing => (ExpandRefusal::PlayerRowMissing, Snapshot::default()),
        ExpandOutcome::Expanded { .. } => unreachable!("a purchase is not a refusal"),
    }
}

/// Log `expand_rejected` and tell the player.
async fn reject(
    client: &Client<'_>,
    refusal: ExpandRefusal,
    vault: &VaultAccess,
    from_slots: Option<i16>,
    snapshot: Snapshot,
    error: Option<&str>,
) {
    let caller = client.caller;
    tracing::warn!(
        target: "bank",
        event = "expand_rejected",
        account_id = caller.account_id,
        player_id = caller.player_id,
        entity_id = caller.entity_id,
        reason = refusal.reason(),
        offered_slots = from_slots,
        bank_slots = snapshot.bank_slots,
        cash = snapshot.cash,
        price = snapshot.price,
        banker_id = vault.banker_id(),
        gm_override = vault.gm_override(),
        distance = vault.distance(),
        error,
        "expand_rejected: nothing bought -- the player sees a chat line saying why"
    );
    client.send_line(&refusal.feedback(snapshot.price)).await;
}

/// `BankCellToBase::ExpansionQuote`: below the ceiling, send the cell the
/// offer; at the ceiling, tell the player the vault is full.
#[tracing::instrument(
    name = "bank.expansion_quote",
    level = "info",
    skip_all,
    fields(entity_id = caller.entity_id, player_id = caller.player_id)
)]
pub async fn handle_expansion_quote(
    caller: ExpandCaller,
    speaker_id: u32,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let client = Client {
        caller,
        transport,
        connected,
        entity_to_addr,
    };
    let state = match db_pool.as_deref() {
        None => Err(("db_unavailable", None)),
        Some(pool) => match read_expansion_state(pool, caller.player_id).await {
            Ok(Some(state)) => Ok(state),
            Ok(None) => Err(("player_row_missing", None)),
            Err(e) => Err(("query_failed", Some(e.to_string()))),
        },
    };
    let state = match state {
        Ok(state) => state,
        Err((reason, error)) => {
            quote_warn(caller, reason, None, error.as_deref());
            return;
        }
    };
    if state.bank_slots >= VAULT_CEILING {
        quote_debug(caller, &state, false, Some("at_ceiling"));
        client
            .send_line(&ExpandRefusal::AtCeiling.feedback(None))
            .await;
        return;
    }
    let Some(price) = state.next_price else {
        quote_warn(caller, "price_missing", Some(&state), None);
        return;
    };
    let msg = BaseToCellMsg::Bank(BankBaseToCell::OfferExpansion {
        entity_id: caller.entity_id,
        player_id: caller.player_id,
        speaker_id,
        from_slots: state.bank_slots,
        price,
    });
    let sent = match cell_tx {
        Some(tx) => tx.send(msg).await.is_ok(),
        None => false,
    };
    if sent {
        quote_debug(caller, &state, true, None);
    } else {
        quote_warn(caller, "cell_channel_closed", Some(&state), None);
    }
}

fn quote_debug(caller: ExpandCaller, state: &ExpansionState, offered: bool, reason: Option<&str>) {
    tracing::debug!(
        target: "bank",
        event = "expand_quote",
        account_id = caller.account_id,
        player_id = caller.player_id,
        entity_id = caller.entity_id,
        offered,
        reason,
        bank_slots = state.bank_slots,
        cash = state.naquadah,
        price = state.next_price,
        "expand_quote: expansion offer decided"
    );
}

fn quote_warn(
    caller: ExpandCaller,
    reason: &str,
    state: Option<&ExpansionState>,
    error: Option<&str>,
) {
    tracing::warn!(
        target: "bank",
        event = "expand_quote",
        account_id = caller.account_id,
        player_id = caller.player_id,
        entity_id = caller.entity_id,
        offered = false,
        reason,
        bank_slots = state.map(|s| s.bank_slots),
        cash = state.map(|s| s.naquadah),
        error,
        "expand_quote: no Expand dialog offered -- the vault opened but cannot be expanded now"
    );
}
