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
pub use data::{
    load_store_buy_items, load_vendor_buyback_prices, load_vendor_recharge_prices,
    load_vendor_repair_prices, load_vendor_sell_prices,
};
pub use helpers::{send_cash_changed_to_client, sync_bandolier_after_inventory_change};
pub use paid_recharge::handle_paid_recharge_inventory_items;
pub use paid_repair::handle_paid_repair_inventory_items;
pub use purchase::handle_purchase_vendor_items;
pub use purchase_helpers::{
    consume_design_quantity, load_vendor_purchase_lines, load_vendor_template_lists,
    normalize_item_quantities, VendorPurchaseLine,
};
pub use recharge::handle_recharge_inventory_items;
pub use repair::{handle_repair_inventory_item, handle_repair_inventory_items};
pub use sell::handle_sell_vendor_items;
pub use serializers::{
    free_inventory_slots, reserve_free_inventory_slots, serialize_empty_store_open,
    serialize_store_item_cost_array, serialize_store_open, serialize_store_update, StoreBuyCost,
    StoreItem, StoreItemCost, StoreItemCostUpdate,
};
pub use store::{
    clear_buyback_items, handle_open_vendor_store, send_store_open_to_client,
    send_store_update_to_client, VendorTemplateLists,
};
