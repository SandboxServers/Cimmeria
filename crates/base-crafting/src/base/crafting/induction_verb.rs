//! What every induction verb does around its own rule: read the catalog
//! and the named items at the request, submit the job and count the
//! answer, read the crafting state when the job completes, and tell the
//! player the result.

use cimmeria_entity::known_names;
use std::sync::Arc;

use cimmeria_cell_catalog::crafting::{shared_crafting_catalog, CraftingCatalog};
use cimmeria_entity::crafting::CraftingState;
use sqlx::PgPool;

use super::feedback::{feedback_text_args, reject, reject_at_completion, CraftReject};
use super::item_lookup::{held_instances, HeldInstance};
use super::persistence::load_crafting_state;
use super::request::CraftCtx;
use super::session::{crafting_sessions, InductionEnv, InductionJob, SubmitOutcome};
use super::telemetry::{
    account_id_of, record_request, send_to_player, sql_error_class, JobIds, Outcome,
};
use crate::mercury::method_idx;

/// What a verb's request needs from the database.
pub struct RequestInputs {
    pub pool: Arc<PgPool>,
    pub catalog: Arc<CraftingCatalog>,
    /// The named instances, in the order they were asked for.
    pub held: Vec<HeldInstance>,
}

/// Read the catalog and the instances `item_ids` names. On a refusal
/// (an instance gone or outside the crafting bags) or a server failure
/// (`lookup_failed` WARN, then `Unavailable`) the player has been answered
/// and `None` comes back.
pub async fn load_request_inputs(
    verb: &'static str,
    action: &'static str,
    entity_id: u32,
    player_id: i32,
    item_ids: &[i32],
    ctx: &CraftCtx<'_>,
) -> Option<RequestInputs> {
    let unavailable = CraftReject::Unavailable { action };
    let lookup_failed = |phase: &'static str, error_class: &'static str, error: String| {
        let account_id = account_id_of(entity_id, ctx.connected, ctx.entity_to_addr);
        let player_label = known_names::player_name(player_id);
        tracing::warn!(
            target: "crafting",
            event = "lookup_failed",
            verb,
            phase,
            account_id,
            account_name = known_names::account_name(account_id),
            player_id,
            player_name = player_label,
            entity_id,
            entity_name = player_label,
            error_class,
            error,
            "crafting request could not be read; refused as unavailable"
        );
    };
    let Some(pool) = ctx.db_pool.clone() else {
        lookup_failed("no_pool", "no_database", String::new());
        reject(verb, entity_id, player_id, &unavailable, ctx.client()).await;
        return None;
    };
    let catalog = match shared_crafting_catalog(&pool).await {
        Ok(catalog) => catalog,
        Err(e) => {
            lookup_failed("catalog_load", sql_error_class(&e), e.to_string());
            reject(verb, entity_id, player_id, &unavailable, ctx.client()).await;
            return None;
        }
    };
    let held = match held_instances(&pool, player_id, item_ids).await {
        Ok(Ok(held)) => held,
        Ok(Err(why)) => {
            reject(verb, entity_id, player_id, &why, ctx.client()).await;
            return None;
        }
        Err(e) => {
            lookup_failed("named_items", sql_error_class(&e), e.to_string());
            reject(verb, entity_id, player_id, &unavailable, ctx.client()).await;
            return None;
        }
    };
    Some(RequestInputs {
        pool,
        catalog,
        held,
    })
}

/// Hand `job` to the induction engine and count the request as accepted
/// when it started or queued. A full queue was already refused and counted
/// by the engine; an entity with no session has no one to answer.
pub async fn submit_job(
    entity_id: u32,
    player_id: i32,
    job: Box<dyn InductionJob>,
    ctx: &CraftCtx<'_>,
) -> SubmitOutcome {
    let verb = job.verb();
    let env = InductionEnv::from_ctx(ctx);
    let outcome = crafting_sessions()
        .submit(entity_id, player_id, job, &env)
        .await;
    if matches!(
        outcome,
        SubmitOutcome::Started | SubmitOutcome::Queued { .. }
    ) {
        record_request(verb, Outcome::Accepted);
    }
    outcome
}

/// The player's crafting state when a job completes, read without a lock
/// (the transaction re-checks what it writes). A read failure is a
/// `lookup_failed` WARN and refuses the job with `InductionFailed`.
pub async fn state_at_completion(env: &InductionEnv, ids: &JobIds) -> Option<CraftingState> {
    let result = match env.db_pool.as_ref() {
        Some(pool) => load_crafting_state(pool, ids.player_id)
            .await
            .map_err(|e| (sql_error_class(&e), e.to_string())),
        None => Err(("no_database", String::new())),
    };
    match result {
        Ok(state) => Some(state),
        Err((error_class, error)) => {
            let player_label = known_names::player_name(ids.player_id);
            tracing::warn!(
                target: "crafting",
                event = "lookup_failed",
                verb = ids.verb,
                phase = "completion_state",
                job_id = ids.job_id, // nt:id-only induction job counter, unnamed
                account_id = ids.account_id,
                account_name = known_names::account_name(ids.account_id),
                player_id = ids.player_id,
                player_name = player_label,
                entity_id = ids.entity_id,
                entity_name = player_label,
                error_class,
                error,
                "crafting state unreadable at completion; nothing was used"
            );
            reject_at_completion(ids, &CraftReject::InductionFailed, env.client()).await;
            None
        }
    }
}

/// Send the player the line that says how a completed job went.
pub async fn send_result_line(env: &InductionEnv, ids: &JobIds, text: &str) {
    send_to_player(
        env,
        ids,
        method_idx::ON_PLAYER_COMMUNICATION,
        &feedback_text_args(text),
        "result_line",
    )
    .await;
}

#[cfg(test)]
#[path = "induction_verb_tests.rs"]
mod tests;
