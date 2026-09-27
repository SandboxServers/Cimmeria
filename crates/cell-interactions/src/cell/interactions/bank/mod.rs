//! The Banker: open the vault window, and keep the vault session the bank
//! moves are checked against (bank-vault campaign, BV-02; D-BV03, D-BV05,
//! D-BV09, D-BV10).
//!
//! # Open path
//!
//! A right-click on a Banker reaches the `Banker` arm of `handle_interact`
//! after the outer dispatcher's range gate (`interact_target_in_range`: the
//! same space, within `MAX_INTERACT_DISTANCE`) and its pin. For a personal
//! Banker, [`open_vault_at_banker`] records a [`VaultSession`] pinned to the
//! Banker and sends `onVaultOpen(banker_id, banker_position)` (client method
//! 106) from the cell, the way the trainer opens: container 17's contents
//! were already sent at login and its size is declared by the world-entry
//! `onBagInfo`, so no base round trip is needed (BV-E1 Q1). Team and Command
//! Bankers are refused with a chat line until the org vaults land (Wave 4).
//!
//! GM `.bank` ([`open_vault_gm`]) opens the personal vault anywhere with a
//! session that has no Banker.
//!
//! # Session and moves
//!
//! The client sends nothing when the window closes and ignores
//! `onVaultOpen`'s position (BV-E1 Q4), so the session ends only on server
//! signals: a space change or logout destroys the `CellEntity` holding it,
//! and a later `interact` pin of another target clears it
//! ([`pin_interaction_target`]). [`vault_move_allowed`] is the one rule a
//! bank move must pass, and it re-checks the Banker's proximity every time.
//!
//! Every event logs under the `bank` target, with the D-BV19 names:
//! `vault_session_opened` and `vault_session_closed` (DEBUG) and
//! `vault_open_rejected` (WARN). The info spans `bank.banker_interact` and
//! `bank.console_open` wrap the two entry points.

mod open;
mod rejection;

use cimmeria_entity::cell_entity::VaultCloseReason;

use super::dispatch::{interact_range, InteractRangeFail};
use crate::cell::space_manager::{log_vault_session_closed, SpaceManager};

pub use open::{open_vault_at_banker, open_vault_gm, reject_banker_out_of_range};
pub use rejection::{reject_vault_open, VaultOpenReject};

// The move rule and its reject enum live in `cimmeria-cell-world` so every
// crate that forwards an inventory request can take the verdict (BV-03).
pub use crate::cell::space_manager::{vault_access, vault_move_allowed, VaultReject};

/// Pin `target` as the player's interaction target, ending a vault session
/// pinned elsewhere (D-BV05) with `vault_session_closed reason=re_pin`.
/// Every `interact` pin calls this.
pub fn pin_interaction_target(space_mgr: &mut SpaceManager, entity_id: u32, target: u32) {
    let Some(player) = space_mgr.get_entity_mut(entity_id) else {
        return;
    };
    let identity = player.identity();
    if let Some(ended) = player.pin_interaction_target(target) {
        log_vault_session_closed(entity_id, identity, &ended, VaultCloseReason::RePin);
    }
}

#[cfg(test)]
mod telemetry_tests;
#[cfg(test)]
mod tests;
