//! The alloy induction: at the end of the bar, one transaction consumes the
//! component and the elementary components the request validated, grants
//! the product and adds expertise. The player must still know the blueprint
//! and its discipline then: the transaction checks it under a share lock on
//! the player row, so a respec during the bar refuses the job.

use super::rules::{bucket_label, AlloyPlan};
use super::VERB;
use crate::base::crafting::feedback::feedback_text_args;
use crate::base::crafting::session::{Completion, InductionJob, JobFuture, JobOutcome};
use crate::base::crafting::telemetry::send_to_player;
use crate::base::crafting::transaction::apply_craft_transaction;
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
            // A refusal or rollback (including a blueprint or discipline
            // forgotten during the bar, checked inside the transaction) has
            // already told the player, resynced the bags and logged why; the
            // engine counts the job as failed.
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
