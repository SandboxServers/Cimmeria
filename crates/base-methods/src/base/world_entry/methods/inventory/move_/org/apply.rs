//! The writes of a Team or Command vault move, inside the caller's move
//! transaction (bank-vault BV-07). An item crosses between `sgw_inventory`
//! and `sgw_organization_vault_items` as `INSERT … SELECT` plus `DELETE`, so
//! every instance column travels in SQL and a whole move keeps its
//! `item_id`; a split takes a fresh id from `sgw_inventory_item_id_seq`, the
//! sequence both tables draw from.
//!
//! Each statement must touch exactly the rows it names; anything else is
//! logged and returns `None`, and the caller rolls back.

use cimmeria_entity::known_names;
use sqlx::postgres::PgArguments;
use sqlx::query::Query;
use sqlx::{Postgres, Transaction};

use super::super::bank_rules::MoveShape;
use super::super::{InventoryInstanceRow, Occupant};
use super::Side;

type MoveTx = Transaction<'static, Postgres>;

/// The instance columns both tables share, for the `INSERT` lists. A macro
/// so each statement stays a `&'static str` (sqlx refuses a built
/// `String`). The `SELECT` lists name the same columns in the same order.
macro_rules! item_columns {
    () => {
        "type_id, stack_size, charges, durability, flags, bound, ammo, cur_ammo_type, \
         ammo_type, ammo_types"
    };
}

/// One vault move to write.
#[derive(Debug, Clone, Copy)]
pub(super) struct Plan {
    pub player_id: i32,
    pub org_id: i32,
    pub org_type: i16,
    pub item_id: i32,
    pub source_side: Side,
    pub target_side: Side,
    pub source: InventoryInstanceRow,
    pub occupant: Option<Occupant>,
    pub shape: MoveShape,
    pub quantity: i32,
    pub target_container_id: i32,
    pub target_slot_id: i32,
}

/// What the write left behind.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Applied {
    /// The row a split created.
    pub new_item_id: Option<i32>,
    /// A whole-stack merge deleted the source row.
    pub source_deleted: bool,
}

/// Run `q` and require exactly `expected` rows.
async fn exact(
    tx: &mut MoveTx,
    q: Query<'_, Postgres, PgArguments>,
    expected: u64,
    what: &'static str,
    plan: &Plan,
) -> Option<()> {
    match q.execute(&mut **tx).await {
        Ok(r) if r.rows_affected() == expected => Some(()),
        Ok(r) => {
            tracing::warn!(
                target: "bank",
                player_id = plan.player_id,
                player_name = known_names::player_name(plan.player_id),
                org_id = plan.org_id,
                org_name = known_names::org_name(plan.org_id),
                item_id = plan.item_id,
                item_name = cimmeria_names::book().item(plan.source.type_id),
                step = what,
                rows_affected = r.rows_affected(),
                expected,
                "org vault move: a write matched the wrong number of rows; rolling back"
            );
            None
        }
        Err(e) => {
            tracing::error!(
                target: "bank",
                player_id = plan.player_id,
                player_name = known_names::player_name(plan.player_id),
                org_id = plan.org_id,
                org_name = known_names::org_name(plan.org_id),
                item_id = plan.item_id,
                item_name = cimmeria_names::book().item(plan.source.type_id),
                step = what,
                "org vault move: a write failed; rolling back: {e}"
            );
            None
        }
    }
}

/// Copy a carried row into the vault at `(container, slot)`: the whole row
/// under its own id (`quantity = None`), or `quantity` of it under a new id.
async fn carried_to_vault(
    tx: &mut MoveTx,
    plan: &Plan,
    item_id: i32,
    container_id: i32,
    slot_id: i32,
    quantity: Option<i32>,
) -> Option<i32> {
    let sql: &str = concat!(
        "INSERT INTO sgw_organization_vault_items \
         (item_id, org_id, org_type, container_id, slot_id, ",
        item_columns!(),
        ", deposited_by_player_id) \
         SELECT CASE WHEN $7::integer IS NULL THEN item_id \
                     ELSE nextval('sgw_inventory_item_id_seq')::integer END, \
                $3, $4, $5, $6, type_id, COALESCE($7, stack_size), charges, durability, flags, \
                bound, ammo, cur_ammo_type, ammo_type, ammo_types, $1 \
         FROM sgw_inventory WHERE character_id = $1 AND item_id = $2 \
         RETURNING item_id"
    );
    inserted(
        sqlx::query_scalar(sql)
            .bind(plan.player_id)
            .bind(item_id)
            .bind(plan.org_id)
            .bind(plan.org_type)
            .bind(container_id)
            .bind(slot_id)
            .bind(quantity)
            .fetch_optional(&mut **tx)
            .await,
        "carried_to_vault",
        plan,
    )
}

/// Copy a vault row into the player's `(container, slot)`, whole or split.
async fn vault_to_carried(
    tx: &mut MoveTx,
    plan: &Plan,
    item_id: i32,
    container_id: i32,
    slot_id: i32,
    quantity: Option<i32>,
) -> Option<i32> {
    let sql: &str = concat!(
        "INSERT INTO sgw_inventory \
         (item_id, character_id, container_id, slot_id, ",
        item_columns!(),
        ") \
         SELECT CASE WHEN $6::integer IS NULL THEN item_id \
                     ELSE nextval('sgw_inventory_item_id_seq')::integer END, \
                $1, $4, $5, type_id, COALESCE($6, stack_size), charges, durability, flags, \
                bound, ammo, cur_ammo_type, ammo_type, ammo_types \
         FROM sgw_organization_vault_items WHERE org_id = $3 AND item_id = $2 \
         RETURNING item_id"
    );
    inserted(
        sqlx::query_scalar(sql)
            .bind(plan.player_id)
            .bind(item_id)
            .bind(plan.org_id)
            .bind(container_id)
            .bind(slot_id)
            .bind(quantity)
            .fetch_optional(&mut **tx)
            .await,
        "vault_to_carried",
        plan,
    )
}

/// Copy `quantity` of a vault row into another slot of the same vault.
async fn vault_split_within(tx: &mut MoveTx, plan: &Plan) -> Option<i32> {
    let sql: &str = concat!(
        "INSERT INTO sgw_organization_vault_items \
         (item_id, org_id, org_type, container_id, slot_id, ",
        item_columns!(),
        ", deposited_by_player_id) \
         SELECT nextval('sgw_inventory_item_id_seq')::integer, org_id, org_type, container_id, $3, \
                type_id, $4, charges, durability, flags, bound, ammo, cur_ammo_type, ammo_type, \
                ammo_types, $5 \
         FROM sgw_organization_vault_items WHERE org_id = $1 AND item_id = $2 \
         RETURNING item_id"
    );
    inserted(
        sqlx::query_scalar(sql)
            .bind(plan.org_id)
            .bind(plan.item_id)
            .bind(plan.target_slot_id)
            .bind(plan.quantity)
            .bind(plan.player_id)
            .fetch_optional(&mut **tx)
            .await,
        "vault_split_within",
        plan,
    )
}

fn inserted(
    result: Result<Option<i32>, sqlx::Error>,
    what: &'static str,
    plan: &Plan,
) -> Option<i32> {
    match result {
        Ok(Some(id)) => Some(id),
        Ok(None) => {
            tracing::warn!(
                target: "bank",
                player_id = plan.player_id,
                player_name = known_names::player_name(plan.player_id),
                org_id = plan.org_id,
                org_name = known_names::org_name(plan.org_id),
                item_id = plan.item_id,
                item_name = cimmeria_names::book().item(plan.source.type_id),
                step = what,
                "org vault move: the row to copy was not there; rolling back"
            );
            None
        }
        Err(e) => {
            tracing::error!(
                target: "bank",
                player_id = plan.player_id,
                player_name = known_names::player_name(plan.player_id),
                org_id = plan.org_id,
                org_name = known_names::org_name(plan.org_id),
                item_id = plan.item_id,
                item_name = cimmeria_names::book().item(plan.source.type_id),
                step = what,
                "org vault move: a copy failed; rolling back: {e}"
            );
            None
        }
    }
}

/// Delete a row from its side.
async fn delete(tx: &mut MoveTx, plan: &Plan, side: Side, item_id: i32) -> Option<()> {
    let q = match side {
        Side::Carried => {
            sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1 AND item_id = $2")
                .bind(plan.player_id)
        }
        Side::Vault => sqlx::query(
            "DELETE FROM sgw_organization_vault_items WHERE org_id = $1 AND item_id = $2",
        )
        .bind(plan.org_id),
    };
    exact(tx, q.bind(item_id), 1, "delete", plan).await
}

/// Add `delta` to a row's stack; the result must stay positive.
async fn add_stack(
    tx: &mut MoveTx,
    plan: &Plan,
    side: Side,
    item_id: i32,
    delta: i32,
) -> Option<()> {
    let q = match side {
        Side::Carried => sqlx::query(
            "UPDATE sgw_inventory SET stack_size = stack_size + $3 \
             WHERE character_id = $1 AND item_id = $2 AND stack_size + $3 > 0",
        )
        .bind(plan.player_id),
        Side::Vault => sqlx::query(
            "UPDATE sgw_organization_vault_items SET stack_size = stack_size + $3 \
             WHERE org_id = $1 AND item_id = $2 AND stack_size + $3 > 0",
        )
        .bind(plan.org_id),
    };
    exact(tx, q.bind(item_id).bind(delta), 1, "add_stack", plan).await
}

/// Write `plan`. See the module docs.
pub(super) async fn apply(tx: &mut MoveTx, plan: &Plan) -> Option<Applied> {
    let (src, dst) = (plan.source_side, plan.target_side);
    let (tc, ts) = (plan.target_container_id, plan.target_slot_id);
    let mut applied = Applied::default();
    match (plan.shape, plan.occupant) {
        (MoveShape::Whole, _) => match (src, dst) {
            (Side::Carried, Side::Vault) => {
                carried_to_vault(tx, plan, plan.item_id, tc, ts, None).await?;
                delete(tx, plan, Side::Carried, plan.item_id).await?;
            }
            (Side::Vault, Side::Carried) => {
                vault_to_carried(tx, plan, plan.item_id, tc, ts, None).await?;
                delete(tx, plan, Side::Vault, plan.item_id).await?;
            }
            _ => {
                let q = sqlx::query(
                    "UPDATE sgw_organization_vault_items SET slot_id = $3 \
                     WHERE org_id = $1 AND item_id = $2",
                )
                .bind(plan.org_id)
                .bind(plan.item_id)
                .bind(ts);
                exact(tx, q, 1, "vault_slot", plan).await?;
            }
        },
        (MoveShape::Split, _) => {
            add_stack(tx, plan, src, plan.item_id, -plan.quantity).await?;
            let new_id = match (src, dst) {
                (Side::Carried, _) => {
                    carried_to_vault(tx, plan, plan.item_id, tc, ts, Some(plan.quantity)).await?
                }
                (Side::Vault, Side::Carried) => {
                    vault_to_carried(tx, plan, plan.item_id, tc, ts, Some(plan.quantity)).await?
                }
                (Side::Vault, Side::Vault) => vault_split_within(tx, plan).await?,
            };
            applied.new_item_id = Some(new_id);
        }
        (MoveShape::Merge, Some(occ)) => {
            add_stack(tx, plan, dst, occ.item_id, plan.quantity).await?;
            if plan.quantity >= plan.source.stack_size {
                delete(tx, plan, src, plan.item_id).await?;
                applied.source_deleted = true;
            } else {
                add_stack(tx, plan, src, plan.item_id, -plan.quantity).await?;
            }
        }
        (MoveShape::Swap, Some(occ)) => match (src, dst) {
            (Side::Vault, Side::Vault) => {
                let q = sqlx::query(
                    "UPDATE sgw_organization_vault_items \
                     SET slot_id = CASE WHEN item_id = $2 THEN $4 ELSE $5 END \
                     WHERE org_id = $1 AND item_id IN ($2, $3)",
                )
                .bind(plan.org_id)
                .bind(plan.item_id)
                .bind(occ.item_id)
                .bind(ts)
                .bind(plan.source.slot_id);
                exact(tx, q, 2, "vault_swap", plan).await?;
            }
            _ => cross_swap(tx, plan, occ).await?,
        },
        (MoveShape::Merge | MoveShape::Swap, None) => return None,
    }
    Some(applied)
}

/// Swap a carried row with a vault row, whichever was dragged: the carried
/// one is parked at slot -1 to free its slot (the personal swap's sentinel,
/// safe under the `(player, 0)` move lock), the vault row moves into that
/// slot, then the carried row takes the vault row's old slot.
async fn cross_swap(tx: &mut MoveTx, plan: &Plan, occ: Occupant) -> Option<()> {
    let (carried_id, carried_container, carried_slot, vault_id, vault_slot) = match plan.source_side
    {
        Side::Carried => (
            plan.item_id,
            plan.source.container_id,
            plan.source.slot_id,
            occ.item_id,
            plan.target_slot_id,
        ),
        Side::Vault => (
            occ.item_id,
            plan.target_container_id,
            plan.target_slot_id,
            plan.item_id,
            plan.source.slot_id,
        ),
    };
    let vault_container = match plan.source_side {
        Side::Carried => plan.target_container_id,
        Side::Vault => plan.source.container_id,
    };
    let park = sqlx::query(
        "UPDATE sgw_inventory SET slot_id = -1 WHERE character_id = $1 AND item_id = $2",
    )
    .bind(plan.player_id)
    .bind(carried_id);
    exact(tx, park, 1, "swap_park", plan).await?;
    vault_to_carried(tx, plan, vault_id, carried_container, carried_slot, None).await?;
    delete(tx, plan, Side::Vault, vault_id).await?;
    carried_to_vault(tx, plan, carried_id, vault_container, vault_slot, None).await?;
    delete(tx, plan, Side::Carried, carried_id).await
}
