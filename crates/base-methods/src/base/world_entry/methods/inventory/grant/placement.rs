//! Which container a grant writes into.
//!
//! Every caller names a container: loot and content take it from the
//! cell's `item_containers` cache, `gmGiveItem` asks for the main bag. The
//! item's own `container_sets` has the last word
//! ([`cimmeria_cell_catalog::item_placement::grant_container`]): a request
//! for a storage container falls through to the first carried bag the item
//! lists (the `{17,15}` crafting components land in the crafting bag), and
//! a carried-bag request the item does not allow moves to the bag it does.

use std::sync::Arc;

use cimmeria_cell_catalog::item_placement::{grant_container, skipped_storage};
use sqlx::PgPool;

/// Where a grant goes, and what was known when that was decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Placement {
    /// The container the caller asked for.
    pub requested: i32,
    /// The container the grant writes into. For a storage-only item this is
    /// the requested storage container, which the vault guard then refuses.
    pub container_id: i32,
    /// The item's `container_sets`, empty when the item has no row.
    pub container_sets: Vec<i32>,
    /// Whether the choice passed over a storage container.
    pub skipped_storage: bool,
    /// The player's account, for the telemetry; `None` when the player row
    /// is missing.
    pub account_id: Option<i32>,
}

/// Read the item's `container_sets` and the player's account id, and choose
/// the container. One round trip; `Err` is a database error.
pub(super) async fn resolve_placement(
    pool: &Arc<PgPool>,
    player_id: i32,
    type_id: i32,
    requested: i32,
) -> Result<Placement, sqlx::Error> {
    let (container_sets, account_id): (Option<Vec<i32>>, Option<i32>) = sqlx::query_as(
        "SELECT (SELECT container_sets FROM resources.items WHERE item_id = $1), \
                (SELECT account_id FROM sgw_player WHERE player_id = $2)",
    )
    .bind(type_id)
    .bind(player_id)
    .fetch_one(pool.as_ref())
    .await?;
    let container_sets = container_sets.unwrap_or_default();
    Ok(place(container_sets, requested, account_id))
}

/// The pure half of [`resolve_placement`].
pub(super) fn place(
    container_sets: Vec<i32>,
    requested: i32,
    account_id: Option<i32>,
) -> Placement {
    let container_id = grant_container(&container_sets, requested).unwrap_or(requested);
    Placement {
        requested,
        container_id,
        skipped_storage: skipped_storage(&container_sets, requested, container_id),
        container_sets,
        account_id,
    }
}

/// `container_sets` as the telemetry writes it: `{17,15}`.
pub(super) fn format_container_sets(container_sets: &[i32]) -> String {
    let inner: Vec<String> = container_sets.iter().map(i32::to_string).collect();
    format!("{{{}}}", inner.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bank_first_component_is_placed_in_the_crafting_bag() {
        for requested in [17, 1, 15] {
            let p = place(vec![17, 15], requested, Some(7));
            assert_eq!(p.container_id, 15, "request {requested}");
            assert!(p.skipped_storage);
        }
    }

    /// A storage-only item keeps the storage request, so the vault guard
    /// sees it and refuses.
    #[test]
    fn storage_only_item_keeps_the_storage_request() {
        let p = place(vec![17], 17, None);
        assert_eq!(p.container_id, 17);
        assert!(!p.skipped_storage);
    }

    #[test]
    fn weapon_requests_are_unchanged() {
        assert_eq!(place(vec![3, 1, 17], 3, None).container_id, 3);
        assert_eq!(place(vec![3, 1, 17], 1, None).container_id, 1);
    }

    #[test]
    fn container_sets_format() {
        assert_eq!(format_container_sets(&[17, 15]), "{17,15}");
        assert_eq!(format_container_sets(&[]), "{}");
    }
}
