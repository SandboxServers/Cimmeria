//! `spendAppliedSciencePoints` (cell method 95): learn a discipline for one
//! applied science point.
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
//! whatever the tree shows, so the server decides. On success the
//! discipline is known at expertise 1 and the ASP total drops by one.
//! Learning grants no blueprints: those come from Blueprint items and
//! research. After the commit the client gets `onUpdateDiscipline` and the
//! new ASP total; a refusal gets its text line and changes nothing. The
//! client never changes its own tree or count on a click, so a refusal needs
//! no correction push.

use crate::base::crafting::telemetry as crafting_telemetry;
use cimmeria_cell_catalog::crafting::{
    racial_paradigm_name, shared_crafting_catalog, CraftingCatalog,
};
use cimmeria_entity::crafting::CraftingState;
use cimmeria_entity::known_names;
use sqlx::PgPool;

use super::feedback::{reject, CraftReject};
use super::inventory_locks::take_inventory_locks;
use super::persistence::load_crafting_state_locked;
use super::request::CraftCtx;
use super::sync::{push_asp, push_discipline};
use super::telemetry::{account_id_of, record_request, sql_error_class, Outcome};

/// A prerequisite counts once its expertise reaches this (the client's
/// tree colours use the same threshold, `DisciplineTrainer.lua:126`).
pub const PREREQUISITE_EXPERTISE: i32 = 50;

/// The expertise a newly learned discipline starts at (Python
/// `Crafter.learnDiscipline`).
pub const LEARNED_EXPERTISE: i32 = 1;

/// The cell method name: the `verb` field and metric label.
const VERB: &str = "spendAppliedSciencePoints";

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
            discipline_id,
            name: discipline.name.clone(),
        });
    }
    if state.applied_science_points < 1 {
        return Err(CraftReject::NoAppliedSciencePoints {
            asp: state.applied_science_points,
        });
    }
    let have = state
        .racial_paradigm_levels
        .get(&discipline.racial_paradigm_id)
        .map_or(0, |&level| i32::from(level));
    if have < discipline.racial_paradigm_level {
        return Err(CraftReject::ParadigmTooLow {
            discipline_id,
            discipline: discipline.name.clone(),
            paradigm_id: discipline.racial_paradigm_id,
            paradigm: racial_paradigm_name(discipline.racial_paradigm_id).unwrap_or("an unknown"),
            required: discipline.racial_paradigm_level,
            have,
        });
    }
    for &prerequisite_id in &discipline.required_discipline_ids {
        let prerequisite = catalog.disciplines.get(&prerequisite_id).map_or_else(
            || format!("discipline {prerequisite_id}"),
            |d| d.name.clone(),
        );
        if !state.knows_discipline(prerequisite_id) {
            return Err(CraftReject::PrerequisiteMissing {
                discipline_id,
                discipline: discipline.name.clone(),
                prerequisite_id,
                prerequisite,
            });
        }
        let expertise = state.get_expertise(prerequisite_id).unwrap_or(0);
        if expertise < PREREQUISITE_EXPERTISE {
            return Err(CraftReject::PrerequisiteExpertise {
                discipline_id,
                discipline: discipline.name.clone(),
                prerequisite_id,
                prerequisite,
                expertise,
                required: PREREQUISITE_EXPERTISE,
            });
        }
    }
    Ok(())
}

/// A committed learn: the discipline's expertise and the ASP total either
/// side of it. `expertise_before` is 0 unless a stray expertise row existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Learned {
    pub expertise_before: i32,
    pub asp_before: i32,
    pub asp_after: i32,
}

/// A spend transaction that failed and rolled back, with the sub-step it
/// failed in.
#[derive(Debug)]
pub struct SpendFailure {
    /// `begin`, `lock_player`, `update_player`, `upsert_expertise` or
    /// `commit`.
    pub phase: &'static str,
    /// The database error; `None` when a statement touched fewer rows than
    /// it had to (then `rows_affected` says how many).
    pub error: Option<sqlx::Error>,
    pub rows_affected: u64,
}

impl SpendFailure {
    fn sql(phase: &'static str) -> impl FnOnce(sqlx::Error) -> Self {
        move |e| SpendFailure {
            phase,
            error: Some(e),
            rows_affected: 0,
        }
    }

    fn short(phase: &'static str, rows_affected: u64) -> Self {
        SpendFailure {
            phase,
            error: None,
            rows_affected,
        }
    }
}

/// The spend transaction. `Ok(Ok(learned))` is a committed learn;
/// `Ok(Err(why))` a refusal, rolled back with nothing written; `Err` a
/// failure, also rolled back.
pub async fn spend_in_db(
    pool: &PgPool,
    catalog: &CraftingCatalog,
    player_id: i32,
    discipline_id: i32,
) -> Result<Result<Learned, CraftReject>, SpendFailure> {
    let mut tx = pool.begin().await.map_err(SpendFailure::sql("begin"))?;
    // The player-wide inventory key before the player row, the order every
    // crafting write uses: an induction completion takes this key and
    // changes expertise without locking the player row, so the spend's
    // prerequisite check waits for it and reads the committed expertise.
    take_inventory_locks(&mut tx, player_id, &[])
        .await
        .map_err(SpendFailure::sql("advisory_lock"))?;
    let Some(state) = load_crafting_state_locked(&mut tx, player_id)
        .await
        .map_err(SpendFailure::sql("lock_player"))?
    else {
        // The cell only forwards requests for a loaded player, so a missing
        // row means the character was deleted under a live session.
        return Err(SpendFailure::short("lock_player", 0));
    };
    if let Err(why) = check_spend(&state, catalog, discipline_id) {
        return Ok(Err(why));
    }

    // Targeted writes under the row lock, rather than rewriting the whole
    // state: the checks above ran against exactly this row.
    let asp_after: i32 = sqlx::query_scalar(
        "UPDATE sgw_player \
         SET discipline_ids = array_append(discipline_ids, $2), \
             applied_science_points = applied_science_points - 1, \
             applied_science_points_spent = applied_science_points_spent + 1 \
         WHERE player_id = $1 \
         RETURNING applied_science_points",
    )
    .bind(player_id)
    .bind(discipline_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(SpendFailure::sql("update_player"))?;
    // A stray expertise row (the persistence layer loads them) is reset:
    // learning starts the discipline at 1.
    let upserted = sqlx::query(
        "INSERT INTO sgw_player_discipline_expertise (player_id, discipline_id, expertise) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (player_id, discipline_id) DO UPDATE SET expertise = EXCLUDED.expertise",
    )
    .bind(player_id)
    .bind(discipline_id)
    .bind(LEARNED_EXPERTISE)
    .execute(&mut *tx)
    .await
    .map_err(SpendFailure::sql("upsert_expertise"))?;
    if upserted.rows_affected() != 1 {
        return Err(SpendFailure::short(
            "upsert_expertise",
            upserted.rows_affected(),
        ));
    }
    tx.commit().await.map_err(SpendFailure::sql("commit"))?;
    Ok(Ok(Learned {
        expertise_before: state.get_expertise(discipline_id).unwrap_or(0),
        asp_before: state.applied_science_points,
        asp_after,
    }))
}

/// Handle a `Spend` request: decide it in [`spend_in_db`], then push the
/// result or send the refusal. Events: `learned` on success, `rejected`
/// (from [`reject`]) on a refusal, `persist_failed` (WARN) on a failure.
pub async fn handle_spend(entity_id: u32, player_id: i32, discipline_id: i32, ctx: &CraftCtx<'_>) {
    let client = ctx.client();
    let account_id = account_id_of(entity_id, ctx.connected, ctx.entity_to_addr);
    let unavailable = CraftReject::Unavailable { action: ACTION };
    let Some(pool) = ctx.db_pool else {
        tracing::warn!(
            target: "crafting",
            event = "persist_failed",
            verb = VERB,
            phase = "no_pool",
            account_id,
            account_name = known_names::account_name(account_id),
            player_id,
            player_name = known_names::player_name(player_id),
            entity_id,
            entity_name = known_names::player_name(player_id),
            discipline_id,
            discipline_name = crafting_telemetry::discipline_name(discipline_id),
            "spend: no database pool"
        );
        reject(VERB, entity_id, player_id, &unavailable, client).await;
        return;
    };
    let catalog = match shared_crafting_catalog(pool).await {
        Ok(catalog) => catalog,
        Err(e) => {
            tracing::warn!(
                target: "crafting",
                event = "persist_failed",
                verb = VERB,
                phase = "catalog_load",
                account_id,
                account_name = known_names::account_name(account_id),
                player_id,
                player_name = known_names::player_name(player_id),
                entity_id,
                entity_name = known_names::player_name(player_id),
                discipline_id,
                discipline_name = crafting_telemetry::discipline_name(discipline_id),
                error_class = sql_error_class(&e),
                error = %e,
                "spend: crafting catalog load failed"
            );
            reject(VERB, entity_id, player_id, &unavailable, client).await;
            return;
        }
    };

    match spend_in_db(pool, &catalog, player_id, discipline_id).await {
        Ok(Ok(Learned {
            expertise_before,
            asp_before,
            asp_after,
        })) => {
            tracing::info!(
                target: "crafting",
                event = "learned",
                verb = VERB,
                account_id,
                account_name = known_names::account_name(account_id),
                player_id,
                player_name = known_names::player_name(player_id),
                entity_id,
                entity_name = known_names::player_name(player_id),
                discipline_id,
                discipline_name = crafting_telemetry::discipline_name(discipline_id),
                expertise_before,
                expertise_after = LEARNED_EXPERTISE,
                asp_before,
                asp_after,
                "discipline learned"
            );
            record_request(VERB, Outcome::Accepted);
            push_discipline(
                entity_id,
                player_id,
                discipline_id,
                LEARNED_EXPERTISE,
                client,
            )
            .await;
            push_asp(entity_id, player_id, asp_after, client).await;
        }
        Ok(Err(why)) => reject(VERB, entity_id, player_id, &why, client).await,
        Err(failure) => {
            match &failure.error {
                Some(e) => tracing::warn!(
                    target: "crafting",
                    event = "persist_failed",
                    verb = VERB,
                    phase = failure.phase,
                    account_id,
                    account_name = known_names::account_name(account_id),
                    player_id,
                    player_name = known_names::player_name(player_id),
                    entity_id,
                    entity_name = known_names::player_name(player_id),
                    discipline_id,
                    discipline_name = crafting_telemetry::discipline_name(discipline_id),
                    error_class = sql_error_class(e),
                    error = %e,
                    "spend: transaction failed, rolled back"
                ),
                None => tracing::warn!(
                    target: "crafting",
                    event = "persist_failed",
                    verb = VERB,
                    phase = failure.phase,
                    reason = "rows_affected_short",
                    rows_affected = failure.rows_affected,
                    expected = 1u64,
                    account_id,
                    account_name = known_names::account_name(account_id),
                    player_id,
                    player_name = known_names::player_name(player_id),
                    entity_id,
                    entity_name = known_names::player_name(player_id),
                    discipline_id,
                    discipline_name = crafting_telemetry::discipline_name(discipline_id),
                    "spend: a write touched fewer rows than it had to, rolled back"
                ),
            }
            reject(VERB, entity_id, player_id, &unavailable, client).await;
        }
    }
}

#[cfg(test)]
mod tests;
