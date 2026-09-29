//! The one transaction behind a crafting item use: lock, decide, consume,
//! save, commit.

use cimmeria_entity::inventory::{INV_CRAFTING, INV_MAIN};
use sqlx::{PgConnection, PgPool};

use super::rule::{decide, Applied, ItemEffects};
use crate::base::crafting::feedback::CraftReject;
use crate::base::crafting::inventory_locks::take_inventory_locks;
use crate::base::crafting::persistence::{load_crafting_state_locked, save_crafting_state_in};
use crate::base::outbox::{self, CellOutboxPayload};

/// The bags an item can be used from: the ones the player carries. Using
/// from the bank, the buyback list or an equipment slot is refused, so a
/// sold item cannot be used and then bought back.
pub const CARRIED_CONTAINERS: [i32; 2] = [INV_MAIN, INV_CRAFTING];

/// Take the advisory locks an item use needs, before any row: the
/// player-wide key, then the carried bags (the shared order in
/// [`crate::base::crafting::inventory_locks`]).
///
/// The player-wide key serializes the use with inventory moves, crafting
/// completions and vendor purchases, which take it first; the main-bag key
/// does the same for trade, which takes it before `sgw_player` and the item
/// rows. The use must lock `sgw_player` because it writes the crafting
/// columns there; it does so after the item row, the order the vendor paths
/// use, and only once it holds every advisory key another path could want
/// while holding that row.
pub async fn take_item_use_locks(
    conn: &mut PgConnection,
    player_id: i32,
) -> Result<(), sqlx::Error> {
    take_inventory_locks(conn, player_id, &CARRIED_CONTAINERS).await
}

/// A committed use: the crafting change and the one item it consumed.
#[derive(Debug)]
pub struct Committed {
    pub applied: Applied,
    pub type_id: i32,
    pub container_id: i32,
    pub qty_before: i32,
    pub qty_after: i32,
    /// The saved blueprint list, for the 139 push (it replaces the client's
    /// whole list).
    pub blueprint_ids: Vec<i32>,
    /// The cell notification for a fully removed instance, enqueued in the
    /// same transaction; the caller dispatches it after the commit.
    pub outbox: Option<(i64, CellOutboxPayload)>,
}

/// A use that failed and rolled back, with the sub-step it failed in.
#[derive(Debug)]
pub struct UseFailure {
    /// `begin`, `advisory_lock`, `lock_item`, `item_effects`,
    /// `lock_player`, `consume`, `save_state`, `outbox` or `commit`.
    pub phase: &'static str,
    /// `sql_error`, `rows_affected_short`, `bad_stack_size`, `no_effects`
    /// or `no_player`.
    pub reason: &'static str,
    /// The database error, for `sql_error`.
    pub error: Option<sqlx::Error>,
    /// For `rows_affected_short`: what the write changed and what it had to.
    pub rows_affected: u64,
    pub expected: u64,
}

impl UseFailure {
    fn sql(phase: &'static str) -> impl FnOnce(sqlx::Error) -> Self {
        move |e| UseFailure {
            phase,
            reason: "sql_error",
            error: Some(e),
            rows_affected: 0,
            expected: 0,
        }
    }

    fn short(phase: &'static str, rows_affected: u64, expected: u64) -> Self {
        UseFailure {
            phase,
            reason: "rows_affected_short",
            error: None,
            rows_affected,
            expected,
        }
    }

    fn data(phase: &'static str, reason: &'static str) -> Self {
        UseFailure {
            phase,
            reason,
            error: None,
            rows_affected: 0,
            expected: 0,
        }
    }
}

/// The use transaction for inventory instance `item_id` of `player_id`.
/// `Ok(Ok(_))` is a committed use; `Ok(Err(why))` a refusal, rolled back
/// with nothing written or consumed; `Err` a failure, also rolled back.
///
/// Lock order ([`take_item_use_locks`]): every advisory lock first, then the
/// item row, then `sgw_player`. The item row is re-read under its lock with
/// the owner in the `WHERE`, so an item traded away after the caller's
/// ownership check is refused, never consumed from its new owner.
pub async fn use_item_in_db(
    pool: &PgPool,
    entity_id: u32,
    player_id: i32,
    item_id: i32,
) -> Result<Result<Committed, CraftReject>, UseFailure> {
    let mut tx = pool.begin().await.map_err(UseFailure::sql("begin"))?;
    take_item_use_locks(&mut tx, player_id)
        .await
        .map_err(UseFailure::sql("advisory_lock"))?;

    let row: Option<(i32, i32, Option<i32>)> = sqlx::query_as(
        "SELECT type_id, container_id, stack_size FROM sgw_inventory \
         WHERE character_id = $1 AND item_id = $2 FOR UPDATE",
    )
    .bind(player_id)
    .bind(item_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(UseFailure::sql("lock_item"))?;
    let Some((type_id, container_id, stack_size)) = row else {
        return Ok(Err(CraftReject::ItemMissing { item_id }));
    };
    if !CARRIED_CONTAINERS.contains(&container_id) {
        return Ok(Err(CraftReject::ItemNotCarried {
            item_id,
            type_id,
            container_id,
        }));
    }
    // An empty stack is a corrupt row (the column is NOT NULL, but nothing
    // bounds it below); using "one" of it must never be a free grant.
    let qty_before = match stack_size {
        Some(n) if n >= 1 => n,
        _ => return Err(UseFailure::data("lock_item", "bad_stack_size")),
    };

    let rows: Vec<(Option<i32>, Option<i32>)> = sqlx::query_as(
        "SELECT blueprint_id, racial_paradigm_id FROM resources.crafting_item_effects \
         WHERE item_id = $1",
    )
    .bind(type_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(UseFailure::sql("item_effects"))?;
    let Some(effects) = ItemEffects::from_rows(&rows) else {
        return Err(UseFailure::data("item_effects", "no_effects"));
    };

    let Some(mut state) = load_crafting_state_locked(&mut tx, player_id)
        .await
        .map_err(UseFailure::sql("lock_player"))?
    else {
        return Err(UseFailure::data("lock_player", "no_player"));
    };
    let applied = match decide(&mut state, &effects, type_id) {
        Ok(applied) => applied,
        Err(why) => return Ok(Err(why)),
    };

    // Consume first: a crafting change is only ever written next to the
    // item that paid for it.
    let removed_all = qty_before == 1;
    let consumed = if removed_all {
        sqlx::query(
            "DELETE FROM sgw_inventory \
             WHERE character_id = $1 AND item_id = $2 AND stack_size = 1",
        )
    } else {
        sqlx::query(
            "UPDATE sgw_inventory SET stack_size = stack_size - 1 \
             WHERE character_id = $1 AND item_id = $2 AND stack_size > 1",
        )
    }
    .bind(player_id)
    .bind(item_id)
    .execute(&mut *tx)
    .await
    .map_err(UseFailure::sql("consume"))?;
    if consumed.rows_affected() != 1 {
        return Err(UseFailure::short("consume", consumed.rows_affected(), 1));
    }

    save_crafting_state_in(&mut tx, player_id, &state)
        .await
        .map_err(UseFailure::sql("save_state"))?;

    let outbox = if removed_all {
        let payload = CellOutboxPayload::InventoryItemRemoved {
            item_id,
            source_container_id: container_id,
        };
        let id = outbox::enqueue_in_tx(&mut tx, entity_id, &payload)
            .await
            .map_err(UseFailure::sql("outbox"))?;
        Some((id, payload))
    } else {
        None
    };

    tx.commit().await.map_err(UseFailure::sql("commit"))?;
    Ok(Ok(Committed {
        applied,
        type_id,
        container_id,
        qty_before,
        qty_after: qty_before - 1,
        blueprint_ids: state.blueprint_ids,
        outbox,
    }))
}
