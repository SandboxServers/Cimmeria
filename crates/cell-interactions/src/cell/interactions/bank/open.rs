//! Opening the personal vault: at a Banker, and for GM `.bank`.
//!
//! A successful open records the [`VaultSession`], logs the `bank`
//! `vault_session_opened` event (DEBUG), sends `onVaultOpen` (106) and
//! asks the base for an expansion quote (BV-05).
//! Every refusal goes through [`super::reject_vault_open`].

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::{NpcInteractionType, VaultScope, VaultSession};
use cimmeria_wire::cell::client_methods::player::ON_VAULT_OPEN;
use cimmeria_wire::cell::vault::build_vault_open_args;

use super::expand::request_expansion_quote;
use super::org_open::request_org_vault;
use super::rejection::{reject_vault_open, VaultOpenReject};
use super::{interact_range, InteractRangeFail};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// The Banker arm of `handle_interact`. The caller has already passed the
/// interact range gate and pinned `banker_id`.
#[tracing::instrument(
    name = "bank.banker_interact",
    level = "info",
    skip_all,
    fields(entity_id, banker_id, scope = scope.as_str())
)]
pub async fn open_vault_at_banker(
    entity_id: u32,
    banker_id: u32,
    scope: VaultScope,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(banker_pos) = space_mgr.get_entity(banker_id).map(|b| b.position) else {
        reject_vault_open(
            entity_id,
            VaultOpenReject::BankerMissing,
            Some(banker_id),
            None,
            tx,
            space_mgr,
        )
        .await;
        return;
    };
    let distance = space_mgr
        .get_entity(entity_id)
        .map(|p| p.position.distance_squared_to(&banker_pos).sqrt());

    if scope != VaultScope::Personal {
        // Membership lives on the base: ask it (BV-07). The window opens
        // when the base's grant comes back (`org_open::grant_org_vault`).
        request_org_vault(entity_id, banker_id, scope, distance, tx, space_mgr).await;
        return;
    }
    let pos = [banker_pos.x, banker_pos.y, banker_pos.z];
    open_personal_vault(
        entity_id,
        Some(banker_id),
        distance,
        banker_id as i32,
        pos,
        tx,
        space_mgr,
    )
    .await;
}

/// A click on `target` failed the interact range gate. If `target` is a
/// Banker, refuse visibly with `vault_open_rejected reason=out_of_range`
/// and return `true`; any other NPC is left to the caller's usual silent
/// drop (`false`).
///
/// The distance is reported for a same-space miss; a Banker in another
/// space has no meaningful distance, so the field is omitted.
#[tracing::instrument(
    name = "bank.banker_interact",
    level = "info",
    skip_all,
    fields(entity_id, banker_id = target)
)]
pub async fn reject_banker_out_of_range(
    entity_id: u32,
    target: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> bool {
    let is_banker = space_mgr
        .get_entity(target)
        .is_some_and(|t| matches!(t.interaction_type, Some(NpcInteractionType::Banker { .. })));
    if !is_banker {
        return false;
    }
    let distance = match interact_range(entity_id, target, space_mgr) {
        Err(InteractRangeFail::TooFar { dist }) => Some(dist),
        _ => None,
    };
    reject_vault_open(
        entity_id,
        VaultOpenReject::OutOfRange,
        Some(target),
        distance,
        tx,
        space_mgr,
    )
    .await;
    true
}

/// GM `.bank`: open the personal vault where the GM stands. The session has
/// no Banker, so moves skip the proximity check; the window is addressed to
/// the GM's own entity and position. Returns whether the vault opened.
pub async fn open_vault_gm(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(pos) = space_mgr.get_entity(entity_id).map(|e| e.position) else {
        log_player_entity_missing(entity_id, None);
        return false;
    };
    open_personal_vault(
        entity_id,
        None,
        None,
        entity_id as i32,
        [pos.x, pos.y, pos.z],
        tx,
        space_mgr,
    )
    .await
}

/// Record the session, log `vault_session_opened`, then send
/// `onVaultOpen(window_entity, window_pos)`.
async fn open_personal_vault(
    entity_id: u32,
    banker_id: Option<u32>,
    distance: Option<f32>,
    window_entity: i32,
    window_pos: [f32; 3],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(space_id) = space_mgr.get_entity_space_id(entity_id) else {
        log_player_entity_missing(entity_id, banker_id);
        return false;
    };
    // `get_entity_space_id` resolved it, so the entity exists.
    let Some(player) = space_mgr.get_entity_mut(entity_id) else {
        return false;
    };
    let id = player.identity();
    player.vault_session = Some(VaultSession {
        scope: VaultScope::Personal,
        org_id: None,
        banker_id,
        space_id,
        opened_at: Instant::now(),
        expansion_offer: None,
    });
    tracing::debug!(
        target: "bank",
        event = "vault_session_opened",
        account_id = id.account_id,
        player_id = id.player_id,
        entity_id,
        scope = VaultScope::Personal.as_str(),
        banker_id,
        gm_override = banker_id.is_none(),
        space_id,
        distance,
        "vault_session_opened: vault session open, sending onVaultOpen"
    );

    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_VAULT_OPEN,
            args: build_vault_open_args(window_entity, window_pos),
        })
        .await
    {
        tracing::warn!(
            target: "bank",
            event = "vault_open_send_failed",
            account_id = id.account_id,
            player_id = id.player_id,
            entity_id,
            banker_id,
            reason = "base_channel_closed",
            error = %e,
            "vault_open_send_failed: onVaultOpen could not be queued (base channel closed) -- \
             the session is open but the player sees no vault window"
        );
    }
    // BV-05: offer the next expansion. A GM session's dialog speaks
    // through the GM's own entity.
    request_expansion_quote(entity_id, banker_id.unwrap_or(entity_id), tx, space_mgr).await;
    true
}

/// Negative log for a lookup miss on the player's own entity. Not a
/// `bank` event: the D-BV19 catalog fixes `vault_open_rejected`'s reasons
/// to the four player-visible refusals, and this one has no player to tell
/// (the entity is in no space). It logs under this crate's own target,
/// which `OTEL_FILTER` exports at DEBUG and above.
fn log_player_entity_missing(entity_id: u32, banker_id: Option<u32>) {
    // Under `bank`, like every other open refusal, so a support query on
    // the target finds it. No `account_id` / `player_id`: they are read from
    // the entity that is missing.
    tracing::warn!(
        target: "bank",
        event = "vault_open_rejected",
        entity_id,
        banker_id,
        reason = "player_entity_missing",
        "bank: vault open found no cell entity for the player -- no vault session and no \
         window; the player may hold stale state"
    );
}
