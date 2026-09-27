//! The input half of the crafting transaction: lock the player, re-check
//! the named instances, consume by design.

use sqlx::{Postgres, Transaction};

use super::failure::{at, expect_rows};
use super::{ConsumedStack, CraftApplied, CraftTxError, CRAFTING_INPUT_BAGS};
use crate::base::crafting::feedback::CraftReject;
use crate::base::crafting::telemetry::JobIds;

/// Lock the player's row for the rest of the transaction, so a completion
/// never interleaves with another `FOR UPDATE` writer of the player (a
/// second completion, a crafting spend, a vendor purchase).
pub(super) async fn lock_player(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
) -> Result<(), CraftTxError> {
    let found: Option<i32> =
        sqlx::query_scalar("SELECT player_id FROM sgw_player WHERE player_id = $1 FOR UPDATE")
            .bind(player_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(at("lock_player"))?;
    match found {
        Some(_) => Ok(()),
        None => Err(CraftTxError::Invalid {
            phase: "lock_player",
            reason: "player_missing",
        }),
    }
}

#[derive(sqlx::FromRow)]
struct NamedRow {
    item_id: i32,
    character_id: i32,
    container_id: i32,
}

/// Lock the named instances and check each still belongs to the player
/// and sits in the main or crafting bag. The request was validated when
/// it was queued; this is the re-check at completion, three seconds later.
pub(super) async fn check_named_items(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
    named: &[i32],
) -> Result<(), CraftTxError> {
    if named.is_empty() {
        return Ok(());
    }
    let rows: Vec<NamedRow> = sqlx::query_as(
        "SELECT item_id, character_id, container_id FROM sgw_inventory \
         WHERE item_id = ANY($1) ORDER BY item_id FOR UPDATE",
    )
    .bind(named)
    .fetch_all(&mut **tx)
    .await
    .map_err(at("check_named"))?;
    for &item_id in named {
        let Some(row) = rows
            .iter()
            .find(|r| r.item_id == item_id && r.character_id == player_id)
        else {
            return Err(CraftReject::ComponentMissing { item_id }.into());
        };
        if !CRAFTING_INPUT_BAGS.contains(&row.container_id) {
            return Err(CraftReject::ComponentNotInCraftingBags {
                item_id,
                container_id: row.container_id,
            }
            .into());
        }
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct StackRow {
    item_id: i32,
    stack_size: i32,
    container_id: i32,
}

/// Consume `quantity` of `design_id` from the main and crafting bags, the
/// crafting bag first (where the crafting pages stage components), each
/// bag in slot order. Each stack touched is recorded with its before and
/// after size; a stack consumed to nothing is deleted. Fewer than
/// `quantity` in both bags refuses the whole transaction.
pub(super) async fn consume_design(
    tx: &mut Transaction<'_, Postgres>,
    ids: &JobIds,
    design_id: i32,
    quantity: i32,
    applied: &mut CraftApplied,
) -> Result<(), CraftTxError> {
    let player_id = ids.player_id;
    if quantity <= 0 {
        // A verb that computed a non-positive cost has a bug (an overflow,
        // a forged count); skipping the cost would grant for free.
        return Err(CraftTxError::Invalid {
            phase: "consume",
            reason: "invalid_quantity",
        });
    }
    let stacks: Vec<StackRow> = sqlx::query_as(
        "SELECT item_id, stack_size, container_id FROM sgw_inventory \
         WHERE character_id = $1 AND type_id = $2 AND container_id = ANY($3) \
         ORDER BY array_position($3, container_id), slot_id FOR UPDATE",
    )
    .bind(player_id)
    .bind(design_id)
    .bind(CRAFTING_INPUT_BAGS.as_slice())
    .fetch_all(&mut **tx)
    .await
    .map_err(at("consume"))?;

    // Summed in i64: a corrupt inventory could hold more than i32::MAX.
    let available: i64 = stacks.iter().map(|s| i64::from(s.stack_size)).sum();
    if available < i64::from(quantity) {
        return Err(CraftReject::NotEnoughComponents {
            design_id,
            needed: quantity,
            available,
        }
        .into());
    }

    let mut remaining = quantity;
    for stack in stacks {
        if remaining <= 0 {
            break;
        }
        let take = remaining.min(stack.stack_size);
        let done = if take == stack.stack_size {
            sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1 AND item_id = $2")
                .bind(player_id)
                .bind(stack.item_id)
                .execute(&mut **tx)
                .await
        } else {
            sqlx::query(
                "UPDATE sgw_inventory SET stack_size = stack_size - $1 \
                 WHERE character_id = $2 AND item_id = $3",
            )
            .bind(take)
            .bind(player_id)
            .bind(stack.item_id)
            .execute(&mut **tx)
            .await
        }
        .map_err(at("consume"))?;
        expect_rows(ids, "consume", done, 1)?;
        remaining -= take;
        applied.consumed.push(ConsumedStack {
            item_id: stack.item_id,
            type_id: design_id,
            container_id: stack.container_id,
            before: stack.stack_size,
            after: stack.stack_size - take,
        });
    }
    Ok(())
}
