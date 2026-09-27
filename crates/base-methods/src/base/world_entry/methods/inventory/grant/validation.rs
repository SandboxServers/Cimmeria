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

/// Containers a grant never writes into: the vaults and auction escrow,
/// 17-20.
///
/// Before BV-01 these had no capacity (`bag_max_slots` returned 0), so every
/// grant into them failed at slot reservation. That mattered because loot
/// and content grants pick `container_sets[1]` as the target, and 752 seeded
/// items list 17 first (`{17,15}`). BV-01 gave 17-20 a capacity for
/// `onBagInfo`; this check keeps those grants refused rather than letting
/// loot land in the bank (D-BV04: the bank is storage the player fills).
fn grant_container_refused(container_id: i32) -> bool {
    use cimmeria_entity::inventory::{INV_BANK, INV_COMMAND_BANK};
    (INV_BANK..=INV_COMMAND_BANK).contains(&container_id)
}

/// Refuse a grant into 17-20 ([`grant_container_refused`]) and log it as
/// `grant_rejected` under the `bank` target. Returns `true` when refused.
///
/// A grant names an item *type*, not an instance, so the event carries
/// `type_id` (the grant's `item_id` argument) and no instance `item_id`,
/// source container or slot. The account id is looked up here because the
/// grant path does not carry it; the lookup only runs on a refusal.
pub(super) async fn refuse_storage_grant(
    pool: &Arc<PgPool>,
    entity_id: u32,
    player_id: i32,
    type_id: i32,
    container_id: i32,
    quantity: i32,
) -> bool {
    if !grant_container_refused(container_id) {
        return false;
    }
    let account_id: Option<i32> =
        match sqlx::query_scalar("SELECT account_id FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .fetch_optional(pool.as_ref())
            .await
        {
            Ok(id) => id,
            Err(e) => {
                tracing::warn!(
                    target: "bank",
                    event = "grant_rejected",
                    player_id,
                    entity_id,
                    type_id,
                    target_container_id = container_id,
                    reason = "account_lookup_failed",
                    "grant_rejected: could not read the account id: {e}"
                );
                None
            }
        };
    tracing::warn!(
        target: "bank",
        event = "grant_rejected",
        account_id,
        player_id,
        entity_id,
        type_id,
        quantity,
        target_container_id = container_id,
        reason = "grant_into_storage_container",
        "grant_rejected: grants never write into the vaults or auction escrow"
    );
    true
}
