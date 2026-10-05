//! `BaseToCellMsg::Org` handler: organization traffic from the base.
//!
//! The base forwards the squad forms of its organization methods here
//! (D-ORG03: squads live on the cell), with the actor's ids from its own
//! session. The squad handlers re-check that the entity is still that
//! character before acting. Later packets add their handlers to this file
//! rather than to `mod.rs`.
//!
//! The handlers are `cimmeria-cell-interactions`' `cell::organization`, not
//! the org plugin's: there is no base-message seam for a plugin until the
//! envelope of `docs/architecture/plugin-architecture.md` §3.4 lands.

use cimmeria_entity::cell_entity::VaultCloseReason;
use cimmeria_entity::organization::OrgLeaveReason;
use tokio::sync::mpsc;

use super::super::super::messages::{CellToBaseMsg, OrgBaseToCell};
use super::super::super::organization::{creation, squad};
use super::super::super::space_manager::SpaceManager;

/// Handle one organization message from the base.
pub(super) async fn handle(
    msg: OrgBaseToCell,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    match msg {
        OrgBaseToCell::SquadInvite {
            player_id,
            entity_id,
            target_name,
        } => {
            squad::handle_invite(player_id, entity_id, &target_name, tx, space_mgr).await;
        }
        OrgBaseToCell::SquadKick {
            player_id,
            entity_id,
            org_id,
            target_name,
        } => {
            squad::handle_kick(player_id, entity_id, org_id, &target_name, tx, space_mgr).await;
        }
        OrgBaseToCell::OrgMembershipEnded {
            player_id,
            entity_id,
            org_id,
            reason,
        } => membership_ended(player_id, entity_id, org_id, reason, space_mgr),
        // ORG-05: creation.
        OrgBaseToCell::RegistrarEligible {
            player_id,
            entity_id,
            npc_entity_id,
            org_type,
        } => {
            creation::on_registrar_eligible(
                player_id,
                entity_id,
                npc_entity_id,
                org_type,
                tx,
                space_mgr,
            )
            .await;
        }
        OrgBaseToCell::CreateResult {
            player_id,
            entity_id,
            created,
            ..
        } => creation::on_create_result(player_id, entity_id, created, space_mgr),
    }
}

/// An online player stopped being a member of a Team or Command (ORG-06's
/// Bank hook, sent by the base beside every `onOrganizationLeft` [36]).
///
/// Logs DEBUG `org.membership_ended`, then ends a Team or Command vault
/// session of that organization the player still has open
/// (`vault_session_closed`, `reason = org_left`; bank-vault BV-07), so the
/// window's next move is refused at the cell as `no_vault_session`. The base
/// would refuse it anyway, since every move re-checks membership under the
/// organization lock; this makes the refusal immediate and visible in the
/// log. A personal session, or another organization's, is left alone.
/// ORG-07 sends the message on a kick too.
fn membership_ended(
    player_id: i32,
    entity_id: u32,
    org_id: i32,
    reason: OrgLeaveReason,
    space_mgr: &mut SpaceManager,
) {
    let live = space_mgr.player_identity(entity_id);
    // The slot may have been recycled since the base sent this; its names
    // are the leaver's only while the live character is the same one.
    let same_player = live.player_id == Some(player_id);
    let player_name = live.player_name.filter(|_| same_player);
    let account_name = live.account_name.filter(|_| same_player);
    tracing::debug!(
        target: "org",
        event = "org.membership_ended",
        account_id = live.account_id,
        account_name,
        player_id,
        player_name,
        entity_id,
        entity_name = player_name,
        org_id, // nt:id-only the base message carries the id only; no org row is loaded on the cell
        reason = reason.as_u8(),
        live_entity = live.player_id == Some(player_id),
        "a player left a Team or Command"
    );
    if live.player_id != Some(player_id) {
        return;
    }
    let org_session = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.vault_session.as_ref())
        .is_some_and(|s| s.org_id == Some(org_id));
    if org_session {
        space_mgr.end_vault_session(entity_id, VaultCloseReason::OrgLeft);
    }
}
