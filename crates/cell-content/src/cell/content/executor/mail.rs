//! `Action::SendSystemMail` (SS-U3): forward one system mail for the acting
//! player to the base, which owns the mail writer and the cooldown table.
//!
//! The cell decides nothing about the mail itself: the base claims the
//! cooldown and writes the mail in one transaction, and sends the player
//! the result line (sent, or how long to wait). So one firing is exactly one
//! `CellToBaseMsg::ContentSystemMail`, and the recipient is always the
//! chain's own player, taken from the cell entity.

use cimmeria_content_engine::actions::Action;
use tokio::sync::mpsc;

use crate::cell::messages::{CellToBaseMsg, ContentMailCooldown, ContentSystemMail};
use crate::cell::space_manager::SpaceManager;

/// `sgw_player_content_cooldown.cooldown_key` for a chain's mail. Keyed by
/// chain, so each authored mail has its own window.
pub(in crate::cell::content) fn cooldown_key(chain_id: i64) -> String {
    format!("send_system_mail/{chain_id}")
}

/// `action` is always an [`Action::SendSystemMail`]; the dispatch arm
/// passes it whole.
pub(super) async fn send_system_mail(
    action: Action,
    entity_id: u32,
    player_id: i32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Action::SendSystemMail {
        sender_name,
        subject,
        body,
        cash,
        item,
        cooldown_secs,
    } = action
    else {
        return;
    };
    let identity = space_mgr.player_identity(entity_id);
    // A chain fired off a non-player (an NPC's death, a timer on no one)
    // has nobody to mail. The executor passes `player_id` 0 there.
    if identity.player_id != Some(player_id) || player_id <= 0 {
        tracing::warn!(
            target: "content",
            event = "content.send_system_mail",
            reason = "no_player",
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            player_id,
            player_name = identity.player_name,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            "send_system_mail: the chain's entity is not this player; no mail sent",
        );
        return;
    }
    {
        let names = cimmeria_names::book();
        tracing::info!(
            target: "content",
            event = "content.send_system_mail",
            outcome = "requested",
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            account_id = identity.account_id,
            account_name = identity.account_name,
            player_id,
            player_name = identity.player_name,
            chain_id,
            chain_name = names.chain(chain_id),
            sender_name = %sender_name,
            cash,
            item_type_id = item.map(|(t, _)| t),
            item_name = item.and_then(|(t, _)| names.item(t)),
            quantity = item.map(|(_, q)| q),
            cooldown_secs,
            "send_system_mail: forwarded to the base",
        );
    }
    let msg = ContentSystemMail {
        entity_id,
        player_id,
        account_id: identity.account_id,
        chain_id,
        sender_name,
        subject,
        body,
        cash,
        item,
        cooldown: cooldown_secs.map(|secs| ContentMailCooldown {
            key: cooldown_key(chain_id),
            secs,
        }),
    };
    if let Err(e) = tx.send(CellToBaseMsg::ContentSystemMail(msg)).await {
        tracing::warn!(
            target: "content",
            event = "content.send_system_mail",
            reason = "base_channel_closed",
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            account_id = identity.account_id,
            account_name = identity.account_name,
            player_id,
            player_name = identity.player_name,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            error = %e,
            "send_system_mail: cell->base send failed; no mail sent",
        );
    }
}
