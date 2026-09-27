//! The blueprint half of the crafting transaction: add the blueprints a
//! plan teaches to the player's known list.

use sqlx::{Postgres, Transaction};

use super::failure::{at, expect_rows};
use super::{BlueprintsLearned, CraftApplied, CraftTxError};
use crate::base::crafting::telemetry::JobIds;

/// Add every `(blueprint_id, discipline_id)` in `teach` whose discipline
/// the player knows and whose blueprint they do not know yet to
/// `sgw_player.blueprint_ids`, keeping the list sorted. The discipline is
/// checked here, under the row lock, because the plan was built from a
/// read taken before the transaction: a discipline dropped in between
/// teaches nothing. Nothing is written when no blueprint qualifies, and
/// `applied.blueprints` stays `None`.
///
/// This is the one step that locks `sgw_player`, and it runs after every
/// inventory row the transaction touches is locked: the vendor paths take
/// an item row then the player row, so taking the player row last never
/// waits on a row a vendor path holds while holding the player row. The
/// expertise rows are locked after it, the order the discipline spend and
/// the respec use (player row, then expertise).
pub(super) async fn teach_blueprints(
    tx: &mut Transaction<'_, Postgres>,
    ids: &JobIds,
    teach: &[(i32, i32)],
    applied: &mut CraftApplied,
) -> Result<(), CraftTxError> {
    let row: Option<(Vec<i32>, Vec<i32>)> = sqlx::query_as(
        "SELECT blueprint_ids, discipline_ids FROM sgw_player WHERE player_id = $1 FOR UPDATE",
    )
    .bind(ids.player_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(at("learn_blueprints"))?;
    let Some((before, disciplines)) = row else {
        return Err(CraftTxError::Invalid {
            phase: "learn_blueprints",
            reason: "player_missing",
        });
    };
    let mut taught: Vec<i32> = teach
        .iter()
        .filter(|(id, discipline)| disciplines.contains(discipline) && !before.contains(id))
        .map(|&(id, _)| id)
        .collect();
    taught.sort_unstable();
    taught.dedup();
    if taught.is_empty() {
        return Ok(());
    }
    let mut after = before.clone();
    after.extend(&taught);
    after.sort_unstable();
    let done = sqlx::query("UPDATE sgw_player SET blueprint_ids = $2 WHERE player_id = $1")
        .bind(ids.player_id)
        .bind(&after)
        .execute(&mut **tx)
        .await
        .map_err(at("learn_blueprints"))?;
    expect_rows(ids, "learn_blueprints", done, 1)?;
    applied.blueprints = Some(BlueprintsLearned {
        taught,
        known_before: before.len(),
        blueprint_ids: after,
    });
    Ok(())
}
