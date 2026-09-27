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
//! Every event logs under the `bank` target.

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::{CellEntity, VaultScope, VaultSession};
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use cimmeria_wire::cell::client_methods::player::ON_VAULT_OPEN;
use cimmeria_wire::cell::vault::build_vault_open_args;

use super::dispatch::{interact_range, InteractRangeFail};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

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
/// pinned elsewhere (D-BV05), and log the end. Every `interact` pin calls
/// this.
pub fn pin_interaction_target(space_mgr: &mut SpaceManager, entity_id: u32, target: u32) {
    let Some(player) = space_mgr.get_entity_mut(entity_id) else {
        return;
    };
    if let Some(ended) = player.pin_interaction_target(target) {
        tracing::debug!(
            target: "bank",
            event = "vault_session_cleared",
            entity_id,
            banker_id = ended.banker_id,
            new_target = target,
            open_secs = ended.opened_at.elapsed().as_secs_f32(),
            reason = "repin",
            "vault_session_cleared: another interaction target was pinned"
        );
    }
}

/// The Banker arm of `handle_interact`. The caller has already passed the
/// interact range gate and pinned `banker_id`.
pub async fn open_vault_at_banker(
    entity_id: u32,
    banker_id: u32,
    scope: VaultScope,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if scope != VaultScope::Personal {
        // The org vaults need the organizations campaign (BV-07, Wave 4).
        // Refuse visibly: the click must not look dead.
        tracing::info!(
            target: "bank",
            event = "vault_open_rejected",
            entity_id,
            banker_id,
            scope = scope.as_str(),
            reason = "org_vault_not_available",
            "vault_open_rejected: organization vaults are not available yet"
        );
        let text = match scope {
            VaultScope::Team => "The Team vault is not available yet.",
            _ => "The Command vault is not available yet.",
        };
        send_bank_feedback(entity_id, text, tx).await;
        return;
    }
    let Some(banker_pos) = space_mgr.get_entity(banker_id).map(|b| b.position) else {
        return;
    };
    let pos = [banker_pos.x, banker_pos.y, banker_pos.z];
    open_personal_vault(
        entity_id,
        Some(banker_id),
        banker_id as i32,
        pos,
        tx,
        space_mgr,
    )
    .await;
}

/// GM `.bank`: open the personal vault where the GM stands. The session has
/// no Banker, so moves skip the proximity check; the window is addressed to
/// the GM's own entity and position.
pub async fn open_vault_gm(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(pos) = space_mgr.get_entity(entity_id).map(|e| e.position) else {
        return;
    };
    open_personal_vault(
        entity_id,
        None,
        entity_id as i32,
        [pos.x, pos.y, pos.z],
        tx,
        space_mgr,
    )
    .await;
}

/// Record the session, then send `onVaultOpen(window_entity, window_pos)`.
async fn open_personal_vault(
    entity_id: u32,
    banker_id: Option<u32>,
    window_entity: i32,
    window_pos: [f32; 3],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(space_id) = space_mgr.get_entity_space_id(entity_id) else {
        return;
    };
    let Some(player) = space_mgr.get_entity_mut(entity_id) else {
        return;
    };
    player.vault_session = Some(VaultSession {
        scope: VaultScope::Personal,
        banker_id,
        space_id,
        opened_at: Instant::now(),
    });

    let sent = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_VAULT_OPEN,
            args: build_vault_open_args(window_entity, window_pos),
        })
        .await;
    match sent {
        Ok(()) => tracing::info!(
            target: "bank",
            event = "vault_open",
            entity_id,
            banker_id,
            space_id,
            scope = "personal",
            gm = banker_id.is_none(),
            "vault_open: sent onVaultOpen"
        ),
        Err(e) => tracing::warn!(
            target: "bank",
            event = "vault_open_send_failed",
            entity_id,
            banker_id,
            error = %e,
            "vault_open: onVaultOpen could not be queued (base channel closed)"
        ),
    }
}

/// A single-recipient `SYSTEM` line on the feedback channel, the same shape
/// the console and the chat channel refusals use. `onErrorCode` alone is not
/// enough: no client Lua consumes it (AT-E1 §2).
pub(crate) async fn send_bank_feedback(
    entity_id: u32,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_PLAYER_COMMUNICATION,
            args: serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text),
        })
        .await
    {
        tracing::warn!(
            target: "bank",
            event = "bank_feedback_send_failed",
            entity_id,
            error = %e,
            "bank feedback line could not be queued (base channel closed)"
        );
    }
}

#[cfg(test)]
mod tests;
