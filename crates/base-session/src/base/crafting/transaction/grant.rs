//! The output half of the crafting transaction: place each product in the
//! first carried bag its `container_sets` allow, merging into a stack with
//! room or taking free slots.

use std::collections::HashSet;

use cimmeria_cell_catalog::item_placement::first_player_container;
use cimmeria_resources::base::resources::{bag_max_slots, bag_min_slot};
use sqlx::{Postgres, Transaction};

use super::failure::{at, expect_rows};
use super::{CraftApplied, CraftTxError, GrantedStack, CRAFTING_INPUT_BAGS};
use crate::base::crafting::feedback::CraftReject;
use crate::base::crafting::telemetry::JobIds;

/// Where one product goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Placement {
    pub design_id: i32,
    pub quantity: i32,
    pub container_id: i32,
    pub max_stack: i32,
}

#[derive(sqlx::FromRow)]
struct ItemRow {
    container_sets: Vec<i32>,
    max_stack_size: i32,
}

/// Resolve every product's bag and stack size. A product with no carried
/// bag refuses the transaction; an unknown design is a data error.
pub(super) async fn resolve(
    tx: &mut Transaction<'_, Postgres>,
    grants: &[(i32, i32)],
) -> Result<Vec<Placement>, CraftTxError> {
    let mut placements = Vec::with_capacity(grants.len());
    for &(design_id, quantity) in grants {
        if quantity <= 0 {
            continue;
        }
        let row: Option<ItemRow> = sqlx::query_as(
            "SELECT container_sets, max_stack_size FROM resources.items WHERE item_id = $1",
        )
        .bind(design_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(at("resolve"))?;
        let Some(row) = row else {
            return Err(CraftTxError::Invalid {
                phase: "resolve",
                reason: "unknown_product",
            });
        };
        let Some(container_id) = first_player_container(&row.container_sets) else {
            return Err(CraftReject::NoCarriedBagForProduct { design_id }.into());
        };
        placements.push(Placement {
            design_id,
            quantity,
            container_id,
            max_stack: row.max_stack_size.max(1),
        });
    }
    Ok(placements)
}

/// The advisory-lock key the inventory move path takes for the whole
/// player before any per-bag lock.
const PLAYER_WIDE_LOCK: i32 = 0;

/// Take every advisory lock the transaction needs before it locks any row:
/// first the player-wide lock the move path takes, then the
/// per-(player, container) lock the grant, vendor and move paths take, for
/// the input bags and every product bag, in container order.
///
/// Holding the player-wide lock serializes a completion with the player's
/// own moves between the main and crafting bags; without it a move
/// (target bag, then source bag) and a completion (bags in order) can each
/// hold the bag the other waits for. Taking every advisory lock before the
/// player row and the inventory rows keeps the order of the grant and
/// trade paths (advisory lock first), so they wait instead of
/// deadlocking.
pub(super) async fn lock_containers(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
    placements: &[Placement],
) -> Result<(), CraftTxError> {
    let mut containers: Vec<i32> = placements.iter().map(|p| p.container_id).collect();
    containers.extend_from_slice(&CRAFTING_INPUT_BAGS);
    containers.sort_unstable();
    containers.dedup();
    for key in std::iter::once(PLAYER_WIDE_LOCK).chain(containers) {
        sqlx::query("SELECT pg_advisory_xact_lock($1, $2)")
            .bind(player_id)
            .bind(key)
            .execute(&mut **tx)
            .await
            .map_err(at("lock"))?;
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct MergeRow {
    item_id: i32,
    slot_id: i32,
    stack_size: i32,
}

/// Grant one product. The whole quantity merges into one unbound stack
/// with room for all of it (the grant path's all-or-nothing rule);
/// otherwise it takes as many free slots as full stacks need. Too few free
/// slots refuses the transaction.
pub(super) async fn place(
    tx: &mut Transaction<'_, Postgres>,
    ids: &JobIds,
    p: Placement,
    applied: &mut CraftApplied,
) -> Result<(), CraftTxError> {
    let player_id = ids.player_id;
    if p.max_stack > 1 {
        let merge: Option<MergeRow> = sqlx::query_as(
            "SELECT item_id, slot_id, stack_size FROM sgw_inventory \
             WHERE character_id = $1 AND container_id = $2 AND type_id = $3 \
               AND bound = false AND stack_size + $4 <= $5 \
             ORDER BY slot_id LIMIT 1 FOR UPDATE",
        )
        .bind(player_id)
        .bind(p.container_id)
        .bind(p.design_id)
        .bind(p.quantity)
        .bind(p.max_stack)
        .fetch_optional(&mut **tx)
        .await
        .map_err(at("place"))?;
        if let Some(target) = merge {
            let done = sqlx::query(
                "UPDATE sgw_inventory SET stack_size = stack_size + $1 WHERE item_id = $2",
            )
            .bind(p.quantity)
            .bind(target.item_id)
            .execute(&mut **tx)
            .await
            .map_err(at("place"))?;
            expect_rows(ids, "place", done, 1)?;
            applied.granted.push(GrantedStack {
                item_id: target.item_id,
                design_id: p.design_id,
                container_id: p.container_id,
                slot_id: target.slot_id,
                before: target.stack_size,
                after: target.stack_size + p.quantity,
            });
            return Ok(());
        }
    }

    // Both are positive: `resolve` skips non-positive quantities and clamps
    // the stack size to at least 1.
    let needed = (p.quantity.unsigned_abs() as usize).div_ceil(p.max_stack.unsigned_abs() as usize);
    let occupied: Vec<i32> = sqlx::query_scalar(
        "SELECT slot_id FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = $2 FOR UPDATE",
    )
    .bind(player_id)
    .bind(p.container_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(at("place"))?;
    let Some(slots) = free_slots(p.container_id, &occupied, needed) else {
        return Err(CraftReject::InventoryFull {
            design_id: p.design_id,
            container_id: p.container_id,
        }
        .into());
    };

    let mut remaining = p.quantity;
    for slot_id in slots {
        let count = remaining.min(p.max_stack);
        remaining -= count;
        // Same row shape as the generic grant: the item's own charges and
        // ammo configuration.
        let item_id: i32 = sqlx::query_scalar(
            "INSERT INTO sgw_inventory \
                (character_id, type_id, stack_size, slot_id, container_id, \
                 bound, durability, charges, ammo_type, ammo_types, ammo, flags) \
             SELECT $1, ri.item_id, $2, $3, $4, false, 100, ri.charges, \
                    COALESCE(ri.default_ammo_type, 'AMMO_NONE'::resources.\"EAmmoType\"), \
                    ri.ammo_types, ri.charges, 0 \
             FROM resources.items ri WHERE ri.item_id = $5 \
             RETURNING item_id",
        )
        .bind(player_id)
        .bind(count)
        .bind(slot_id)
        .bind(p.container_id)
        .bind(p.design_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(at("place"))?;
        applied.granted.push(GrantedStack {
            item_id,
            design_id: p.design_id,
            container_id: p.container_id,
            slot_id,
            before: 0,
            after: count,
        });
    }
    Ok(())
}

/// The first `needed` free slots of `container_id`, or `None` if it has
/// fewer.
fn free_slots(container_id: i32, occupied: &[i32], needed: usize) -> Option<Vec<i32>> {
    let occupied: HashSet<i32> = occupied.iter().copied().collect();
    let slots: Vec<i32> = (bag_min_slot(container_id)..bag_max_slots(container_id))
        .filter(|s| !occupied.contains(s))
        .take(needed)
        .collect();
    (slots.len() == needed).then_some(slots)
}

#[cfg(test)]
mod free_slot_tests {
    use super::free_slots;

    #[test]
    fn free_slots_fill_holes_in_order() {
        assert_eq!(free_slots(15, &[0, 2], 2), Some(vec![1, 3]));
    }

    #[test]
    fn a_full_bag_has_no_free_slot() {
        let full: Vec<i32> = (0..100).collect();
        assert_eq!(free_slots(15, &full, 1), None);
    }

    #[test]
    fn storage_containers_have_no_grant_slots() {
        assert_eq!(free_slots(17, &[], 1), None);
    }
}
