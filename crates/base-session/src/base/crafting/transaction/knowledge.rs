//! The crafting knowledge a completion still needs, checked inside the
//! transaction.
//!
//! A respec (or any other write that forgets a blueprint or discipline)
//! updates the `sgw_player` row. Reading that row `FOR SHARE` here makes
//! the two serialize: a respec that committed first is seen, and one that
//! comes later waits for this commit. A check outside the transaction
//! could pass on the old state and then craft anyway.
//!
//! Lock order: this runs after the advisory locks and every inventory row
//! (consumption and placement), which is the order the vendor stack takes
//! (rows, then the player row). The player-wide advisory lock the
//! completion already holds keeps trades and purchases out.

use sqlx::{Postgres, Transaction};

use super::failure::{at, CraftTxError};
use super::plan::RequiredKnowledge;
use crate::base::crafting::feedback::CraftReject;

pub(super) async fn check_knowledge(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
    required: RequiredKnowledge,
) -> Result<(), CraftTxError> {
    let row: Option<(Vec<i32>, Vec<i32>)> = sqlx::query_as(
        "SELECT blueprint_ids, discipline_ids FROM sgw_player WHERE player_id = $1 FOR SHARE",
    )
    .bind(player_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(at("knowledge"))?;
    let Some((blueprints, disciplines)) = row else {
        return Err(CraftTxError::Invalid {
            phase: "knowledge",
            reason: "player_missing",
        });
    };
    let RequiredKnowledge {
        blueprint_id,
        discipline_id,
    } = required;
    if !blueprints.contains(&blueprint_id) {
        return Err(CraftReject::UnknownBlueprint { blueprint_id }.into());
    }
    if !disciplines.contains(&discipline_id) {
        return Err(CraftReject::DisciplineUnknown {
            blueprint_id,
            discipline_id,
        }
        .into());
    }
    Ok(())
}
