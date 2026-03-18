//! Vendor stock loading and caching.
//!
//! NPC vendors have buy/sell item lists defined by their `entity_templates` row.
//! We load these once from the database and cache them for the lifetime of the
//! CellService so repeated store opens don't hit the DB.
//!
//! Wire format for `onStoreOpen` (flat index 109):
//! ```text
//! entityId:i32, vendorType:i32,
//! buyList:ARRAY<StoreItem>, sellList:ARRAY<StoreItem>,
//! buybackList:ARRAY<StoreItem>, repairList:ARRAY<StoreItem>,
//! rechargeList:ARRAY<StoreItem>
//! ```
//!
//! Each `StoreItem` in the array is serialized as:
//! ```text
//! itemTypeId:i32, quantity:i32, unitPrice:i32
//! ```
//!
//! Reference: `python/cell/SGWPlayer.py` onStoreOpen, `python/base/SGWVendor.py`

use std::collections::HashMap;
use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::RwLock;

/// A single item in a vendor's buy or sell list.
#[derive(Debug, Clone)]
pub struct VendorItem {
    /// Item type ID from the items table.
    pub item_type_id: i32,
    /// Stack quantity available (-1 = unlimited).
    pub quantity: i32,
    /// Unit price in currency. 0 = use item's base price.
    pub unit_price: i32,
}

/// Pre-loaded stock for a single vendor template.
#[derive(Debug, Clone, Default)]
pub struct VendorStock {
    /// Items the vendor sells to the player.
    pub buy_list: Vec<VendorItem>,
    /// Items the vendor will buy from the player (empty = buys anything).
    pub sell_list: Vec<VendorItem>,
}

impl VendorStock {
    /// Serialize the buy list into the `onStoreOpen` wire format fragment.
    ///
    /// Format: `count:u32, [itemTypeId:i32, quantity:i32, unitPrice:i32] * count`
    pub fn serialize_buy_list(&self) -> Vec<u8> {
        serialize_item_array(&self.buy_list)
    }

    /// Serialize the sell list into the `onStoreOpen` wire format fragment.
    pub fn serialize_sell_list(&self) -> Vec<u8> {
        serialize_item_array(&self.sell_list)
    }
}

/// Serialize a list of vendor items as a wire-format array.
fn serialize_item_array(items: &[VendorItem]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(4 + items.len() * 12);
    buf.extend_from_slice(&(items.len() as u32).to_le_bytes());
    for item in items {
        buf.extend_from_slice(&item.item_type_id.to_le_bytes());
        buf.extend_from_slice(&item.quantity.to_le_bytes());
        buf.extend_from_slice(&item.unit_price.to_le_bytes());
    }
    buf
}

/// Cache of vendor stocks keyed by template_id.
///
/// Wrapped in `Arc<RwLock>` so the cell loop can read without blocking and
/// the initial load can populate it once.
#[derive(Debug, Clone)]
pub struct VendorCache {
    inner: Arc<RwLock<HashMap<i32, VendorStock>>>,
}

impl VendorCache {
    /// Create an empty vendor cache.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get cached stock for a template_id. Returns None if not loaded.
    pub async fn get(&self, template_id: i32) -> Option<VendorStock> {
        self.inner.read().await.get(&template_id).cloned()
    }

    /// Insert stock for a template_id.
    pub async fn insert(&self, template_id: i32, stock: VendorStock) {
        self.inner.write().await.insert(template_id, stock);
    }

    /// Load vendor stock for a single template from the database and cache it.
    ///
    /// If already cached, returns the cached version without querying.
    pub async fn load_or_get(
        &self,
        pool: &PgPool,
        template_id: i32,
    ) -> Result<VendorStock, sqlx::Error> {
        // Fast path: already cached
        if let Some(stock) = self.get(template_id).await {
            return Ok(stock);
        }

        // Slow path: query DB
        let stock = load_vendor_stock(pool, template_id).await?;
        self.insert(template_id, stock.clone()).await;
        Ok(stock)
    }

    /// Look up the buy price for a given design_id across all cached vendors.
    ///
    /// Returns the first matching `unit_price` from any vendor's buy list.
    /// This is a simple linear scan — fine for the small number of vendors
    /// typical in a zone.
    pub async fn find_buy_price(&self, design_id: i32) -> Option<i32> {
        let inner = self.inner.read().await;
        for stock in inner.values() {
            for item in &stock.buy_list {
                if item.item_type_id == design_id {
                    return Some(item.unit_price);
                }
            }
        }
        None
    }

    /// Look up the sell price for a given design_id across all cached vendors.
    ///
    /// Sell lists may be empty (vendor buys anything). When a vendor has no
    /// sell list entry for an item, a default sell ratio is applied by the caller.
    pub async fn find_sell_price(&self, design_id: i32) -> Option<i32> {
        let inner = self.inner.read().await;
        for stock in inner.values() {
            for item in &stock.sell_list {
                if item.item_type_id == design_id {
                    return Some(item.unit_price);
                }
            }
        }
        None
    }

    /// Bulk-load all vendor stocks referenced by the given template IDs.
    ///
    /// Call this at startup after spawning DB NPCs to pre-warm the cache.
    pub async fn preload(
        &self,
        pool: &PgPool,
        template_ids: &[i32],
    ) -> Result<usize, sqlx::Error> {
        let mut loaded = 0;
        for &tid in template_ids {
            // Skip if already cached
            if self.inner.read().await.contains_key(&tid) {
                continue;
            }
            match load_vendor_stock(pool, tid).await {
                Ok(stock) => {
                    if !stock.buy_list.is_empty() || !stock.sell_list.is_empty() {
                        tracing::debug!(template_id = tid, buy = stock.buy_list.len(), sell = stock.sell_list.len(), "Cached vendor stock");
                        loaded += 1;
                    }
                    self.insert(tid, stock).await;
                }
                Err(e) => {
                    tracing::warn!(template_id = tid, "Failed to load vendor stock: {e}");
                }
            }
        }
        if loaded > 0 {
            tracing::info!(loaded, "Vendor stocks pre-loaded");
        }
        Ok(loaded)
    }
}

/// Query the database for a vendor's buy and sell item lists.
///
/// The `entity_templates` table has `buy_item_list` and `sell_item_list` columns
/// that reference item list tables. Each list maps to a set of (item_type_id,
/// quantity, unit_price) tuples.
///
/// TODO: The exact table structure for item lists needs verification. Using
/// placeholder table/column names that should be adjusted to match the real schema.
pub async fn load_vendor_stock(
    pool: &PgPool,
    template_id: i32,
) -> Result<VendorStock, sqlx::Error> {
    use sqlx::Row;

    // Step 1: Get the buy_item_list and sell_item_list IDs from the template
    let template_row = sqlx::query(
        "SELECT buy_item_list, sell_item_list \
         FROM resources.entity_templates \
         WHERE template_id = $1"
    )
    .bind(template_id)
    .fetch_optional(pool)
    .await?;

    let (buy_list_id, sell_list_id): (Option<i32>, Option<i32>) = match template_row {
        Some(row) => (row.get("buy_item_list"), row.get("sell_item_list")),
        None => {
            tracing::debug!(template_id, "No template found for vendor stock");
            return Ok(VendorStock::default());
        }
    };

    // Step 2: Load items from each list
    // Load items from resources.item_list_items (keyed by item_list_id)
    let buy_list = if let Some(list_id) = buy_list_id {
        load_item_list(pool, list_id).await?
    } else {
        Vec::new()
    };

    let sell_list = if let Some(list_id) = sell_list_id {
        load_item_list(pool, list_id).await?
    } else {
        Vec::new()
    };

    Ok(VendorStock {
        buy_list,
        sell_list,
    })
}

/// Load items from a single item list by list_id.
///
/// Load items for a vendor list from `resources.item_list_items`.
///
/// Schema: item_list_items(item_id, item_list_id, design_id, quantity, naquadah)
async fn load_item_list(
    pool: &PgPool,
    list_id: i32,
) -> Result<Vec<VendorItem>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT design_id, quantity, naquadah \
         FROM resources.item_list_items \
         WHERE item_list_id = $1 \
         ORDER BY design_id"
    )
    .bind(list_id)
    .fetch_all(pool)
    .await?;

    let items = rows.into_iter().map(|r| {
        VendorItem {
            item_type_id: r.get("design_id"),
            quantity: r.get("quantity"),
            unit_price: r.get("naquadah"),
        }
    }).collect();

    Ok(items)
}

/// Build the `onStoreOpen` argument payload from vendor stock.
///
/// Wire format: `entityId:i32, vendorType:i32, buyList:ARRAY, sellList:ARRAY,
///               buybackList:ARRAY, repairList:ARRAY, rechargeList:ARRAY`
pub fn build_store_open_args(npc_entity_id: i32, stock: &VendorStock) -> Vec<u8> {
    let buy_bytes = stock.serialize_buy_list();
    let sell_bytes = stock.serialize_sell_list();
    let empty_array = 0u32.to_le_bytes();

    let total = 4 + 4 + buy_bytes.len() + sell_bytes.len() + 3 * 4;
    let mut args = Vec::with_capacity(total);
    args.extend_from_slice(&npc_entity_id.to_le_bytes());   // EntityId
    args.extend_from_slice(&1i32.to_le_bytes());             // VendorType (1 = general)
    args.extend_from_slice(&buy_bytes);                      // buyList
    args.extend_from_slice(&sell_bytes);                     // sellList
    args.extend_from_slice(&empty_array);                    // buybackList (always empty from server)
    args.extend_from_slice(&empty_array);                    // repairList (TODO)
    args.extend_from_slice(&empty_array);                    // rechargeList (TODO)
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_empty_stock() {
        let stock = VendorStock::default();
        let args = build_store_open_args(42, &stock);
        // 4 (entityId) + 4 (vendorType) + 5 * 4 (five empty arrays) = 28
        assert_eq!(args.len(), 28);
        assert_eq!(i32::from_le_bytes([args[0], args[1], args[2], args[3]]), 42);
        assert_eq!(i32::from_le_bytes([args[4], args[5], args[6], args[7]]), 1);
        // All 5 array counts = 0
        for i in 0..5 {
            let off = 8 + i * 4;
            assert_eq!(u32::from_le_bytes([args[off], args[off+1], args[off+2], args[off+3]]), 0);
        }
    }

    #[test]
    fn serialize_stock_with_items() {
        let stock = VendorStock {
            buy_list: vec![
                VendorItem { item_type_id: 100, quantity: -1, unit_price: 50 },
                VendorItem { item_type_id: 200, quantity: 10, unit_price: 120 },
            ],
            sell_list: vec![
                VendorItem { item_type_id: 300, quantity: 1, unit_price: 0 },
            ],
        };
        let args = build_store_open_args(999, &stock);

        // entityId(4) + vendorType(4) + buy(4 + 2*12) + sell(4 + 1*12) + 3 empty(3*4) = 64
        assert_eq!(args.len(), 64);

        // Buy list count = 2
        let buy_off = 8;
        assert_eq!(u32::from_le_bytes([args[buy_off], args[buy_off+1], args[buy_off+2], args[buy_off+3]]), 2);

        // First buy item: type=100
        let item_off = buy_off + 4;
        assert_eq!(i32::from_le_bytes([args[item_off], args[item_off+1], args[item_off+2], args[item_off+3]]), 100);
        // quantity=-1
        assert_eq!(i32::from_le_bytes([args[item_off+4], args[item_off+5], args[item_off+6], args[item_off+7]]), -1);
        // price=50
        assert_eq!(i32::from_le_bytes([args[item_off+8], args[item_off+9], args[item_off+10], args[item_off+11]]), 50);

        // Sell list count = 1
        let sell_off = buy_off + 4 + 2 * 12;
        assert_eq!(u32::from_le_bytes([args[sell_off], args[sell_off+1], args[sell_off+2], args[sell_off+3]]), 1);
    }

    #[test]
    fn serialize_item_array_empty() {
        let buf = serialize_item_array(&[]);
        assert_eq!(buf, &[0, 0, 0, 0]);
    }

    #[test]
    fn serialize_item_array_roundtrip() {
        let items = vec![
            VendorItem { item_type_id: 1, quantity: 5, unit_price: 10 },
        ];
        let buf = serialize_item_array(&items);
        // count(4) + 1 item(12) = 16
        assert_eq!(buf.len(), 16);
        assert_eq!(u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]), 1);
        assert_eq!(i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]), 1);
        assert_eq!(i32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]), 5);
        assert_eq!(i32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]), 10);
    }

    #[test]
    fn vendor_cache_new_is_empty() {
        let cache = VendorCache::new();
        // Just verify it constructs without panic
        let _ = format!("{:?}", cache);
    }
}
