//! The `query_inventory_items` loader.
//!
//! Extracted from `player_load/core.rs` (issue #529). Reads `sgw_inventory`
//! rows via the shared `INVENTORY_ITEM_SELECT` and maps them to wire-ready
//! `InvItem` structs (DB 0-indexed slot → wire 1-indexed). Pure code movement.

use sqlx::PgPool;

use super::INVENTORY_ITEM_SELECT;

/// Query inventory items from `sgw_inventory` for a character.
///
/// Returns `InvItem` structs ready for wire serialization via `onUpdateItem`.
/// Note: `slot_id` is stored 0-indexed in DB but sent 1-indexed on the wire.
pub async fn query_inventory_items(
    pool: &PgPool,
    player_id: i32,
) -> Vec<cimmeria_entity::inventory::InvItem> {
    #[derive(sqlx::FromRow)]
    struct InvRow {
        item_id: i32,
        type_id: i32,
        stack_size: i32,
        slot_id: i32,
        container_id: i32,
        bound: bool,
        durability: i32,
        charges: i32,
        ammo_type_ids: Vec<i32>,
        cur_ammo_type_id: i32,
    }

    match sqlx::query_as::<_, InvRow>(INVENTORY_ITEM_SELECT)
        .bind(player_id)
        .fetch_all(pool)
        .await
    {
        Ok(rows) => rows
            .into_iter()
            .map(|r| cimmeria_entity::inventory::InvItem {
                id: r.item_id,
                dbid: r.type_id,
                stack_size: r.stack_size,
                slot_id: r.slot_id + 1,
                container_id: r.container_id,
                is_bound: r.bound,
                durability: r.durability,
                ammo_types: r.ammo_type_ids,
                cur_ammo_type: r.cur_ammo_type_id,
                charges: r.charges,
            })
            .collect(),
        Err(e) => {
            tracing::error!(player_id, "Failed to query inventory items: {e}");
            vec![]
        }
    }
}
