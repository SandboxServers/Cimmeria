//! `BaseToCellMsg::UpdateIgnoreList` handler: store the player's Ignore set
//! on the cell entity, where spatial chat reads it (D-SS15).

use std::collections::HashSet;

use super::super::super::space_manager::SpaceManager;

/// Replace the Ignore set of `entity_id`. A missing entity (the push raced a
/// teardown) is logged and dropped: the next world entry re-seeds it.
pub(super) fn handle(
    entity_id: u32,
    player_id: i32,
    ignore_names: HashSet<String>,
    space_mgr: &mut SpaceManager,
) {
    let count = ignore_names.len();
    match space_mgr.get_entity_mut(entity_id) {
        Some(entity) => {
            let before = entity.ignore_names.len();
            entity.ignore_names = ignore_names;
            tracing::debug!(
                target: "chat",
                event = "chat.ignore_set_applied",
                entity_id,
                player_id,
                before,
                after = count,
                "cell Ignore set replaced"
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
