// World entry and player load data functions, split into focused modules.
// All functions previously in world_entry_player.rs are now distributed here.
// Organized into semantic groups: player_load/, inventory/, vendor/ for maintainability.

// Renamed from `world_entry.rs` to avoid name-collision with the parent
// `world_entry/` module after the split-and-consolidate refactor.
pub mod inventory;
pub mod mail;
pub mod missions;
pub mod player_load;
pub mod progression;
pub mod vendor;
pub mod world_entry_db;

// Re-export all public functions for backward compatibility
pub use inventory::{
    handle_grant_item, handle_move_inventory_item, handle_remove_inventory_item,
    handle_remove_inventory_item_by_type, handle_use_inventory_item, send_full_inventory_update,
};
pub use mail::handle_mail_request;
pub use missions::{handle_mission_update, query_saved_missions};
pub use player_load::{default_player_load_data, query_player_load_data};
pub use progression::{handle_grant_cash, handle_grant_xp};
pub use vendor::{
    handle_buyback_vendor_items, handle_open_vendor_store, handle_purchase_vendor_items,
    handle_recharge_inventory_items, handle_repair_inventory_item, handle_repair_inventory_items,
    handle_sell_vendor_items,
};
pub use world_entry_db::{query_world_entry, query_world_stargates};
