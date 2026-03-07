use serde::{Deserialize, Serialize};

/// A vendor item listing with price information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorListing {
    pub item_template_id: i32,
    pub buy_price: i64,
    pub sell_price: i64,
    pub stock: Option<u32>,
}

/// Vendor interaction state for a player browsing an NPC shop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorSession {
    pub vendor_entity_id: i32,
    pub player_entity_id: i32,
    pub item_list_id: i32,
    pub listings: Vec<VendorListing>,
}

impl VendorSession {
    /// Open a vendor session, loading the item list from the database.
    pub fn open(_vendor_entity_id: i32, _player_entity_id: i32, _item_list_id: i32) -> Self {
        todo!("VendorSession::open - load item_list_items + item_list_prices from DB")
    }

    /// Player buys an item from the vendor.
    pub fn buy_item(&mut self, _item_template_id: i32, _quantity: u32) -> VendorResult {
        todo!("VendorSession::buy_item - check money, stock, add to inventory")
    }

    /// Player sells an item to the vendor.
    pub fn sell_item(&mut self, _inventory_slot: usize) -> VendorResult {
        todo!("VendorSession::sell_item - remove from inventory, add money")
    }
}

/// Result of a vendor transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VendorResult {
    Success,
    NotEnoughMoney,
    OutOfStock,
    InventoryFull,
    ItemNotSellable,
}
