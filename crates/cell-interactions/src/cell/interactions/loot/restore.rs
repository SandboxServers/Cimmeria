//! Putting a looted item back on its corpse when the grant never happened.
//!
//! `lootItem` takes the item off the corpse before the base writes it to
//! the looter's inventory. When the base refuses the grant (the bag is
//! full, the item cannot be carried, a database error) it answers with
//! `BaseToCellMsg::LootGrantRefused`, and [`handle_loot_grant_refused`] puts
//! the item back and tells the looter. A grant the cell could not even
//! send comes back through [`return_unsent`].

use cimmeria_entity::cell_entity::{LootItem, NpcInteractionType};
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use tokio::sync::mpsc;

use crate::cell::messages::{CellToBaseMsg, GrantRefusal, LootGrantSource};
use crate::cell::space_manager::SpaceManager;

/// Why an item could not go back on its corpse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RestoreMiss {
    /// The corpse entity no longer exists.
    CorpseGone,
    /// The entity is a different body now: it respawned (and maybe died
    /// again) or its template changed while the grant was in flight.
    CorpseChanged,
    /// The corpse already has an entry with that index.
    IndexTaken,
}

impl RestoreMiss {
    fn as_str(self) -> &'static str {
        match self {
            Self::CorpseGone => "corpse_gone",
            Self::CorpseChanged => "corpse_changed",
            Self::IndexTaken => "index_taken",
        }
    }
}

/// Put `item` back on the corpse `source` names. `Ok(true)` when the corpse
/// had lost its loot bit (it was emptied) and got it back, so the caller
/// must broadcast the interaction flags again.
pub(super) fn put_back(
    space_mgr: &mut SpaceManager,
    source: LootGrantSource,
    item: LootItem,
) -> Result<bool, RestoreMiss> {
    let corpse = space_mgr
        .get_entity_mut(source.corpse_id)
        .ok_or(RestoreMiss::CorpseGone)?;
    if corpse.respawn_at != source.corpse_respawn_at
        || corpse.template_id != source.corpse_template_id
        || source.index >= corpse.next_loot_index
    {
        return Err(RestoreMiss::CorpseChanged);
    }
    if corpse.loot.iter().any(|li| li.index == source.index) {
        return Err(RestoreMiss::IndexTaken);
    }
    let at = corpse
        .loot
        .iter()
        .position(|li| li.index > source.index)
        .unwrap_or(corpse.loot.len());
    corpse.loot.insert(at, item);
    let reflagged = corpse.interaction_type_flags & crate::cell::abilities::INT_NORMAL_LOOT == 0;
    if reflagged {
        corpse.interaction_type_flags |= crate::cell::abilities::INT_NORMAL_LOOT;
        corpse.interaction_type = Some(NpcInteractionType::Loot);
    }
    Ok(reflagged)
}

/// The line the looter reads after a refused pickup whose item went back.
fn restored_text(reason: GrantRefusal, container_id: i32) -> &'static str {
    match reason {
        GrantRefusal::ContainerFull if container_id == 15 => {
            "Your crafting bag is full. The item was left on the corpse."
        }
        GrantRefusal::ContainerFull => "Your inventory is full. The item was left on the corpse.",
        GrantRefusal::StorageOnly => "That item cannot be carried. It was left on the corpse.",
        GrantRefusal::NoDatabase | GrantRefusal::DatabaseError => {
            "That item could not be picked up right now. It was left on the corpse."
        }
    }
}

/// The line the looter reads when the item could not go back.
const LOST_TEXT: &str = "That item could not be picked up, and the corpse it was on is gone.";

/// `BaseToCellMsg::LootGrantRefused`: put the item back on its corpse, show
/// it again, and tell the looter why the pickup failed.
pub async fn handle_loot_grant_refused(
    entity_id: u32,
    player_id: i32,
    source: LootGrantSource,
    design_id: i32,
    quantity: i32,
    container_id: i32,
    reason: GrantRefusal,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // The looter's entity id may have been reused by now: only the same
    // player gets the line and the refreshed window.
    let looter = space_mgr
        .get_entity(entity_id)
        .filter(|e| e.player_id == Some(player_id))
        .map(|e| (e.account_id, e.looting_entity == Some(source.corpse_id)));
    let account_id = looter.and_then(|(account_id, _)| account_id);
    let item = LootItem {
        design_id: Some(design_id),
        quantity,
        index: source.index,
    };
    match put_back(space_mgr, source, item) {
        Ok(reflagged) => {
            tracing::info!(
                target: "inventory",
                event = "loot_restored",
                account_id,
                player_id,
                entity_id,
                corpse_id = source.corpse_id,
                index = source.index,
                type_id = design_id,
                qty = quantity,
                container_id,
                reason = reason.as_str(),
                reflagged,
                "loot_restored: the refused item is back on the corpse"
            );
            if reflagged {
                let flags = space_mgr
                    .get_entity(source.corpse_id)
                    .map_or(0, |c| c.interaction_type_flags);
                crate::cell::abilities::send_entity_method(
                    source.corpse_id,
                    crate::mercury::method_idx::INTERACTION_TYPE,
                    (flags as u64).to_le_bytes().to_vec(),
                    tx,
                    space_mgr,
                )
                .await;
            }
            if let Some((_, still_looting)) = looter {
                send_feedback(
                    entity_id,
                    player_id,
                    account_id,
                    restored_text(reason, container_id),
                    tx,
                )
                .await;
                if still_looting {
                    super::send_loot_display(entity_id, source.corpse_id as i32, 0, tx, space_mgr)
                        .await;
                }
            }
        }
        Err(miss) => {
            tracing::warn!(
                target: "inventory",
                event = "loot_restore_failed",
                account_id,
                player_id,
                entity_id,
                corpse_id = source.corpse_id,
                index = source.index,
                type_id = design_id,
                qty = quantity,
                refusal = reason.as_str(),
                reason = miss.as_str(),
                "loot_restore_failed: the refused item could not go back on its corpse and is lost"
            );
            if looter.is_some() {
                send_feedback(entity_id, player_id, account_id, LOST_TEXT, tx).await;
            }
        }
    }
}

/// The grant never reached the base (the channel is closed): put the item
/// straight back. The corpse still holds its loot bit, because the empty-
/// loot clear runs after the send.
pub(super) fn return_unsent(
    entity_id: u32,
    player_id: i32,
    source: LootGrantSource,
    item: LootItem,
    space_mgr: &mut SpaceManager,
) {
    let type_id = item.design_id;
    let qty = item.quantity;
    let restored = put_back(space_mgr, source, item);
    tracing::warn!(
        target: "inventory",
        event = "loot_grant_send_failed",
        player_id,
        entity_id,
        corpse_id = source.corpse_id,
        index = source.index,
        type_id = ?type_id,
        qty,
        restored = restored.is_ok(),
        reason = restored.err().map_or("restored", RestoreMiss::as_str),
        "loot_grant_send_failed: the base channel is closed; the item stays on the corpse"
    );
}

async fn send_feedback(
    entity_id: u32,
    player_id: i32,
    account_id: Option<u32>,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let args = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
    if tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: crate::mercury::method_idx::ON_PLAYER_COMMUNICATION,
            args,
        })
        .await
        .is_err()
    {
        tracing::warn!(
            target: "inventory",
            event = "feedback_send_failed",
            account_id,
            player_id,
            entity_id,
            reason = "send_error",
            "feedback_send_failed: the loot refusal line did not reach the base"
        );
    }
}
