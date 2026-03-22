//! SGWInventoryManager interface ClientMethods (indices 69–75).

/// Bag/container info (sizes, types).
pub const ON_BAG_INFO: u16 = 69;
/// Active equipment slot changed.
pub const ON_ACTIVE_SLOT_UPDATE: u16 = 70;
/// Remove item(s) from inventory.
pub const ON_REMOVE_ITEM: u16 = 71;
/// Add or update item(s) in inventory.
pub const ON_UPDATE_ITEM: u16 = 72;
/// Refresh a single item's data.
pub const ON_REFRESH_ITEM: u16 = 73;
/// Clear organization vault inventory.
pub const ON_CLEAR_ORG_VAULT_INVENTORY: u16 = 74;
/// Player cash amount changed.
pub const ON_CASH_CHANGED: u16 = 75;
