//! Exact carried item counts of a staged fixture (FX-03, D-FX7).
//!
//! An item entry means "after the fixture the character carries exactly
//! `count` of this design, all of it in `container_id`": every carried
//! instance (containers 1 to 15, `FIXTURE_CARRIED_CONTAINERS`, so the
//! bandolier and the equipment slots too) is deleted and `count` is granted
//! fresh, one row per stack of the design's `max_stack_size`. The buyback,
//! bank, auction and org-vault containers (16 to 20) are never touched.
//!
//! Split like the missions. [`plan_items`] runs in the apply's precheck: the
//! static checks, the design lookups, and a dry run of the whole write in a
//! transaction it rolls back, so a list that cannot land (no room for the
//! stacks) refuses the fixture before anything, missions included, is
//! written. [`write_items`] runs the same write in one transaction and
//! commits it: a failure part-way leaves the inventory as it was.
//!
//! The only persisted pointer into the carried inventory is
//! `sgw_player.bandolier_slot` (a slot of container 3, read at
//! `onClientReady` next to the bandolier rows). Removing the active weapon
//! would leave it on an empty slot while other weapons remain, a state no
//! other removal path leaves, so the write reconciles it the way
//! `sync_bandolier_after_inventory_change` does.

use std::collections::HashSet;

use cimmeria_entity::inventory::{INV_BANDOLIER, INV_MAIN, INV_MISSION};
use sqlx::{PgPool, Postgres, Transaction};

use cimmeria_base_session::base::inventory_locks::take_inventory_locks;
use cimmeria_base_session::base::world_entry::lab_fixture::{
    StagedItem, FIXTURE_CARRIED_CONTAINERS, FIXTURE_MAX_ITEM_COUNT,
};

use super::super::methods::vendor::serializers::reserve_free_inventory_slots;

/// `resources.items.flags` bit for bind-on-acquire
/// (`cimmeria_cell_catalog::crafting::ItemFlags::BIND_ON_ACQUIRE`; this crate
/// does not depend on the catalog). A fixture item is bound when its design
/// says so, as the grant path makes it.
const BIND_ON_ACQUIRE: i32 = 4;

/// Durability of a granted item, as the grant and vendor paths write it.
const GRANTED_DURABILITY: i32 = 100;

/// One checked fixture item, ready to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ItemPlan {
    type_id: i32,
    /// The design's name, for the error text.
    name: String,
    container_id: i32,
    /// The rows to insert, one stack each, in order.
    stacks: Vec<i32>,
}

impl ItemPlan {
    fn label(&self) -> String {
        format!("item {} ({})", self.name, self.type_id)
    }
}

/// Check every fixture item and dry-run the write, writing nothing: the
/// apply's precheck, so a bad item anywhere in the fixture writes nothing
/// at all.
pub(crate) async fn plan_items(
    pool: &PgPool,
    player_id: i32,
    items: &[StagedItem],
) -> Result<Vec<ItemPlan>, String> {
    // The static checks first, so a malformed list never touches the
    // database. FX-04's stager runs the same ones; these are the backstop
    // for anything else that stages.
    let mut seen = HashSet::new();
    for item in items {
        if !seen.insert(item.type_id) {
            return Err(format!("item {} is listed twice", item.type_id));
        }
        if item.count > FIXTURE_MAX_ITEM_COUNT {
            return Err(format!(
                "item {} count {} is over the fixture limit of {FIXTURE_MAX_ITEM_COUNT}",
                item.type_id, item.count
            ));
        }
        if item.container_id != INV_MAIN && item.container_id != INV_MISSION {
            return Err(format!(
                "item {} container {} is not main ({INV_MAIN}) or mission ({INV_MISSION})",
                item.type_id, item.container_id
            ));
        }
    }
    let mut plans = Vec::with_capacity(items.len());
    for item in items {
        let design: Option<(String, i32)> =
            sqlx::query_as("SELECT name, max_stack_size FROM resources.items WHERE item_id = $1")
                .bind(item.type_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("item {} lookup failed: {e}", item.type_id))?;
        let Some((name, max_stack)) = design else {
            return Err(format!("item {} is not in resources.items", item.type_id));
        };
        plans.push(ItemPlan {
            type_id: item.type_id,
            name,
            container_id: item.container_id,
            stacks: stack_sizes(item.count, max_stack),
        });
    }
    if plans.is_empty() {
        return Ok(plans);
    }
    // The dry run: the real write, rolled back. Room for the stacks depends
    // on what the deletes free, so only running the write answers it.
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("items dry run: begin failed: {e}"))?;
    set_counts(&mut tx, player_id, &plans).await?;
    tx.rollback()
        .await
        .map_err(|e| format!("items dry run: rollback failed: {e}"))?;
    Ok(plans)
}

/// Write the counts [`plan_items`] checked, in one transaction; returns
/// `(removed, granted)` inventory rows.
pub(crate) async fn write_items(
    pool: &PgPool,
    player_id: i32,
    plans: &[ItemPlan],
) -> Result<(u64, u64), String> {
    if plans.is_empty() {
        return Ok((0, 0));
    }
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("items: begin failed: {e}"))?;
    // An Err drops `tx`, which rolls every delete and insert back.
    let counts = set_counts(&mut tx, player_id, plans).await?;
    tx.commit()
        .await
        .map_err(|e| format!("items: commit failed: {e}"))?;
    Ok(counts)
}

/// `count` split into stacks of at most `max_stack` (a design with a
/// non-positive stack size is treated as unstackable).
fn stack_sizes(count: u32, max_stack: i32) -> Vec<i32> {
    let max_stack = max_stack.max(1);
    let mut left = i32::try_from(count).unwrap_or(i32::MAX);
    let mut stacks = Vec::new();
    while left > 0 {
        let size = left.min(max_stack);
        stacks.push(size);
        left -= size;
    }
    stacks
}

/// The write, inside `tx`: lock, delete every carried instance, grant the
/// stacks, reconcile the bandolier pointer.
async fn set_counts(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
    plans: &[ItemPlan],
) -> Result<(u64, u64), String> {
    // The shared inventory order (`inventory_locks`): every advisory lock
    // first, then item rows, then the player row (the bandolier UPDATE
    // last). The deletes reach every carried bag, so lock them all.
    let bags: Vec<i32> = FIXTURE_CARRIED_CONTAINERS.collect();
    take_inventory_locks(tx, player_id, &bags)
        .await
        .map_err(|e| format!("items: inventory locks failed: {e}"))?;

    let mut removed = 0;
    for plan in plans {
        let done = sqlx::query(
            "DELETE FROM sgw_inventory WHERE character_id = $1 AND type_id = $2 \
             AND container_id BETWEEN $3 AND $4",
        )
        .bind(player_id)
        .bind(plan.type_id)
        .bind(*FIXTURE_CARRIED_CONTAINERS.start())
        .bind(*FIXTURE_CARRIED_CONTAINERS.end())
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("{}: delete failed: {e}", plan.label()))?;
        removed += done.rows_affected();
    }

    let mut granted = 0;
    for plan in plans.iter().filter(|p| !p.stacks.is_empty()) {
        let slots =
            reserve_free_inventory_slots(tx, player_id, plan.container_id, plan.stacks.len())
                .await
                .map_err(|e| format!("{}: slot reserve failed: {e}", plan.label()))?
                .ok_or_else(|| {
                    format!(
                        "{}: container {} has no room for {} stack(s)",
                        plan.label(),
                        plan.container_id,
                        plan.stacks.len()
                    )
                })?;
        for (slot, size) in slots.iter().zip(&plan.stacks) {
            insert_stack(tx, player_id, plan, *slot, *size).await?;
            granted += 1;
        }
    }

    reconcile_bandolier_slot(tx, player_id).await?;
    Ok((removed, granted))
}

/// One stack, with the grant INSERT's columns and defaults
/// (`inventory/grant/persist.rs`): durability 100, the design's charges,
/// ammo configuration and bind-on-acquire flag.
async fn insert_stack(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
    plan: &ItemPlan,
    slot_id: i32,
    stack_size: i32,
) -> Result<(), String> {
    let done = sqlx::query(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges, \
             ammo_type, ammo_types, ammo, flags) \
         SELECT $1, ri.item_id, $2, $3, $4, (ri.flags & $5) <> 0, $6, ri.charges, \
                COALESCE(ri.default_ammo_type, 'AMMO_NONE'::resources.\"EAmmoType\"), \
                ri.ammo_types, ri.charges, 0 \
         FROM resources.items ri WHERE ri.item_id = $7",
    )
    .bind(player_id)
    .bind(stack_size)
    .bind(slot_id)
    .bind(plan.container_id)
    .bind(BIND_ON_ACQUIRE)
    .bind(GRANTED_DURABILITY)
    .bind(plan.type_id)
    .execute(&mut **tx)
    .await
    .map_err(|e| format!("{}: insert failed: {e}", plan.label()))?;
    if done.rows_affected() != 1 {
        return Err(format!(
            "{}: insert wrote {} rows",
            plan.label(),
            done.rows_affected()
        ));
    }
    Ok(())
}

/// Point `bandolier_slot` at a remaining weapon when the fixture removed the
/// active one: the lowest occupied bandolier slot, as
/// `sync_bandolier_after_inventory_change` picks it. An empty bandolier
/// keeps its value, as that function leaves it (nothing is held either way).
async fn reconcile_bandolier_slot(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE sgw_player p SET bandolier_slot = b.slot \
         FROM (SELECT min(slot_id) AS slot FROM sgw_inventory \
               WHERE character_id = $1 AND container_id = $2) b \
         WHERE p.player_id = $1 AND b.slot IS NOT NULL \
           AND NOT EXISTS (SELECT 1 FROM sgw_inventory i \
                           WHERE i.character_id = $1 AND i.container_id = $2 \
                             AND i.slot_id = p.bandolier_slot)",
    )
    .bind(player_id)
    .bind(INV_BANDOLIER)
    .execute(&mut **tx)
    .await
    .map_err(|e| format!("items: bandolier slot reconcile failed: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::stack_sizes;

    #[test]
    fn a_count_splits_into_stacks_of_the_design_maximum() {
        assert_eq!(stack_sizes(0, 5), Vec::<i32>::new());
        assert_eq!(stack_sizes(3, 1), vec![1, 1, 1]);
        assert_eq!(stack_sizes(7, 3), vec![3, 3, 1]);
        assert_eq!(stack_sizes(10, 20), vec![10]);
        // A design with no stack size still lands one row per unit.
        assert_eq!(stack_sizes(2, 0), vec![1, 1]);
    }
}
