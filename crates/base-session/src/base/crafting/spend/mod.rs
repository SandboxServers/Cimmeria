//! `spendAppliedSciencePoints` (cell method 95): learn a discipline for one
//! applied science point (CR-04).
//!
//! One transaction locks the player's `sgw_player` row `FOR UPDATE`, then
//! checks, in this order ([`check_spend`]):
//!
//! 1. the discipline exists in the catalog;
//! 2. it is not already known (so a replayed request changes nothing);
//! 3. the player has at least one unspent ASP;
//! 4. the discipline's racial paradigm is at its required level;
//! 5. every required discipline is known at expertise 50 or more.
//!
//! These are the rules the client's discipline trainer draws with
//! (`DisciplineTrainer.lua:117-128`), but the client sends 95 for any click
//! (audit C-36), so the server decides. On success the discipline is known
//! at expertise 1 and the ASP total drops by one. Learning grants no
//! blueprints (D-CR04). After the commit the client gets
//! `onUpdateDiscipline` and the new ASP total; a refusal gets its text line
//! (D-CR14) and changes nothing. The client never changes its own tree or
//! count on a click, so a refusal needs no correction push.

use cimmeria_cell_catalog::crafting::{
    racial_paradigm_name, shared_crafting_catalog, CraftingCatalog,
};
use cimmeria_entity::crafting::CraftingState;
use sqlx::PgPool;

use super::feedback::{reject, CraftReject};
use super::persistence::load_crafting_state_locked;
use super::request::CraftCtx;
use super::sync::{push_asp, push_discipline, CraftClient};

/// A prerequisite counts once its expertise reaches this (the client's
/// tree colours use the same threshold, `DisciplineTrainer.lua:126`).
pub const PREREQUISITE_EXPERTISE: i32 = 50;

/// The expertise a newly learned discipline starts at (Python
/// `Crafter.learnDiscipline`).
pub const LEARNED_EXPERTISE: i32 = 1;

/// The text subject for this verb's `Unavailable` rejection.
const ACTION: &str = "Learning disciplines";

/// Decide whether `state` may learn `discipline_id`. Pure: the transaction
/// in [`spend_in_db`] supplies the locked state.
pub fn check_spend(
    state: &CraftingState,
    catalog: &CraftingCatalog,
    discipline_id: i32,
) -> Result<(), CraftReject> {
    let Some(discipline) = catalog.disciplines.get(&discipline_id) else {
        return Err(CraftReject::UnknownDiscipline { discipline_id });
    };
    if state.knows_discipline(discipline_id) {
        return Err(CraftReject::DisciplineAlreadyKnown {
            name: discipline.name.clone(),
        });
    }
    if state.applied_science_points < 1 {
        return Err(CraftReject::NoAppliedSciencePoints);
    }
    let have = state
        .racial_paradigm_levels
        .get(&discipline.racial_paradigm_id)
        .map_or(0, |&level| i32::from(level));
    if have < discipline.racial_paradigm_level {
        return Err(CraftReject::ParadigmTooLow {
            discipline: discipline.name.clone(),
            paradigm: racial_paradigm_name(discipline.racial_paradigm_id).unwrap_or("an unknown"),
            required: discipline.racial_paradigm_level,
            have,
        });
    }
    for &prerequisite in &discipline.required_discipline_ids {
        let met = state.knows_discipline(prerequisite)
            && state.get_expertise(prerequisite).unwrap_or(0) >= PREREQUISITE_EXPERTISE;
        if !met {
            let prerequisite = catalog
                .disciplines
                .get(&prerequisite)
                .map_or_else(|| format!("discipline {prerequisite}"), |d| d.name.clone());
            return Err(CraftReject::PrerequisiteMissing {
                discipline: discipline.name.clone(),
                prerequisite,
            });
        }
    }
    Ok(())
}

/// The spend transaction. `Ok(Ok(total))` is the new unspent ASP total
/// after a committed learn; `Ok(Err(why))` a refusal, rolled back with
/// nothing written; `Err` a database failure, also rolled back.
pub async fn spend_in_db(
    pool: &PgPool,
    catalog: &CraftingCatalog,
    player_id: i32,
    discipline_id: i32,
) -> Result<Result<i32, CraftReject>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let Some(state) = load_crafting_state_locked(&mut tx, player_id).await? else {
        // The cell only forwards requests for a loaded player, so a missing
        // row means the character was deleted under a live session.
        return Err(sqlx::Error::RowNotFound);
    };
    if let Err(why) = check_spend(&state, catalog, discipline_id) {
        return Ok(Err(why));
    }

    // Targeted writes under the row lock, rather than rewriting the whole
    // state: the checks above ran against exactly this row.
    let total: i32 = sqlx::query_scalar(
        "UPDATE sgw_player \
         SET discipline_ids = array_append(discipline_ids, $2), \
             applied_science_points = applied_science_points - 1 \
         WHERE player_id = $1 \
         RETURNING applied_science_points",
    )
    .bind(player_id)
    .bind(discipline_id)
    .fetch_one(&mut *tx)
    .await?;
    // A stray expertise row (the persistence layer loads them, audit C-02)
    // is reset: learning starts the discipline at 1.
    sqlx::query(
        "INSERT INTO sgw_player_discipline_expertise (player_id, discipline_id, expertise) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (player_id, discipline_id) DO UPDATE SET expertise = EXCLUDED.expertise",
    )
    .bind(player_id)
    .bind(discipline_id)
    .bind(LEARNED_EXPERTISE)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Ok(total))
}

/// Handle a `Spend` request: decide it in [`spend_in_db`], then push the
/// result or send the refusal.
pub async fn handle_spend(entity_id: u32, player_id: i32, discipline_id: i32, ctx: &CraftCtx<'_>) {
    let refuse = |why: CraftReject| async move {
        reject(
            entity_id,
            player_id,
            &why,
            ctx.transport,
            ctx.connected,
            ctx.entity_to_addr,
        )
        .await;
    };
    let Some(pool) = ctx.db_pool else {
        tracing::warn!(
            target: "crafting",
            event = "persist_failed",
            op = "spend",
            entity_id,
            player_id,
            discipline_id,
            "spend: no database pool"
        );
        refuse(CraftReject::Unavailable { action: ACTION }).await;
        return;
    };
    let catalog = match shared_crafting_catalog(pool).await {
        Ok(catalog) => catalog,
        Err(e) => {
            tracing::warn!(
                target: "crafting",
                event = "persist_failed",
                op = "catalog_load",
                entity_id,
                player_id,
                error = %e,
                "spend: crafting catalog load failed"
            );
            refuse(CraftReject::Unavailable { action: ACTION }).await;
            return;
        }
    };

    match spend_in_db(pool, &catalog, player_id, discipline_id).await {
        Ok(Ok(total)) => {
            tracing::info!(
                target: "crafting",
                event = "completed",
                verb = "spend",
                entity_id,
                player_id,
                discipline_id,
                asp = total,
                "discipline learned"
            );
            let client = CraftClient {
                transport: ctx.transport,
                connected: ctx.connected,
                entity_to_addr: ctx.entity_to_addr,
            };
            push_discipline(entity_id, discipline_id, LEARNED_EXPERTISE, client).await;
            push_asp(entity_id, total, client).await;
        }
        Ok(Err(why)) => refuse(why).await,
        Err(e) => {
            tracing::warn!(
                target: "crafting",
                event = "persist_failed",
                op = "spend",
                entity_id,
                player_id,
                discipline_id,
                error = %e,
                "spend: transaction failed, rolled back"
            );
            refuse(CraftReject::Unavailable { action: ACTION }).await;
        }
    }
}

#[cfg(test)]
mod tests;
