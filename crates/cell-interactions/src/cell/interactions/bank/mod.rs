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

use cimmeria_entity::cell_entity::{CellEntity, VaultCloseReason};

use super::dispatch::{interact_range, InteractRangeFail};
use crate::cell::space_manager::{log_vault_session_closed, SpaceManager};

pub use open::{open_vault_at_banker, open_vault_gm, reject_banker_out_of_range};
pub use rejection::{reject_vault_open, VaultOpenReject};

/// Why a bank move is refused. The label ([`VaultReject::reason`]) is the
/// `reason` field BV-03 logs with `move_rejected`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VaultReject {
    /// No vault window is open.
    NoSession,
    /// The session was opened in another space than the player is in now.
    /// A space change destroys the entity that held it, so this is a
    /// belt-and-braces check.
    SessionInOtherSpace,
    /// The pinned Banker no longer exists (despawned).
    BankerGone,
    /// The pinned Banker is in another space than the player.
    BankerInOtherSpace,
    /// The player walked out of the interact distance of the Banker.
    BankerOutOfRange {
        /// The distance, in world units.
        dist: f32,
    },
    /// The player entity is not in any space.
    PlayerMissing,
}

impl VaultReject {
    /// Stable `reason` label for the `bank` log target.
    pub fn reason(self) -> &'static str {
        match self {
            VaultReject::NoSession => "no_vault_session",
            VaultReject::SessionInOtherSpace => "vault_session_other_space",
            VaultReject::BankerGone => "banker_gone",
            VaultReject::BankerInOtherSpace => "banker_other_space",
            VaultReject::BankerOutOfRange { .. } => "banker_out_of_range",
            VaultReject::PlayerMissing => "player_missing",
        }
    }
}

/// May `player` move an item into or out of its vault right now?
///
/// Pure: no logging, no sends. The rule (D-BV05):
/// - a vault session must be open, opened in the space the player is in;
/// - with a Banker (`banker_id` is `Some`), the Banker must still exist, be
///   in the player's space, and be within `MAX_INTERACT_DISTANCE`: the same
///   [`interact_range`] rule the opening `interact` passed;
/// - a GM `.bank` session (`banker_id` is `None`) skips the proximity check.
///
/// It does not look at `session.scope`; the caller knows which container it
/// is moving and checks the scope that container needs.
pub fn vault_move_allowed(
    player: &CellEntity,
    space_mgr: &SpaceManager,
) -> Result<(), VaultReject> {
    let session = player
        .vault_session
        .as_ref()
        .ok_or(VaultReject::NoSession)?;
    if i64::from(session.space_id) != i64::from(player.space_id.0) {
        return Err(VaultReject::SessionInOtherSpace);
    }
    let Some(banker_id) = session.banker_id else {
        return Ok(());
    };
    let player_id = player.entity_id.0 as u32;
    interact_range(player_id, banker_id, space_mgr).map_err(|fail| match fail {
        InteractRangeFail::PlayerMissing => VaultReject::PlayerMissing,
        InteractRangeFail::TargetMissing => VaultReject::BankerGone,
        InteractRangeFail::OtherSpace => VaultReject::BankerInOtherSpace,
        InteractRangeFail::TooFar { dist } => VaultReject::BankerOutOfRange { dist },
    })
}

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
