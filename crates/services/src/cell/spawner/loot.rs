//! Loot tables and item-side caches.
//!
//! `LootTableEntry` rows back the NPC death loot rolls. `WeaponDef` and
//! the item→container mapping are item-side caches keyed by `item_id` and
//! consumed by the content engine's `GrantItem` action so runtime grants
//! seed bandolier slots / land in the right inventory bag.

use sqlx::PgPool;

/// A single entry in a loot table, loaded from `resources.loot`.
#[derive(Debug, Clone)]
pub struct LootTableEntry {
    pub design_id: Option<i32>,
    pub min_quantity: i32,
    pub max_quantity: i32,
    pub probability: f32,
}

/// Load loot tables from the database.
///
/// Returns `loot_table_id → Vec<LootTableEntry>` so that loot generation at
/// NPC death can roll drops without per-kill DB queries.
///
/// Reference: `python/cell/interactions/Lootable.py:randomizeLoot()`
pub async fn load_loot_tables(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, Vec<LootTableEntry>>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT loot_table_id, design_id, min_quantity, max_quantity, probability \
         FROM resources.loot \
         ORDER BY loot_table_id, loot_id",
    )
    .fetch_all(pool)
    .await?;

    let mut map: std::collections::HashMap<i32, Vec<LootTableEntry>> =
        std::collections::HashMap::new();
    for r in &rows {
        let table_id: i32 = r.get("loot_table_id");
        let entry = LootTableEntry {
            design_id: r.get("design_id"),
            min_quantity: r.get("min_quantity"),
            max_quantity: r.get("max_quantity"),
            probability: r.get("probability"),
        };
        map.entry(table_id).or_default().push(entry);
    }

    tracing::info!(
        tables = map.len(),
        entries = map.values().map(|v| v.len()).sum::<usize>(),
        "Loaded loot tables"
    );
    Ok(map)
}

/// Load item → preferred container mappings from `resources.items.container_sets`.
///
/// The `container_sets` column is a PostgreSQL `integer[]`. We pick the first
/// element as the preferred container for runtime grants (mission items → 2,
/// weapons → 3, etc.). Items with an empty array are omitted — they default
/// to INV_Main (1) at the call site.
pub async fn load_item_containers(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, i32>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT item_id, container_sets[1] AS container_id \
         FROM resources.items \
         WHERE array_length(container_sets, 1) > 0",
    )
    .fetch_all(pool)
    .await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for r in &rows {
        let item_id: i32 = r.get("item_id");
        let container_id: i32 = r.get("container_id");
        map.insert(item_id, container_id);
    }

    tracing::info!(count = map.len(), "Loaded item container mappings");
    Ok(map)
}

/// Cached weapon stats for runtime item grants.
///
/// The content engine's `GrantItem` action seeds bandolier slots and AmmoSlot
/// stats from this cache so the client renders the correct empty magazine for
/// the new weapon. Mirrors the per-row clip_size + default_ammo_type that
/// `BANDOLIER_ITEMS_QUERY` reads for player_load.
///
/// `allowed_ammo_types` is the whitelist of subtypes a player may pick via
/// `requestAmmoChange` for this weapon — empty means "no choice; default
/// only". Mirrors `resources.items.ammo_types` converted to 0-based indices.
#[derive(Debug, Clone)]
pub struct WeaponDef {
    pub clip_size: i32,
    /// 0-based EAmmoType index, matching the wire format used in
    /// `bandolier_items` and `onEntityProperty(AmmoTypeId)`.
    pub default_ammo_type: i32,
    /// 0-based EAmmoType indices the player may pick via requestAmmoChange.
    /// Empty Vec means "no allowed subtypes" (default ammo type only).
    pub allowed_ammo_types: Vec<i32>,
}

/// Load weapon stats (clip_size + default_ammo_type + allowed ammo subtypes)
/// from `resources.items`.
///
/// Skips items with `clip_size IS NULL` (non-weapons). The `default_ammo_type`
/// conversion mirrors `BANDOLIER_ITEMS_QUERY` exactly so that values seeded
/// here for runtime grants match the wire format the player_load path uses.
/// `ammo_types` is unnested and each enum entry converted via the same
/// `array_position` trick, then aggregated back to an `integer[]` array.
pub async fn load_item_defs(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, WeaponDef>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT item_id, clip_size, \
                CASE WHEN default_ammo_type IS NULL THEN 0 \
                     ELSE array_position(enum_range(NULL::resources.\"EAmmoType\"), default_ammo_type) - 1 \
                END AS default_ammo_type_id, \
                COALESCE( \
                    (SELECT array_agg((array_position(enum_range(NULL::resources.\"EAmmoType\"), e) - 1)::integer) \
                     FROM unnest(ammo_types) AS e), \
                    '{}'::integer[] \
                ) AS allowed_ammo_type_ids \
         FROM resources.items \
         WHERE clip_size IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for r in &rows {
        let item_id: i32 = r.get("item_id");
        let clip_size: i32 = r.get("clip_size");
        let default_ammo_type: i32 = r.get("default_ammo_type_id");
        let allowed_ammo_types: Vec<i32> = r.get("allowed_ammo_type_ids");
        map.insert(
            item_id,
            WeaponDef {
                clip_size,
                default_ammo_type,
                allowed_ammo_types,
            },
        );
    }

    tracing::info!(count = map.len(), "Loaded weapon defs");
    Ok(map)
}
