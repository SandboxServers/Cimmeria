// World entry and player load data functions, split into focused modules.
// All functions previously in world_entry_player.rs are now distributed here.
// Organized into semantic groups: player_load/, inventory/, vendor/ for maintainability.

// Renamed from `world_entry.rs` to avoid name-collision with the parent
// `world_entry/` module after the split-and-consolidate refactor.
pub mod world_entry_db;
pub mod progression;
pub mod missions;
pub mod mail;
pub mod player_load;
pub mod inventory;
pub mod vendor;

// Re-export all public functions for backward compatibility
pub use world_entry_db::{query_world_entry, query_world_stargates};
pub use player_load::{query_player_load_data, query_player_load_data_by_account, query_inventory_items, default_player_load_data, query_bandolier_items, query_archetype_ability_tree};
pub use progression::{handle_grant_xp, handle_grant_cash};
pub use missions::{query_saved_missions, handle_mission_update};
pub use mail::handle_mail_request;
pub use inventory::{handle_grant_item, item_allows_container, normalize_item_ids, send_full_inventory_update, handle_remove_inventory_item, handle_use_inventory_item, handle_move_inventory_item};
pub use vendor::{
    serialize_store_open, serialize_empty_store_open, serialize_store_item_cost_array,
    serialize_store_update, free_inventory_slots, reserve_free_inventory_slots,
    handle_open_vendor_store, clear_buyback_items, send_store_open_to_client, send_store_update_to_client,
    load_store_buy_items, load_vendor_sell_prices, load_vendor_buyback_prices,
    load_vendor_repair_prices, load_vendor_recharge_prices,
    load_vendor_template_lists, load_vendor_purchase_lines, consume_design_quantity, normalize_item_quantities,
    handle_purchase_vendor_items, handle_sell_vendor_items, handle_buyback_vendor_items,
    handle_repair_inventory_item, handle_repair_inventory_items,
    handle_paid_repair_inventory_items,
    handle_recharge_inventory_items, handle_paid_recharge_inventory_items,
    send_cash_changed_to_client, sync_bandolier_after_inventory_change,
};
