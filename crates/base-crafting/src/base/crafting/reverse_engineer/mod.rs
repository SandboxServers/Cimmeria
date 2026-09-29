//! `reverseEngineer` (cell method 98): take an item apart for some of the
//! components of a recipe that makes it.
//!
//! At the request the item must still be the player's, in the main or
//! crafting bag, flagged reverse-engineerable and made by a blueprint with
//! a recipe ([`rule::check_request`]). No discipline needs to be known. The
//! reverse-engineering page sends one request per slotted item, up to ten
//! at once, and each queues as its own induction.
//!
//! At completion the job picks a blueprint and one of its component sets
//! uniformly and rolls each component ([`rule::recover`]). One transaction
//! then consumes exactly the named instance (never another stack of the
//! same design) and grants the recovered components.
//!
//! Events: the engine's `queued` → `induction_started` → `completed`
//! chain, with `blueprint_id`, `component_set_id`, `bias` and `rolls` on
//! `completed`; `rejected` for every refusal.

pub mod rule;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use cimmeria_cell_catalog::crafting::CraftingCatalog;

use super::feedback::reject;
use super::induction_verb::{
    load_request_inputs, send_result_line, state_at_completion, submit_job,
};
use super::item_lookup::HeldInstance;
use super::request::CraftCtx;
use super::session::{Completion, InductionJob, JobFuture, JobOutcome};
use super::transaction::{apply_craft_transaction, CraftApplied, CraftTransaction, NamedItem};
use rule::{candidate_blueprints, check_request, recover, Recovery};

/// The cell method name: the `verb` of every event and metric.
pub const VERB: &str = "reverseEngineer";
const ACTION: &str = "Reverse engineering";

/// Handle a `ReverseEngineer` request that passed the station gate.
pub async fn handle_reverse_engineer(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    ctx: &CraftCtx<'_>,
) {
    if let Some(job) = reverse_engineer_job(entity_id, player_id, item_id, ctx).await {
        submit_job(entity_id, player_id, Box::new(job), ctx).await;
    }
}

/// Validate a `ReverseEngineer` request and build its job; `None` when it
/// was refused (the player has been told).
pub async fn reverse_engineer_job(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    ctx: &CraftCtx<'_>,
) -> Option<ReverseEngineerJob> {
    let inputs = load_request_inputs(VERB, ACTION, entity_id, player_id, &[item_id], ctx).await?;
    let item = inputs.held.first().copied()?;
    if let Err(why) = check_request(&inputs.catalog, &item) {
        reject(VERB, entity_id, player_id, &why, ctx.client()).await;
        return None;
    }
    Some(ReverseEngineerJob {
        catalog: inputs.catalog.clone(),
        item,
    })
}

/// One reverse-engineering induction.
pub struct ReverseEngineerJob {
    pub catalog: Arc<CraftingCatalog>,
    pub item: HeldInstance,
}

impl ReverseEngineerJob {
    /// The transaction for `recovery`: one unit of exactly the named
    /// instance, and the recovered components.
    pub fn plan(&self, recovery: &Recovery) -> CraftTransaction {
        CraftTransaction {
            named_items: vec![NamedItem::new(self.item.item_id, self.item.type_id)],
            consume_named: vec![(self.item.item_id, 1)],
            grant: recovery.grants(),
            ..CraftTransaction::default()
        }
    }
}

impl InductionJob for ReverseEngineerJob {
    fn verb(&self) -> &'static str {
        VERB
    }

    fn timer_id(&self) -> i32 {
        // The legacy server started reverse engineering with blueprint 0.
        0
    }

    fn complete<'a>(self: Box<Self>, done: Completion<'a>) -> JobFuture<'a> {
        Box::pin(async move {
            let Some(state) = state_at_completion(done.env, &done.ids).await else {
                return JobOutcome::Failed;
            };
            let candidates = candidate_blueprints(&self.catalog, self.item.type_id);
            let tech_comp = self
                .catalog
                .item(self.item.type_id)
                .map_or(0, |a| a.tech_comp);
            // `check_request` found a candidate, and the catalog does not
            // change while the process runs.
            let Some(recovery) = recover(&candidates, &state, tech_comp, done.rng) else {
                return JobOutcome::Failed;
            };
            let plan = self.plan(&recovery);
            let Ok(applied) = apply_craft_transaction(done.env, &done.ids, &plan).await else {
                return JobOutcome::Failed;
            };
            send_result_line(done.env, &done.ids, &result_line(&applied)).await;
            let mut report = applied.report();
            report.item_id = Some(self.item.item_id);
            report.blueprint_id = Some(recovery.blueprint_id);
            report.component_set_id = Some(recovery.component_set_id);
            report.bias = Some(recovery.bias);
            report.rolls = recovery.rolls_field();
            JobOutcome::Completed(Box::new(report))
        })
    }
}

/// The line the player reads when a reverse engineering completes.
pub fn result_line(applied: &CraftApplied) -> String {
    let units: i32 = applied.granted.iter().map(|g| g.quantity()).sum();
    format!(
        "Reverse engineering complete: recovered {units} component{}.",
        if units == 1 { "" } else { "s" }
    )
}
