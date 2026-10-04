//! Opening a Team or Command vault (bank-vault BV-07; D-BV09, D-BV12).
//!
//! The cell holds the session and the positions, the base holds
//! organization membership, so an org Banker click is a round trip:
//!
//! 1. [`request_org_vault`]: the Banker arm, after the interact range gate
//!    and the pin, asks the base (`BankCellToBase::OrgVaultOpen`).
//! 2. The base finds the player's Team or Command, checks membership under
//!    the organization lock, sends `onBagInfo` and the vault's contents, logs
//!    `org_vault_opened`, and answers `BankBaseToCell::OrgVaultGranted`. A
//!    refusal (`org_vault_open_rejected`) and its chat line are the base's.
//! 3. [`grant_org_vault`]: the cell re-checks that the player still has that
//!    Banker pinned and in range, records the session, and sends
//!    `onTeamVaultOpen` (107) or `onCommandVaultOpen` (108).
//!
//! The session names the org (`VaultSession::org_id`), but it authorizes
//! nothing on its own: every move re-checks membership and the rank's bank
//! bits under the organization lock on the base.

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::{PlayerIdentity, VaultScope, VaultSession};
use cimmeria_wire::cell::vault::{build_vault_open_args, vault_open_method};

use super::rejection::send_bank_feedback;
use super::{interact_range, InteractRangeFail};
use crate::cell::messages::{BankCellToBase, CellToBaseMsg};
use crate::cell::space_manager::SpaceManager;

/// The Banker arm for a Team or Command Banker: forward the request to the
/// base. The caller has passed the range gate and pinned `banker_id`.
pub(super) async fn request_org_vault(
    entity_id: u32,
    banker_id: u32,
    scope: VaultScope,
    distance: Option<f32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(entity_id);
    let Some(space_id) = space_mgr.get_entity_space_id(entity_id) else {
        // The caller found the player a moment ago; nothing to tell.
        return;
    };
    tracing::debug!(
        target: "bank",
        event = "org_vault_open_requested",
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        entity_id,
        entity_name = id.player_name,
        scope = scope.as_str(),
        banker_id,
        banker_name = space_mgr.entity_label(banker_id),
        space_id,
        world = space_mgr.world_name_for_space(space_id),
        distance,
        "org_vault_open_requested: asking the base for Team/Command membership"
    );
    let request = CellToBaseMsg::Bank(BankCellToBase::OrgVaultOpen {
        entity_id,
        account_id: id.account_id,
        player_id: id.player_id,
        scope,
        banker_id,
        distance,
        space_id,
    });
    if let Err(e) = tx.send(request).await {
        tracing::warn!(
            target: "bank",
            event = "vault_open_send_failed",
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            entity_id,
            entity_name = id.player_name,
            banker_id,
            banker_name = space_mgr.entity_label(banker_id),
            scope = scope.as_str(),
            reason = "base_channel_closed",
            error = %e,
            "vault_open_send_failed: the org vault request could not be queued (base channel \
             closed) -- the player sees no vault window"
        );
    }
}

/// Why the cell did not open a vault the base granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrgGrantReject {
    /// No cell entity for the player any more (logout, gate travel).
    PlayerEntityMissing,
    /// The entity now plays another character.
    StaleEntity,
    /// The player pinned another target while the base was answering.
    BankerNotPinned,
    /// The Banker is gone.
    BankerMissing,
    /// The player walked out of range, or left the space, while the base
    /// was answering.
    OutOfRange,
}

impl OrgGrantReject {
    /// The stable `reason` of `org_vault_open_rejected`.
    pub fn reason(self) -> &'static str {
        match self {
            OrgGrantReject::PlayerEntityMissing => "player_entity_missing",
            OrgGrantReject::StaleEntity => "stale_entity",
            OrgGrantReject::BankerNotPinned => "banker_not_pinned",
            OrgGrantReject::BankerMissing => "banker_missing",
            OrgGrantReject::OutOfRange => "out_of_range",
        }
    }

    /// The line the player sees, when there is a player to tell.
    fn feedback(self) -> Option<&'static str> {
        match self {
            OrgGrantReject::PlayerEntityMissing | OrgGrantReject::StaleEntity => None,
            OrgGrantReject::BankerNotPinned => {
                Some("You turned away before the vault opened. Talk to the Banker again.")
            }
            OrgGrantReject::BankerMissing => {
                Some("That Banker is no longer here. The vault did not open.")
            }
            OrgGrantReject::OutOfRange => Some("You are too far away to use the vault."),
        }
    }
}

/// The base granted `player_id` the vault of `org_id` at `banker_id`
/// (`BankBaseToCell::OrgVaultGranted`). Record the session and show the
/// window, or refuse with `org_vault_open_rejected` and a line.
#[tracing::instrument(
    name = "bank.org_vault_grant",
    level = "info",
    skip_all,
    fields(entity_id, player_id, org_id, banker_id, scope = scope.as_str())
)]
pub async fn grant_org_vault(
    entity_id: u32,
    player_id: i32,
    scope: VaultScope,
    org_id: i32,
    banker_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let grant = Grant {
        entity_id,
        player_id,
        scope,
        org_id,
        banker_id,
    };
    let checked = check_grant(&grant, space_mgr);
    let (banker_pos, distance) = match checked {
        Ok(found) => found,
        Err(reject) => {
            grant.refuse(reject, tx, space_mgr).await;
            return;
        }
    };
    // `check_grant` found the entity in a space.
    let Some(space_id) = space_mgr.get_entity_space_id(entity_id) else {
        return;
    };
    let Some(player) = space_mgr.get_entity_mut(entity_id) else {
        return;
    };
    let id = player.identity();
    player.vault_session = Some(VaultSession {
        expansion_offer: None,
        scope,
        org_id: Some(org_id),
        banker_id: Some(banker_id),
        space_id,
        opened_at: Instant::now(),
    });
    tracing::debug!(
        target: "bank",
        event = "vault_session_opened",
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        entity_id,
        entity_name = id.player_name,
        scope = scope.as_str(),
        org_id, // nt:id-only org names live in the base; the cell holds no org name
        banker_id,
        banker_name = space_mgr.entity_label(banker_id),
        gm_override = false,
        space_id,
        world = space_mgr.world_name_for_space(space_id),
        distance,
        "vault_session_opened: org vault session open, sending the vault window"
    );
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: vault_open_method(scope),
            args: build_vault_open_args(banker_id as i32, banker_pos),
        })
        .await
    {
        tracing::warn!(
            target: "bank",
            event = "vault_open_send_failed",
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            entity_id,
            entity_name = id.player_name,
            banker_id,
            banker_name = space_mgr.entity_label(banker_id),
            org_id, // nt:id-only org names live in the base; the cell holds no org name
            scope = scope.as_str(),
            reason = "base_channel_closed",
            error = %e,
            "vault_open_send_failed: the org vault window could not be queued (base channel \
             closed) -- the session is open but the player sees no window"
        );
    }
}

/// One grant, as the base sent it.
struct Grant {
    entity_id: u32,
    player_id: i32,
    scope: VaultScope,
    org_id: i32,
    banker_id: u32,
}

/// The grant's checks: the same character, the Banker still pinned, and the
/// Banker still within the interact distance in the player's space. Returns
/// the Banker's position and the distance.
fn check_grant(grant: &Grant, space_mgr: &SpaceManager) -> Result<([f32; 3], f32), OrgGrantReject> {
    let player = space_mgr
        .get_entity(grant.entity_id)
        .ok_or(OrgGrantReject::PlayerEntityMissing)?;
    if player.player_id != Some(grant.player_id) {
        return Err(OrgGrantReject::StaleEntity);
    }
    if player.last_interaction_target != Some(grant.banker_id) {
        return Err(OrgGrantReject::BankerNotPinned);
    }
    match interact_range(grant.entity_id, grant.banker_id, space_mgr) {
        Ok(()) => {}
        Err(InteractRangeFail::PlayerMissing) => return Err(OrgGrantReject::PlayerEntityMissing),
        Err(InteractRangeFail::TargetMissing) => return Err(OrgGrantReject::BankerMissing),
        Err(InteractRangeFail::OtherSpace | InteractRangeFail::TooFar { .. }) => {
            return Err(OrgGrantReject::OutOfRange)
        }
    }
    let banker = space_mgr
        .get_entity(grant.banker_id)
        .ok_or(OrgGrantReject::BankerMissing)?
        .position;
    let distance = player.position.distance_squared_to(&banker).sqrt();
    Ok(([banker.x, banker.y, banker.z], distance))
}

impl Grant {
    /// Log `org_vault_open_rejected` and tell the player, when the entity is
    /// still theirs.
    async fn refuse(
        &self,
        reject: OrgGrantReject,
        tx: &mpsc::Sender<CellToBaseMsg>,
        space_mgr: &SpaceManager,
    ) {
        // For a missing or re-used entity the live identity is not this
        // grant's player; log the grant's own player id instead.
        let live = space_mgr.player_identity(self.entity_id);
        let id = match reject {
            OrgGrantReject::PlayerEntityMissing | OrgGrantReject::StaleEntity => {
                PlayerIdentity::new(None, Some(self.player_id))
            }
            _ => live,
        };
        tracing::warn!(
            target: "bank",
            event = "org_vault_open_rejected",
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            entity_id = self.entity_id,
            entity_name = id.player_name,
            org_id = self.org_id, // nt:id-only org names live in the base; the cell holds no org name
            org_type = self.scope.org_type().map(|t| t.name()),
            scope = self.scope.as_str(),
            banker_id = self.banker_id,
            banker_name = space_mgr.entity_label(self.banker_id),
            reason = reject.reason(),
            "org_vault_open_rejected: the base granted the vault but the cell did not open it"
        );
        if let Some(text) = reject.feedback() {
            send_bank_feedback(self.entity_id, text, tx, space_mgr).await;
        }
    }
}
