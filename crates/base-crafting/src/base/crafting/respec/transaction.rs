//! The respec transaction: clear every discipline and its expertise and
//! refund the applied science points spent on disciplines since the last
//! respec, under the `sgw_player` row lock.

use sqlx::PgPool;

use crate::base::crafting::feedback::CraftReject;
use crate::base::crafting::inventory_locks::take_inventory_locks;
use crate::base::crafting::persistence::load_crafting_state_locked;

/// A committed respec: what was cleared and what was kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Respecced {
    /// Each known discipline and its expertise before the respec, in
    /// learning order.
    pub cleared: Vec<(i32, i32)>,
    /// The ASP refunded: what the player spent on disciplines since the
    /// last respec, never more. A GM-granted discipline refunds nothing.
    pub refund: i32,
    /// Expertise rows deleted, including any for a discipline that was not
    /// in the known list.
    pub expertise_rows_deleted: u64,
    pub asp_before: i32,
    pub asp_after: i32,
    /// The known blueprints, untouched.
    pub blueprint_ids: Vec<i32>,
    /// The racial paradigm levels, untouched, by paradigm id.
    pub paradigm_levels: Vec<(i32, i8)>,
}

/// A respec transaction that failed and rolled back, with the sub-step it
/// failed in.
#[derive(Debug)]
pub struct RespecFailure {
    /// `begin`, `advisory_lock`, `lock_player`, `read_spent`,
    /// `update_player`, `delete_expertise` or `commit`.
    pub phase: &'static str,
    /// The database error; `None` when the player row was missing (then
    /// `rows_affected` is 0 against an expected 1).
    pub error: Option<sqlx::Error>,
    pub rows_affected: u64,
}

impl RespecFailure {
    fn sql(phase: &'static str) -> impl FnOnce(sqlx::Error) -> Self {
        move |e| RespecFailure {
            phase,
            error: Some(e),
            rows_affected: 0,
        }
    }
}

/// The respec transaction. `Ok(Ok(respecced))` is a committed respec;
/// `Ok(Err(why))` a refusal, rolled back with nothing written; `Err` a
/// failure, also rolled back.
///
/// Blueprints and racial paradigm levels are not written: they come from
/// items the player used, not from disciplines, so a respec keeps them.
pub async fn respec_in_db(
    pool: &PgPool,
    player_id: i32,
) -> Result<Result<Respecced, CraftReject>, RespecFailure> {
    let mut tx = pool.begin().await.map_err(RespecFailure::sql("begin"))?;
    // Lock order for transactions that touch items is advisory keys, then
    // item rows, then the `sgw_player` row. A respec touches no items, but
    // it takes the player-wide inventory key first, as an item transaction
    // does, so it queues behind a completion that holds item
    // rows instead of taking the player row that completion will wait for.
    take_inventory_locks(&mut tx, player_id, &[])
        .await
        .map_err(RespecFailure::sql("advisory_lock"))?;
    let Some(state) = load_crafting_state_locked(&mut tx, player_id)
        .await
        .map_err(RespecFailure::sql("lock_player"))?
    else {
        return Err(RespecFailure {
            phase: "lock_player",
            error: None,
            rows_affected: 0,
        });
    };
    if state.discipline_ids.is_empty() && state.expertise.is_empty() {
        return Ok(Err(CraftReject::NothingToRespec));
    }

    // Refund what was paid, not what is held: a GM grant or `.allcraft`
    // adds disciplines without charging, and refunding those would mint
    // points. The row is locked, so the counter cannot move under us.
    let spent: i32 = sqlx::query_scalar(
        "SELECT applied_science_points_spent FROM sgw_player WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(RespecFailure::sql("read_spent"))?;
    let refund = spent.max(0);

    let mut cleared: Vec<(i32, i32)> = Vec::with_capacity(state.discipline_ids.len());
    for &discipline_id in &state.discipline_ids {
        if !cleared.iter().any(|&(id, _)| id == discipline_id) {
            cleared.push((
                discipline_id,
                state.get_expertise(discipline_id).unwrap_or(0),
            ));
        }
    }
    let asp_after: i32 = sqlx::query_scalar(
        "UPDATE sgw_player \
         SET discipline_ids = '{}', \
             applied_science_points = LEAST(applied_science_points::bigint + $2, 2147483647)::int, \
             applied_science_points_spent = 0 \
         WHERE player_id = $1 \
         RETURNING applied_science_points",
    )
    .bind(player_id)
    .bind(i64::from(refund))
    .fetch_one(&mut *tx)
    .await
    .map_err(RespecFailure::sql("update_player"))?;
    let deleted = sqlx::query("DELETE FROM sgw_player_discipline_expertise WHERE player_id = $1")
        .bind(player_id)
        .execute(&mut *tx)
        .await
        .map_err(RespecFailure::sql("delete_expertise"))?;
    tx.commit().await.map_err(RespecFailure::sql("commit"))?;

    let mut paradigm_levels: Vec<(i32, i8)> = state
        .racial_paradigm_levels
        .iter()
        .map(|(&id, &level)| (id, level))
        .collect();
    paradigm_levels.sort_unstable_by_key(|&(id, _)| id);
    Ok(Ok(Respecced {
        cleared,
        refund,
        expertise_rows_deleted: deleted.rows_affected(),
        asp_before: state.applied_science_points,
        asp_after,
        blueprint_ids: state.blueprint_ids,
        paradigm_levels,
    }))
}
