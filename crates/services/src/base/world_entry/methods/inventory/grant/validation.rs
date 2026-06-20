//! Grant-path input validation helpers.
//!
//! Extracted from `grant/mod.rs` (issue #529): `normalize_item_ids` (pure
//! id-list normalization, reused by the vendor repair/recharge paths) and
//! `item_allows_container` (the container-placement gate the move/grant paths
//! consult). Pure code movement.

use std::sync::Arc;

use sqlx::PgPool;

/// Normalize item ID array: remove dupes, sort, filter invalid IDs.
pub fn normalize_item_ids(mut item_ids: Vec<i32>) -> Vec<i32> {
    item_ids.retain(|id| *id > 0);
    item_ids.sort_unstable();
    item_ids.dedup();
    item_ids
}

/// Check if an item type can be placed in a container.
///
/// Returns `false` on DB error rather than silently defaulting to "main bag" —
/// the caller can decide whether to abort the operation or try a fallback.
///
/// Default rule (no `container_sets` configured for the item type): only the
/// main inventory bag (container 1) is allowed.
pub async fn item_allows_container(pool: &Arc<PgPool>, type_id: i32, container_id: i32) -> bool {
    let result = sqlx::query_scalar::<_, Option<Vec<i32>>>(
        "SELECT container_sets FROM resources.items WHERE item_id = $1",
    )
    .bind(type_id)
    .fetch_optional(pool.as_ref())
    .await;

    let container_sets: Option<Vec<i32>> = match result {
        Ok(row) => row.flatten(),
        Err(e) => {
            tracing::error!(
                type_id,
                container_id,
                "item_allows_container query failed: {e}"
            );
            return false;
        }
    };

    match container_sets {
        Some(sets) if !sets.is_empty() => sets.contains(&container_id),
        // Either the item type has no row, or `container_sets` is NULL/empty —
        // fall back to allowing only the main bag.
        _ => container_id == 1,
    }
}
