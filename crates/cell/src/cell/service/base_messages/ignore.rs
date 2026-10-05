//! `BaseToCellMsg::UpdateIgnoreList` handler: store the player's Ignore set
//! on the cell entity, where spatial chat reads it (D-SS15).
//!
//! The message names both the entity and the `player_id`. Entity ids are
//! recycled and gate travel gives a character a new one, so the set is
//! applied only when the entity still belongs to that player; a push that
//! raced a teardown and landed on a reused id is dropped, not applied to a
//! stranger. Every world entry re-seeds the new entity (`onClientReady`
//! resync on the base), so a dropped push is never the last word.

use std::collections::HashSet;

use super::super::super::space_manager::SpaceManager;

/// Replace the Ignore set of `entity_id` if it is still `player_id`'s.
pub(super) fn handle(
    entity_id: u32,
    player_id: i32,
    account_id: u32,
    version: u64,
    ignore_names: HashSet<String>,
    space_mgr: &mut SpaceManager,
) {
    let count = ignore_names.len();
    match space_mgr.get_entity_mut(entity_id) {
        Some(entity) if entity.player_id == Some(player_id) && version <= entity.ignore_version => {
            let ident = entity.identity();
            tracing::debug!(
                target: "chat",
                event = "chat.ignore_set_dropped",
                entity_id,
                entity_name = ident.player_name,
                player_id,
                player_name = ident.player_name,
                account_id,
                account_name = ident.account_name,
                version,
                applied_version = entity.ignore_version,
                reason = "stale_version",
                "UpdateIgnoreList older than the set the cell holds; dropped"
            );
        }
        Some(entity) if entity.player_id == Some(player_id) => {
            let before = entity.ignore_names.len();
            entity.ignore_names = ignore_names;
            entity.ignore_version = version;
            let ident = entity.identity();
            tracing::debug!(
                target: "chat",
                event = "chat.ignore_set_applied",
                entity_id,
                entity_name = ident.player_name,
                player_id,
                player_name = ident.player_name,
                account_id,
                account_name = ident.account_name,
                before,
                after = count,
                "cell Ignore set replaced"
            );
        }
        Some(entity) => {
            let holder = entity.identity();
            tracing::warn!(
                target: "chat",
                event = "chat.ignore_set_dropped",
                entity_id,
                entity_name = holder.player_name,
                player_id, // nt:id-only the push names the intended character by id only; the cell holds no name for it
                entity_player_id = entity.player_id,
                entity_player_name = holder.player_name,
                entity_account_id = entity.account_id,
                entity_account_name = holder.account_name,
                account_id, // nt:id-only the push carries the intended account id only; the entity now holds another
                count,
                reason = "player_mismatch",
                "UpdateIgnoreList for an entity id now held by another character; dropped"
            );
        }
        None => {
            tracing::debug!(
                target: "chat",
                event = "chat.ignore_set_dropped",
                entity_id, // nt:id-only the cell holds no entity under this id, so there is no name to give
                player_id, // nt:id-only the push names the intended character by id only; the entity is gone
                account_id, // nt:id-only the push carries the intended account id only; the entity is gone
                count,
                reason = "entity_missing",
                "UpdateIgnoreList for an entity the cell does not hold; dropped"
            );
        }
    }
}
