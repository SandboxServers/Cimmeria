//! The request-time read of the item instances a crafting request names.
//!
//! A verb validates at request and consumes at completion, where the
//! transaction checks every named instance again under its row lock. This
//! read is the first check: it answers the player at once when an item is
//! gone or sits outside the crafting bags, instead of three seconds later.

use sqlx::PgPool;

use super::feedback::CraftReject;
use super::transaction::CRAFTING_INPUT_BAGS;

/// One named instance as the request found it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeldInstance {
    pub item_id: i32,
    pub type_id: i32,
    pub container_id: i32,
    pub stack_size: i32,
}

/// Read `item_ids` for `player_id`, in request order. The inner `Err` is
/// the refusal for the first instance that is not the player's
/// (`ComponentMissing`) or not in the main or crafting bag
/// (`ComponentNotInCraftingBags`).
pub async fn held_instances(
    pool: &PgPool,
    player_id: i32,
    item_ids: &[i32],
) -> Result<Result<Vec<HeldInstance>, CraftReject>, sqlx::Error> {
    let rows: Vec<(i32, i32, i32, i32)> = sqlx::query_as(
        "SELECT item_id, type_id, container_id, stack_size FROM sgw_inventory \
         WHERE character_id = $1 AND item_id = ANY($2)",
    )
    .bind(player_id)
    .bind(item_ids)
    .fetch_all(pool)
    .await?;
    Ok(in_request_order(&rows, item_ids))
}

fn in_request_order(
    rows: &[(i32, i32, i32, i32)],
    item_ids: &[i32],
) -> Result<Vec<HeldInstance>, CraftReject> {
    item_ids
        .iter()
        .map(|&item_id| {
            let Some(&(_, type_id, container_id, stack_size)) =
                rows.iter().find(|r| r.0 == item_id)
            else {
                return Err(CraftReject::ComponentMissing { item_id });
            };
            if !CRAFTING_INPUT_BAGS.contains(&container_id) {
                return Err(CraftReject::ComponentNotInCraftingBags {
                    item_id,
                    container_id,
                });
            }
            Ok(HeldInstance {
                item_id,
                type_id,
                container_id,
                stack_size,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instances_come_back_in_request_order() {
        let rows = [(2, 20, 1, 1), (1, 10, 15, 3)];
        let held = in_request_order(&rows, &[1, 2]).unwrap();
        assert_eq!(held.iter().map(|h| h.item_id).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(held[0].type_id, 10);
        assert_eq!(held[0].stack_size, 3);
    }

    #[test]
    fn a_missing_or_banked_instance_is_refused() {
        let rows = [(1, 10, 17, 1)];
        assert_eq!(
            in_request_order(&rows, &[2]),
            Err(CraftReject::ComponentMissing { item_id: 2 })
        );
        assert_eq!(
            in_request_order(&rows, &[1]),
            Err(CraftReject::ComponentNotInCraftingBags {
                item_id: 1,
                container_id: 17
            })
        );
    }
}
