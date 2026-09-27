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
    ignore_names: HashSet<String>,
    space_mgr: &mut SpaceManager,
) {
    let count = ignore_names.len();
    match space_mgr.get_entity_mut(entity_id) {
        Some(entity) if entity.player_id == Some(player_id) => {
            let before = entity.ignore_names.len();
            entity.ignore_names = ignore_names;
            tracing::debug!(
                target: "chat",
                event = "chat.ignore_set_applied",
                entity_id,
                player_id,
                account_id = entity.account_id,
                before,
                after = count,
                "cell Ignore set replaced"
            );
        }
        Some(entity) => {
            tracing::warn!(
                target: "chat",
                event = "chat.ignore_set_dropped",
                entity_id,
                player_id,
                entity_player_id = entity.player_id,
                account_id = entity.account_id,
                count,
                reason = "player_mismatch",
                "UpdateIgnoreList for an entity id now held by another character; dropped"
            );
        }
        None => {
            tracing::debug!(
                target: "chat",
                event = "chat.ignore_set_dropped",
                entity_id,
                player_id,
                count,
                reason = "entity_missing",
                "UpdateIgnoreList for an entity the cell does not hold; dropped"
            );
        }
    }
}
