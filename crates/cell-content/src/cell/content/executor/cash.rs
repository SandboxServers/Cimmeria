//! `Action::GrantCash` (UB-50b1, U-D07): forward one naquadah grant for the
//! acting player to the base as `CellToBaseMsg::GrantCash`. The base
//! (`handle_grant_cash`) updates `sgw_player.naquadah` and sends the client
//! `onCashChanged`. The loot path sends the same message.
//!
//! No debounce: a mission completes once, so the trigger alone prevents a
//! double pay. One firing is exactly one grant.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Grant `amount` naquadah to `player_id` through `entity_id`'s identity.
pub(super) async fn grant(
    amount: i32,
    entity_id: u32,
    player_id: i32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let identity = space_mgr.player_identity(entity_id);
    // A chain fired off a non-player (an NPC's death, a timer on no one)
    // has no naquadah to grant. The executor passes `player_id` 0 there.
    if identity.player_id != Some(player_id) || player_id <= 0 {
        tracing::warn!(
            target: "content",
            event = "content.grant_cash",
            reason = "no_player",
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            player_id,
            player_name = identity.player_name,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            "grant_cash: the chain's entity is not this player; no naquadah granted",
        );
        return;
    }
    if amount <= 0 {
        tracing::warn!(
            target: "content",
            event = "content.grant_cash",
            reason = "bad_amount",
            amount,
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            player_id,
            player_name = identity.player_name,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            "grant_cash: the amount is not positive; no naquadah granted",
        );
        return;
    }
    {
        tracing::info!(
            target: "content",
            event = "content.grant_cash",
            outcome = "requested",
            amount,
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            account_id = identity.account_id,
            account_name = identity.account_name,
            player_id,
            player_name = identity.player_name,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            "Content: granting naquadah",
        );
    }
    if let Err(e) = tx
        .send(CellToBaseMsg::GrantCash {
            entity_id,
            player_id,
            amount,
            gm_feedback_to: None,
        })
        .await
    {
        tracing::warn!(
            target: "content",
            event = "content.grant_cash",
            reason = "base_channel_closed",
            amount,
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            account_id = identity.account_id,
            account_name = identity.account_name,
            player_id,
            player_name = identity.player_name,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            error = %e,
            "grant_cash: cell->base send failed; no naquadah granted",
        );
    }
}
