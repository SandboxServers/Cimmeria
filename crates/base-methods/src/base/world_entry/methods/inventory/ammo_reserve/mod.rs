//! `AmmoReserve`: special-ammo rounds held as ordinary stacks in the carried
//! bags (ammo campaign AM-F, issue #1026; D-AM01, D-AM05).
//!
//! Three calls, each inside the caller's transaction:
//!
//! * [`count`] — rounds of one ammo type across every stack in the bags;
//! * [`draw`] — remove up to `n` rounds (a reload, AM-02);
//! * [`return_rounds`] — put up to `n` rounds back (an ammo-type switch,
//!   AM-02), reporting what did not fit so the caller keeps it in the clip.
//!
//! **Which rows.** The ammo type maps to one item design through
//! `resources.ammo_item_types` (never a hardcoded id). Only the carried bags
//! [`RESERVE_BAGS`] count: main (1), then crafting (15). The personal vault
//! (17) is storage, not a reserve. Stacks are visited in bag order, then slot
//! order.
//!
//! **Locks.** [`draw`] and [`return_rounds`] take the shared inventory
//! advisory locks first (`take_inventory_locks`: the player-wide key, then
//! bags 1 and 15), then `FOR UPDATE` on every stack they read, the order
//! every other inventory writer uses (crafting, mail, trade, vault moves), so
//! a reserve write never deadlocks against them. The advisory locks are
//! re-entrant inside one transaction, so a caller that already holds them
//! loses nothing. [`count`] takes no lock; a caller that reads then writes
//! calls [`draw`], which re-reads under the lock.
//!
//! **What the caller still owes.** These are database writes only. After the
//! commit the caller tells the client about every [`StackChange`] (an
//! `onUpdateItem` for a changed or new stack, an `onRemoveItem` for an
//! emptied one, or `send_full_inventory_resync`) and logs the catalog event
//! (`reload_draw`, `ammo_switch_return`; `cimmeria_entity::ammo_telemetry`).
//! Nothing here reads the `ammo.finite_special` flag: the caller decides
//! whether to draw at all.

mod commits;
mod plan;
mod requests;

#[cfg(test)]
mod live_db_tests;
#[cfg(test)]
mod requests_live_db_tests;
#[cfg(test)]
mod requests_shell_live_db_tests;

pub use commits::{commit_reload_draw, commit_switch_return, DrawCommit, ReturnCommit};
pub use requests::{handle_ammo_reserve_request, ReserveIo};

use cimmeria_entity::inventory::{INV_CRAFTING, INV_MAIN};
use sqlx::{Postgres, Transaction};

use crate::base::inventory_locks::take_inventory_locks;
use crate::base::resources::{bag_max_slots, bag_min_slot};
use plan::{plan_draw, plan_return};

/// The bags a reserve draws from and returns to, in visiting order.
pub const RESERVE_BAGS: [i32; 2] = [INV_MAIN, INV_CRAFTING];

/// One stack row a draw or return touched.
///
/// `before == 0` means the call opened this stack (a new row);
/// `after == 0` means it emptied the stack and deleted the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackChange {
    /// `sgw_inventory.item_id`, the row's instance id.
    pub instance_id: i32,
    pub container_id: i32,
    pub slot_id: i32,
    pub before: i32,
    pub after: i32,
}

/// The result of [`draw`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AmmoDraw {
    /// The ammo item's design id, or `None` when the type has no reserve
    /// item (default ammo, daggers): then nothing was drawn.
    pub item_id: Option<i32>,
    /// Rounds actually removed; may be less than requested (D-AM05: a short
    /// stack loads what is there).
    pub drawn: i32,
    /// Rounds of this type in the reserve bags before and after.
    pub stack_before: i32,
    pub stack_after: i32,
    /// Every stack row changed, in the order it was changed.
    pub changes: Vec<StackChange>,
}

/// The result of [`return_rounds`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AmmoReturn {
    /// The ammo item's design id, or `None` when the type has no reserve
    /// item: then every round is `remainder`.
    pub item_id: Option<i32>,
    /// Rounds actually added to the bags.
    pub returned: i32,
    /// Rounds that did not fit. The caller keeps them in the clip; they are
    /// never deleted (D-AM05).
    pub remainder: i32,
    /// Rounds of this type in the reserve bags before and after.
    pub stack_before: i32,
    pub stack_after: i32,
    /// Every stack row changed or opened, in the order it was written.
    pub changes: Vec<StackChange>,
}

#[derive(Debug, sqlx::FromRow)]
struct StackRow {
    item_id: i32,
    container_id: i32,
    slot_id: i32,
    stack_size: i32,
}

/// The reserve item's design id and stack cap for an `EAmmoType` ordinal, or
/// `None` when the type has no row in `resources.ammo_item_types` (default
/// ammo, daggers, or an out-of-range value).
async fn reserve_item(
    tx: &mut Transaction<'_, Postgres>,
    ammo_type: i32,
) -> Result<Option<(i32, i32)>, sqlx::Error> {
    if ammo_type < 0 {
        return Ok(None);
    }
    // `enum_range(...)[n]` is NULL past the end, so an unknown ordinal
    // matches no row instead of erroring.
    sqlx::query_as::<_, (i32, i32)>(
        "SELECT t.item_id, i.max_stack_size \
           FROM resources.ammo_item_types t \
           JOIN resources.items i ON i.item_id = t.item_id \
          WHERE t.ammo_type = (enum_range(NULL::resources.\"EAmmoType\"))[$1 + 1]",
    )
    .bind(ammo_type)
    .fetch_optional(&mut **tx)
    .await
}

/// Rounds of `ammo_type` in the player's reserve bags, summed across every
/// stack. Read-only, no lock; 0 for a type with no reserve item.
pub async fn count(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
    ammo_type: i32,
) -> Result<i32, sqlx::Error> {
    let Some((item_id, _)) = reserve_item(tx, ammo_type).await? else {
        return Ok(0);
    };
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(stack_size), 0)::bigint FROM sgw_inventory \
          WHERE character_id = $1 AND container_id = ANY($2) AND type_id = $3 \
            AND stack_size > 0",
    )
    .bind(player_id)
    .bind(&RESERVE_BAGS[..])
    .bind(item_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(i32::try_from(total).unwrap_or(i32::MAX))
}

/// The player's stacks of `item_id` in the reserve bags, locked, in bag then
/// slot order. `only_unbound` limits it to stacks a return may merge into.
async fn locked_stacks(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
    item_id: i32,
    only_unbound: bool,
) -> Result<Vec<StackRow>, sqlx::Error> {
    sqlx::query_as::<_, StackRow>(
        "SELECT item_id, container_id, slot_id, stack_size FROM sgw_inventory \
          WHERE character_id = $1 AND container_id = ANY($2) AND type_id = $3 \
            AND stack_size > 0 AND (NOT $4 OR bound = false) \
          ORDER BY array_position($2, container_id), slot_id \
          FOR UPDATE",
    )
    .bind(player_id)
    .bind(&RESERVE_BAGS[..])
    .bind(item_id)
    .bind(only_unbound)
    .fetch_all(&mut **tx)
    .await
}

/// Remove up to `n` rounds of `ammo_type` from the player's reserve bags,
/// across as many stacks as needed. Locks first (see the module docs),
/// deletes a stack that reaches zero, and never removes more than is
/// present: `drawn` is what actually loaded. `n <= 0` draws nothing.
pub async fn draw(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
    ammo_type: i32,
    n: i32,
) -> Result<AmmoDraw, sqlx::Error> {
    let Some((item_id, _)) = reserve_item(tx, ammo_type).await? else {
        return Ok(AmmoDraw::default());
    };
    take_inventory_locks(tx, player_id, &RESERVE_BAGS).await?;
    let stacks = locked_stacks(tx, player_id, item_id, false).await?;
    let stack_before: i32 = stacks.iter().map(|s| s.stack_size).sum();
    let takes = plan_draw(&stacks.iter().map(|s| s.stack_size).collect::<Vec<_>>(), n);

    let mut out = AmmoDraw {
        item_id: Some(item_id),
        stack_before,
        ..AmmoDraw::default()
    };
    for (stack, take) in stacks.iter().zip(takes) {
        if take <= 0 {
            continue;
        }
        let after = stack.stack_size - take;
        if after == 0 {
            sqlx::query("DELETE FROM sgw_inventory WHERE item_id = $1")
                .bind(stack.item_id)
                .execute(&mut **tx)
                .await?;
        } else {
            sqlx::query("UPDATE sgw_inventory SET stack_size = $1 WHERE item_id = $2")
                .bind(after)
                .bind(stack.item_id)
                .execute(&mut **tx)
                .await?;
        }
        out.drawn += take;
        out.changes.push(StackChange {
            instance_id: stack.item_id,
            container_id: stack.container_id,
            slot_id: stack.slot_id,
            before: stack.stack_size,
            after,
        });
    }
    out.stack_after = stack_before - out.drawn;
    Ok(out)
}

/// Free slots in the reserve bags, in bag then slot order. Call with the
/// advisory locks held, so no other writer can take one before the insert.
async fn free_reserve_slots(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
) -> Result<Vec<(i32, i32)>, sqlx::Error> {
    let mut free = Vec::new();
    for bag in RESERVE_BAGS {
        let occupied: Vec<i32> = sqlx::query_scalar(
            "SELECT slot_id FROM sgw_inventory WHERE character_id = $1 AND container_id = $2",
        )
        .bind(player_id)
        .bind(bag)
        .fetch_all(&mut **tx)
        .await?;
        free.extend(
            (bag_min_slot(bag).max(0)..bag_max_slots(bag))
                .filter(|slot| !occupied.contains(slot))
                .map(|slot| (bag, slot)),
        );
    }
    Ok(free)
}

/// Add up to `n` rounds of `ammo_type` back to the player's reserve bags:
/// top up existing unbound stacks first (up to the item's `max_stack_size`),
/// then open new stacks in free slots. `remainder` is what did not fit; the
/// caller keeps those rounds in the clip (D-AM05). `n <= 0` returns nothing.
pub async fn return_rounds(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
    ammo_type: i32,
    n: i32,
) -> Result<AmmoReturn, sqlx::Error> {
    let n = n.max(0);
    let Some((item_id, cap)) = reserve_item(tx, ammo_type).await? else {
        return Ok(AmmoReturn {
            remainder: n,
            ..AmmoReturn::default()
        });
    };
    take_inventory_locks(tx, player_id, &RESERVE_BAGS).await?;
    let stack_before = count(tx, player_id, ammo_type).await?;
    let stacks = locked_stacks(tx, player_id, item_id, true).await?;
    let free = if n > 0 {
        free_reserve_slots(tx, player_id).await?
    } else {
        Vec::new()
    };
    let plan = plan_return(
        &stacks.iter().map(|s| s.stack_size).collect::<Vec<_>>(),
        cap,
        free.len(),
        n,
    );

    let mut out = AmmoReturn {
        item_id: Some(item_id),
        stack_before,
        remainder: plan.remainder,
        ..AmmoReturn::default()
    };
    for (stack, add) in stacks.iter().zip(&plan.merges) {
        if *add <= 0 {
            continue;
        }
        let after = stack.stack_size + add;
        sqlx::query("UPDATE sgw_inventory SET stack_size = $1 WHERE item_id = $2")
            .bind(after)
            .bind(stack.item_id)
            .execute(&mut **tx)
            .await?;
        out.returned += add;
        out.changes.push(StackChange {
            instance_id: stack.item_id,
            container_id: stack.container_id,
            slot_id: stack.slot_id,
            before: stack.stack_size,
            after,
        });
    }
    for (&(container_id, slot_id), &size) in free.iter().zip(&plan.new_stacks) {
        // Same column defaults as the generic grant (`grant/persist.rs`).
        let instance_id: i32 = sqlx::query_scalar(
            "INSERT INTO sgw_inventory \
                (character_id, type_id, stack_size, slot_id, container_id, \
                 bound, durability, charges, ammo_type, ammo_types, ammo, flags) \
             SELECT $1, ri.item_id, $2, $3, $4, false, 100, ri.charges, \
                    COALESCE(ri.default_ammo_type, 'AMMO_NONE'::resources.\"EAmmoType\"), \
                    ri.ammo_types, 0, 0 \
               FROM resources.items ri WHERE ri.item_id = $5 \
             RETURNING item_id",
        )
        .bind(player_id)
        .bind(size)
        .bind(slot_id)
        .bind(container_id)
        .bind(item_id)
        .fetch_one(&mut **tx)
        .await?;
        out.returned += size;
        out.changes.push(StackChange {
            instance_id,
            container_id,
            slot_id,
            before: 0,
            after: size,
        });
    }
    out.stack_after = stack_before + out.returned;
    Ok(out)
}
