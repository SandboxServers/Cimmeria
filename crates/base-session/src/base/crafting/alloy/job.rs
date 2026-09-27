//! The alloy induction: at the end of the bar, one transaction consumes the
//! component and the elementary components the request validated, grants
//! the product and adds expertise. The player must still know the blueprint
//! and its discipline then: a respec during the bar refuses the job.

use super::rules::{bucket_label, AlloyPlan};
use super::VERB;
use crate::base::crafting::feedback::{feedback_text_args, reject_at_completion, CraftReject};
use crate::base::crafting::persistence::load_crafting_state;
use crate::base::crafting::session::{
    Completion, InductionEnv, InductionJob, JobFuture, JobOutcome,
};
use crate::base::crafting::telemetry::{send_to_player, sql_error_class, JobIds};
use crate::base::crafting::transaction::{apply_craft_transaction, resync_inventory};
use crate::mercury::method_idx;

/// A queued alloy.
#[derive(Debug)]
pub struct AlloyJob {
    pub plan: AlloyPlan,
    /// The product's item name, for the success line.
    pub product_name: String,
}

impl AlloyJob {
    /// The line the player reads when the alloy is made.
    pub fn success_text(&self) -> String {
        format!(
            "Alloying complete: {} x {}.",
            self.plan.product_quantity, self.product_name
        )
    }
}

impl AlloyJob {
    /// Why the job may no longer run: the blueprint or its discipline was
    /// forgotten during the bar, or the state could not be read (logged as
    /// `lookup_failed`). `None` with no database: the transaction reports
    /// that itself.
    async fn knowledge_lost(&self, env: &InductionEnv, ids: &JobIds) -> Option<CraftReject> {
        let pool = env.db_pool.as_ref()?;
        let state = match load_crafting_state(pool, ids.player_id).await {
            Ok(state) => state,
            Err(e) => {
                tracing::warn!(
                    target: "crafting",
                    event = "lookup_failed",
                    verb = ids.verb,
                    phase = "alloy_completion_state",
                    job_id = ids.job_id,
                    account_id = ids.account_id,
                    player_id = ids.player_id,
                    entity_id = ids.entity_id,
                    blueprint_id = self.plan.blueprint_id,
                    error_class = sql_error_class(&e),
                    error = %e,
                    "alloy: crafting state unreadable at completion; job refused"
                );
                return Some(CraftReject::InductionFailed);
            }
        };
        let blueprint_id = self.plan.blueprint_id;
        if !state.blueprint_ids.contains(&blueprint_id) {
            return Some(CraftReject::UnknownBlueprint { blueprint_id });
        }
        if !state.knows_discipline(self.plan.discipline_id) {
            return Some(CraftReject::DisciplineUnknown {
                blueprint_id,
                discipline_id: self.plan.discipline_id,
            });
        }
        None
    }
}

impl InductionJob for AlloyJob {
    fn verb(&self) -> &'static str {
        VERB
    }

    /// The bar carries the blueprint id, as the legacy `onCraftingStarted`
    /// did for alloying.
    fn timer_id(&self) -> i32 {
        self.plan.blueprint_id
    }

    fn complete<'a>(self: Box<Self>, done: Completion<'a>) -> JobFuture<'a> {
        Box::pin(async move {
            if let Some(why) = self.knowledge_lost(done.env, &done.ids).await {
                reject_at_completion(&done.ids, &why, done.env.client()).await;
                if let Some(pool) = &done.env.db_pool {
                    resync_inventory(done.env, pool, &done.ids).await;
                }
                return JobOutcome::Failed;
            }
            // A refusal or rollback has already told the player and logged
            // why; the engine counts the job as failed.
            let Ok(applied) =
                apply_craft_transaction(done.env, &done.ids, &self.plan.transaction()).await
            else {
                return JobOutcome::Failed;
            };
            send_to_player(
                done.env,
                &done.ids,
                method_idx::ON_PLAYER_COMMUNICATION,
                &feedback_text_args(&self.success_text()),
                "success_line",
            )
            .await;
            let mut report = applied.report();
            report.blueprint_id = Some(self.plan.blueprint_id);
            report.item_id = Some(self.plan.current_tier_item_id);
            report.quality_bucket = self.plan.bucket.map(bucket_label);
            report.elementary = self.plan.elementary_field();
            JobOutcome::Completed(Box::new(report))
        })
    }
}
