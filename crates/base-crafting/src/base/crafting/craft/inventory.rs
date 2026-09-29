//! The request-time inventory reads a craft decides on. They take no lock:
//! the completion transaction re-checks everything under its own locks.

use std::collections::HashMap;

use sqlx::PgPool;

use super::rules::NamedInstance;
use crate::base::crafting::transaction::CRAFTING_INPUT_BAGS;

/// The player's own instances among `item_ids`, wherever they sit. An id
/// that is not the player's is simply absent.
pub async fn named_instances(
    pool: &PgPool,
    player_id: i32,
    item_ids: &[i32],
) -> Result<Vec<NamedInstance>, sqlx::Error> {
    let rows: Vec<(i32, i32, i32)> = sqlx::query_as(
        "SELECT item_id, type_id, container_id FROM sgw_inventory \
         WHERE character_id = $1 AND item_id = ANY($2)",
    )
    .bind(player_id)
    .bind(item_ids)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(item_id, type_id, container_id)| NamedInstance {
            item_id,
            type_id,
            container_id,
        })
        .collect())
}

/// The total stack size of each of `type_ids` in the main and crafting
/// bags. A design the player has none of is absent.
pub async fn carried_totals(
    pool: &PgPool,
    player_id: i32,
    type_ids: &[i32],
) -> Result<HashMap<i32, i64>, sqlx::Error> {
    let rows: Vec<(i32, i64)> = sqlx::query_as(
        "SELECT type_id, SUM(stack_size)::int8 FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = ANY($2) AND type_id = ANY($3) \
         GROUP BY type_id",
    )
    .bind(player_id)
    .bind(&CRAFTING_INPUT_BAGS[..])
    .bind(type_ids)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

/// The design's display name, for the player's lines.
pub async fn item_name(pool: &PgPool, design_id: i32) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT name FROM resources.items WHERE item_id = $1")
        .bind(design_id)
        .fetch_optional(pool)
        .await
}
