//! The native consumable round trip: `ConsumeItemForUse` (cell to base,
//! carried by `CellToBaseMsg::ConsumeItemForUse`) and `ItemUseConsumed`
//! (base to cell, carried by `BaseToCellMsg::ItemUseConsumed`).
//!
//! An item whose `items_event_sets` event-5 ability is a real consumable
//! (a slappack heal, a stimpack buff) is paid for before it takes effect:
//! the cell decides the use is allowed (the target is alive, the pool is
//! not already full) and asks the base to take one unit off the clicked
//! stack; the base does that in one locked transaction and only then tells
//! the cell to apply the ability. So a double-click on the last unit, or a
//! redelivered `ItemUsed`, consumes one item and applies one effect: the
//! second consume finds no row and nothing comes back.
//!
//! Neither message carries an ability or effect id. The cell re-derives the
//! ability from `items_event_sets` and the `type_id` the base read from the
//! row it consumed, so no client-supplied number ever picks what is applied.

use crate::cell::vault::VaultAccess;

/// Take one unit off an inventory instance so its use can take effect.
#[derive(Debug, Clone, PartialEq)]
pub struct ConsumeItemForUse {
    /// The user's entity, for the client inventory update and the answer.
    pub entity_id: u32,
    /// The user's `sgw_player.player_id` (owner check on the row).
    pub player_id: i32,
    /// The inventory row (`sgw_inventory.item_id`) the player clicked.
    pub instance_id: i32,
    /// The design id the cell resolved the use for. The base refuses a row
    /// of any other type, so the effect always matches the item paid.
    pub type_id: i32,
    /// The cell's vault-session verdict: an item in the vault (17) is
    /// consumable only with a session open (BV-03).
    pub vault: VaultAccess,
}

/// The base took one unit of `type_id` off `instance_id`, committed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemUseConsumed {
    pub entity_id: u32,
    pub player_id: i32,
    pub instance_id: i32,
    /// The consumed row's design id, read under the row lock.
    pub type_id: i32,
}
