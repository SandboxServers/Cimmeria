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
pub(super) const GENERICPROPERTY_AMMO_TYPE_ID: i32 = 3;

/// Build the `onEntityProperty(propId, value)` arg payload (8 bytes LE).
pub(super) fn build_entity_property_args(prop_id: i32, value: i32) -> Vec<u8> {
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&prop_id.to_le_bytes());
    args.extend_from_slice(&value.to_le_bytes());
    args
}
