pub mod data;
pub mod serializers;
pub mod store;

/// Containers that can be operated on by the vendor stack — main bag,
/// bandolier (3), the eleven equipment slots (4..=14), and quick bar (15).
/// Bank, mail attachments, and loot bags are intentionally excluded so
/// vendor sell/repair/recharge can't reach into them.
pub(crate) const VENDOR_FILTER_BAGS: [i32; 14] = [1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
pub mod buyback;
pub mod helpers;
pub mod paid_recharge;
pub mod paid_repair;
pub mod purchase;
pub mod purchase_helpers;
pub mod recharge;
pub mod repair;
pub mod sell;

pub use buyback::handle_buyback_vendor_items;
pub use purchase::handle_purchase_vendor_items;
pub use recharge::handle_recharge_inventory_items;
pub use repair::{handle_repair_inventory_item, handle_repair_inventory_items};
pub use sell::handle_sell_vendor_items;
pub use store::handle_open_vendor_store;
