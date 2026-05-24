//! Method indices and shared property-id constants for the inventory module.

pub const REMOVE_ITEM: u16 = 36;
pub const LIST_ITEMS: u16 = 37;
pub const MOVE_ITEM: u16 = 38;
pub const USE_ITEM: u16 = 39;
pub const REPAIR_ITEM_REQUEST: u16 = 40;
pub const REQUEST_ACTIVE_SLOT_CHANGE: u16 = 41;
pub const REQUEST_AMMO_CHANGE: u16 = 42;

/// `GENERICPROPERTY_AmmoTypeId` from `entities/defs/enumerations.xml`. Used as
/// the property-id arg for `onEntityProperty` ammo-type indicator updates.
/// Widened from `pub(super)` so the cell-side base-message handlers in
/// [`crate::cell::service::base_messages::bandolier`] can emit it on the
/// equip paths (issue #372 — right-click equip skipped the AmmoTypeId
/// update, leaving the client unable to play the fire animation).
pub(crate) const GENERICPROPERTY_AMMO_TYPE_ID: i32 = 3;

/// Build the `onEntityProperty(propId, value)` arg payload (8 bytes LE).
/// Visibility widened alongside [`GENERICPROPERTY_AMMO_TYPE_ID`] above so
/// the base-message handlers can call it directly instead of duplicating
/// the 8-byte LE pack.
pub(crate) fn build_entity_property_args(prop_id: i32, value: i32) -> Vec<u8> {
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&prop_id.to_le_bytes());
    args.extend_from_slice(&value.to_le_bytes());
    args
}
