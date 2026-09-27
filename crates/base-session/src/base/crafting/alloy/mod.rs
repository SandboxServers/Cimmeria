//! `alloying` (cell method 99): turn one current-tier component plus
//! elementary components one tier below it into the alloy blueprint's
//! product.
//!
//! The request is validated when it arrives ([`rules::check_alloy`] over
//! the player's crafting state, the catalog and the named item rows) and
//! nothing is consumed then. A valid alloy is queued as an induction
//! ([`job::AlloyJob`]); when its bar ends, one transaction re-checks the
//! named instances, consumes them, grants the product and adds one
//! expertise to the blueprint's discipline.
//!
//! Every refusal sends its line and then a full inventory resync, because
//! the alloy page empties its slots on confirm (`AlloyPage.lua:88-97`).
//! The station gate ran before this verb: alloying needs a station, never a
//! tool.

use std::sync::Arc;

use cimmeria_cell_catalog::crafting::{shared_crafting_catalog, CraftingCatalog};
use cimmeria_entity::crafting::CraftingState;
use sqlx::PgPool;

use super::feedback::{reject, CraftReject};
use super::persistence::load_crafting_state;
use super::request::CraftCtx;
use super::session::{crafting_sessions, CraftingSessions, InductionEnv, SubmitOutcome};
use super::telemetry::{account_id_of, record_request, sql_error_class, JobIds, Outcome};
use super::transaction::{resync_inventory, CRAFTING_INPUT_BAGS};

pub mod job;
pub mod rules;

#[cfg(test)]
mod tests;

pub use job::AlloyJob;
pub use rules::{check_alloy, AlloyCheck, AlloyPlan, AlloyRequest, HeldItem, MAX_ELEMENTARY_ITEMS};

/// The cell method name: the `verb` field and metric label.
pub const VERB: &str = "alloying";

/// The text subject for this verb's `Unavailable` rejection.
const ACTION: &str = "Alloying";

/// Handle an `alloying` request on the server's induction engine.
pub async fn handle_alloy(
    entity_id: u32,
    player_id: i32,
    request: AlloyRequest<'_>,
    ctx: &CraftCtx<'_>,
) {
    handle_alloy_in(crafting_sessions(), entity_id, player_id, request, ctx).await;
}

/// [`handle_alloy`] on a given engine. Refusals log `rejected`; a data or
/// database fault logs `lookup_failed` (WARN) and answers "unavailable";
/// an accepted alloy is counted once the engine has started or queued it.
pub async fn handle_alloy_in(
    sessions: &Arc<CraftingSessions>,
    entity_id: u32,
    player_id: i32,
    request: AlloyRequest<'_>,
    ctx: &CraftCtx<'_>,
) {
    let account_id = account_id_of(entity_id, ctx.connected, ctx.entity_to_addr);
    if request.lower_tier_items.len() > MAX_ELEMENTARY_ITEMS {
        // Only a forged packet names more ids than the page has slots:
        // dropped like any other malformed crafting request, with no line.
        tracing::warn!(
            target: "crafting",
            event = "malformed",
            verb = VERB,
            reason = "too_many_elementary_items",
            account_id,
            player_id,
            entity_id,
            count = request.lower_tier_items.len(),
            limit = MAX_ELEMENTARY_ITEMS,
            "alloy request names more elementary items than the page has slots; dropped"
        );
        return;
    }
    let refuse = |why: CraftReject| async move {
        reject(VERB, entity_id, player_id, &why, ctx.client()).await;
        if let Some(pool) = ctx.db_pool {
            let ids = JobIds {
                job_id: 0,
                verb: VERB,
                account_id: account_id.unwrap_or(0),
                player_id,
                entity_id,
            };
            resync_inventory(&InductionEnv::from_ctx(ctx), pool, &ids).await;
        }
    };
    let unavailable = CraftReject::Unavailable { action: ACTION };
    let lookup_failed = |phase: &'static str, id: i32, error: &str, class: &'static str| {
        tracing::warn!(
            target: "crafting",
            event = "lookup_failed",
            verb = VERB,
            phase,
            account_id,
            player_id,
            entity_id,
            blueprint_id = request.blueprint_id,
            id,
            error_class = class,
            error,
            "alloy: lookup failed; request answered as unavailable"
        );
    };

    let Some(pool) = ctx.db_pool else {
        lookup_failed("no_pool", 0, "no database pool", "no_database");
        refuse(unavailable).await;
        return;
    };
    let catalog = match shared_crafting_catalog(pool).await {
        Ok(catalog) => catalog,
        Err(e) => {
            lookup_failed("catalog", 0, &e.to_string(), sql_error_class(&e));
            refuse(unavailable).await;
            return;
        }
    };
    let inputs = match load_inputs(pool, &catalog, player_id, &request).await {
        Ok(inputs) => inputs,
        Err(e) => {
            lookup_failed("alloy_inputs", 0, &e.to_string(), sql_error_class(&e));
            refuse(unavailable).await;
            return;
        }
    };
    let plan = match check_alloy(
        &inputs.state,
        &catalog,
        &request,
        &inputs.held,
        inputs.component_available,
    ) {
        Ok(plan) => plan,
        Err(AlloyCheck::Refused(why)) => {
            refuse(why).await;
            return;
        }
        Err(AlloyCheck::Catalog { phase, id }) => {
            lookup_failed(phase, id, "missing from the crafting catalog", "catalog");
            refuse(unavailable).await;
            return;
        }
    };

    let job = AlloyJob {
        product_name: inputs
            .product_name
            .unwrap_or_else(|| format!("item {}", plan.product_id)),
        plan,
    };
    let env = InductionEnv::from_ctx(ctx);
    match sessions
        .submit(entity_id, player_id, Box::new(job), &env)
        .await
    {
        SubmitOutcome::Started | SubmitOutcome::Queued { .. } => {
            record_request(VERB, Outcome::Accepted);
        }
        // Answered and counted by the engine's refusal, or no session to
        // answer (logged as `queue_dropped`).
        SubmitOutcome::QueueFull | SubmitOutcome::NotConnected => {}
    }
}

/// What the request-time check reads from the database.
struct AlloyInputs {
    state: CraftingState,
    held: Vec<HeldItem>,
    component_available: i64,
    product_name: Option<String>,
}

async fn load_inputs(
    pool: &Arc<PgPool>,
    catalog: &CraftingCatalog,
    player_id: i32,
    request: &AlloyRequest<'_>,
) -> Result<AlloyInputs, sqlx::Error> {
    let state = load_crafting_state(pool, player_id).await?;
    let mut item_ids = Vec::with_capacity(1 + request.lower_tier_items.len());
    item_ids.push(request.current_tier_item_id);
    item_ids.extend_from_slice(request.lower_tier_items);
    let held: Vec<(i32, i32, i32, i32)> = sqlx::query_as(
        "SELECT item_id, type_id, stack_size, container_id FROM sgw_inventory \
         WHERE character_id = $1 AND item_id = ANY($2)",
    )
    .bind(player_id)
    .bind(&item_ids)
    .fetch_all(pool.as_ref())
    .await?;
    let held = held
        .into_iter()
        .map(|(item_id, type_id, stack_size, container_id)| HeldItem {
            item_id,
            type_id,
            stack_size,
            container_id,
        })
        .collect();

    let blueprint = catalog.blueprint(request.blueprint_id);
    let component_id = blueprint
        .and_then(|b| b.component_sets.first())
        .and_then(|s| s.components.first())
        .map(|c| c.item_id);
    let component_available: i64 = match component_id {
        Some(design_id) => {
            sqlx::query_scalar(
                "SELECT COALESCE(SUM(stack_size), 0)::bigint FROM sgw_inventory \
             WHERE character_id = $1 AND type_id = $2 AND container_id = ANY($3)",
            )
            .bind(player_id)
            .bind(design_id)
            .bind(CRAFTING_INPUT_BAGS.as_slice())
            .fetch_one(pool.as_ref())
            .await?
        }
        None => 0,
    };
    let product_name: Option<String> = match blueprint.and_then(|b| b.product_id) {
        Some(product_id) => {
            sqlx::query_scalar("SELECT name FROM resources.items WHERE item_id = $1")
                .bind(product_id)
                .fetch_optional(pool.as_ref())
                .await?
        }
        None => None,
    };
    Ok(AlloyInputs {
        state,
        held,
        component_available,
        product_name: product_name.map(|n| n.trim().to_string()),
    })
}
