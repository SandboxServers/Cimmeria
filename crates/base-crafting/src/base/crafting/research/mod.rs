//! `research` (cell method 97): study an item, optionally with kickers,
//! for a chance at expertise and the blueprint that makes it.
//!
//! At the request the item and every kicker must still be the player's,
//! in the main or crafting bag, and pass [`rule::check_request`], and the
//! player must have a discipline to research the item in
//! ([`rule::check_eligible`]). The job then waits its induction. At
//! completion it reads the player's crafting state, rolls
//! ([`rule::roll`]) and runs one transaction that checks the eligibility
//! again under the player row lock, consumes exactly the named item and
//! kickers, adds the expertise on a success, and teaches every blueprint
//! that makes the item and whose discipline the player knows. Once the
//! roll is made the item and kickers are used whatever it gives; a
//! research with no eligible discipline, at the request or at completion,
//! uses nothing.
//!
//! Events: the engine's `queued` → `induction_started` → `completed`
//! chain, with `eligible_disciplines`, `discipline_id`, `chance`, `roll`,
//! `result`, the expertise before and after, and `blueprints_learned` on
//! `completed`; a `blueprint_learned` event of its own when a blueprint was
//! taught; `rejected` for every refusal.

pub mod rule;

#[cfg(test)]
mod tests;

use cimmeria_entity::known_names;
use std::sync::Arc;

use cimmeria_cell_catalog::crafting::CraftingCatalog;

use super::feedback::{reject, reject_at_completion, CraftReject};
use super::induction_verb::{
    load_request_inputs, send_result_line, state_at_completion, submit_job,
};
use super::item_lookup::HeldInstance;
use super::persistence::load_crafting_state;
use super::request::CraftCtx;
use super::session::{Completion, InductionJob, JobFuture, JobOutcome};
use super::telemetry::{account_id_of, sql_error_class};
use super::transaction::{
    apply_craft_transaction, resync_inventory, CraftApplied, CraftTransaction, NamedItem,
};
use rule::{
    blueprints_taught, check_eligible, check_request, researched_item, roll, ResearchRoll,
    RESEARCH_EXPERTISE_GAIN,
};

/// The cell method name: the `verb` of every event and metric.
pub const VERB: &str = "research";
const ACTION: &str = "Research";

/// Handle a `Research` request that passed the station gate.
pub async fn handle_research(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    kickers: Vec<i32>,
    ctx: &CraftCtx<'_>,
) {
    if let Some(job) = research_job(entity_id, player_id, item_id, &kickers, ctx).await {
        submit_job(entity_id, player_id, Box::new(job), ctx).await;
    }
}

/// Validate a `Research` request and build its job; `None` when it was
/// refused (the player has been told).
pub async fn research_job(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    kickers: &[i32],
    ctx: &CraftCtx<'_>,
) -> Option<ResearchJob> {
    let mut named = Vec::with_capacity(kickers.len() + 1);
    named.push(item_id);
    named.extend(kickers);
    let inputs = load_request_inputs(VERB, ACTION, entity_id, player_id, &named, ctx).await?;
    // The item is named first, so a successful read is never empty.
    let (item, kickers) = inputs.held.split_first()?;
    if let Err(why) = check_request(&inputs.catalog, item, kickers) {
        reject(VERB, entity_id, player_id, &why, ctx.client()).await;
        return None;
    }
    // `check_request` found the item in the catalog.
    let attrs = inputs.catalog.item(item.type_id)?;
    let state = match load_crafting_state(&inputs.pool, player_id).await {
        Ok(state) => state,
        Err(e) => {
            let account_id = account_id_of(entity_id, ctx.connected, ctx.entity_to_addr);
            tracing::warn!(
                target: "crafting",
                event = "lookup_failed",
                verb = VERB,
                phase = "crafting_state",
                account_id,
                account_name = known_names::account_name(account_id),
                player_id,
                player_name = known_names::player_name(player_id),
                entity_id,
                entity_name = known_names::player_name(player_id),
                item_id, // nt:id-only instance id, type unread yet
                error_class = sql_error_class(&e),
                error = %e,
                "crafting state unreadable at the research request; refused as unavailable"
            );
            let why = CraftReject::Unavailable { action: ACTION };
            reject(VERB, entity_id, player_id, &why, ctx.client()).await;
            return None;
        }
    };
    if let Err(why) = check_eligible(&researched_item(item, attrs), &state) {
        reject(VERB, entity_id, player_id, &why, ctx.client()).await;
        return None;
    }
    Some(ResearchJob {
        catalog: inputs.catalog.clone(),
        item: *item,
        kickers: kickers.to_vec(),
    })
}

/// One research induction.
pub struct ResearchJob {
    pub catalog: Arc<CraftingCatalog>,
    pub item: HeldInstance,
    pub kickers: Vec<HeldInstance>,
}

impl ResearchJob {
    /// The transaction for `outcome`: the item and every kicker, exactly;
    /// on a success the expertise and the blueprints to teach; and the
    /// eligibility the transaction checks again under its lock.
    pub fn plan(&self, outcome: &ResearchRoll, teach: Vec<(i32, i32)>) -> CraftTransaction {
        let research = self
            .catalog
            .item(self.item.type_id)
            .map(|attrs| researched_item(&self.item, attrs));
        let inputs = std::iter::once(&self.item).chain(&self.kickers);
        let expertise = match (outcome.success, outcome.discipline_id) {
            (true, Some(d)) => vec![(d, RESEARCH_EXPERTISE_GAIN)],
            _ => Vec::new(),
        };
        CraftTransaction {
            named_items: inputs
                .clone()
                .map(|h| NamedItem::new(h.item_id, h.type_id))
                .collect(),
            consume_named: inputs.map(|h| (h.item_id, 1)).collect(),
            expertise,
            learn_blueprints: if outcome.success { teach } else { Vec::new() },
            research,
            ..CraftTransaction::default()
        }
    }
}

impl InductionJob for ResearchJob {
    fn verb(&self) -> &'static str {
        VERB
    }

    fn timer_id(&self) -> i32 {
        // The legacy server started research with blueprint 0.
        0
    }

    fn complete<'a>(self: Box<Self>, done: Completion<'a>) -> JobFuture<'a> {
        Box::pin(async move {
            let Some(state) = state_at_completion(done.env, &done.ids).await else {
                return JobOutcome::Failed;
            };
            let Some(attrs) = self.catalog.item(self.item.type_id) else {
                // `check_request` found the item in the catalog, which does
                // not change while the process runs.
                return JobOutcome::Failed;
            };
            // A discipline dropped, or an expertise that reached the tech
            // competency, since the request: refuse before rolling, as the
            // request would have, and restore the slots the research page
            // emptied.
            if let Err(why) = check_eligible(&researched_item(&self.item, attrs), &state) {
                reject_at_completion(&done.ids, &why, done.env.client()).await;
                if let Some(pool) = done.env.db_pool.as_ref() {
                    resync_inventory(done.env, pool, &done.ids).await;
                }
                return JobOutcome::Failed;
            }
            let outcome = roll(attrs, &state, self.kickers.len(), done.rng);
            let teach = blueprints_taught(&self.catalog, &state, self.item.type_id);
            let plan = self.plan(&outcome, teach);
            let Ok(applied) = apply_craft_transaction(done.env, &done.ids, &plan).await else {
                return JobOutcome::Failed;
            };
            if let Some(learned) = &applied.blueprints {
                tracing::info!(
                    target: "crafting",
                    event = "blueprint_learned",
                    verb = VERB,
                    job_id = done.ids.job_id, // nt:id-only induction job counter, unnamed
                    account_id = done.ids.account_id,
                    account_name = known_names::account_name(done.ids.account_id),
                    player_id = done.ids.player_id,
                    player_name = known_names::player_name(done.ids.player_id),
                    entity_id = done.ids.entity_id,
                    entity_name = known_names::player_name(done.ids.player_id),
                    item_id = self.item.item_id,
                    item_name = cimmeria_names::book().item(self.item.type_id),
                    item_type_id = self.item.type_id,
                    blueprints = %learned.field(),
                    known_before = learned.known_before,
                    known_after = learned.blueprint_ids.len(),
                    "blueprint learned from research"
                );
            }
            let line = result_line(&self.catalog, &outcome, &applied);
            send_result_line(done.env, &done.ids, &line).await;
            let mut report = applied.report();
            report.item_id = Some(self.item.item_id);
            report.result = Some(if outcome.success {
                "success"
            } else {
                "failure"
            });
            report.chance = outcome.chance;
            report.roll = outcome.roll;
            report.discipline_id = outcome.discipline_id;
            report.eligible_disciplines = outcome
                .eligible
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(",");
            JobOutcome::Completed(Box::new(report))
        })
    }
}

/// The line the player reads when a research completes.
pub fn result_line(
    catalog: &CraftingCatalog,
    outcome: &ResearchRoll,
    applied: &CraftApplied,
) -> String {
    let Some(discipline_id) = outcome.discipline_id else {
        // Not reached from a completion: a research with no eligible
        // discipline is refused before the roll.
        return "Research complete, but no expertise was gained.".to_string();
    };
    if !outcome.success {
        return "Research complete, but no expertise was gained.".to_string();
    }
    let name = catalog
        .discipline(discipline_id)
        .map_or("Your discipline", |d| d.name.as_str());
    let mut line = match applied.expertise.first() {
        Some(e) => format!(
            "Research succeeded: {name} expertise increased to {}.",
            e.after
        ),
        None => "Research succeeded.".to_string(),
    };
    if let Some(learned) = &applied.blueprints {
        let n = learned.taught.len();
        line.push_str(&format!(
            " You learned {n} new blueprint{}.",
            if n == 1 { "" } else { "s" }
        ));
    }
    line
}
