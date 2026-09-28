//! An induction whose job runs the transaction: consumption happens at
//! completion, never at the request, so a logout during the bar loses
//! nothing.

use super::*;
use crate::base::crafting::rng::ScriptedRng;
use crate::base::crafting::session::{
    Completion, CraftingSessions, DropReason, InductionJob, JobFuture, JobOutcome, ManualScheduler,
    SubmitOutcome,
};
use crate::test_support::require_db_or_skip;

/// A test-only verb: its completion applies a fixed plan.
struct PlanJob(CraftTransaction);

impl InductionJob for PlanJob {
    fn verb(&self) -> &'static str {
        "test_plan"
    }

    fn timer_id(&self) -> i32 {
        0
    }

    fn complete<'a>(self: Box<Self>, done: Completion<'a>) -> JobFuture<'a> {
        Box::pin(async move {
            match apply_craft_transaction(done.env, &done.ids, &self.0).await {
                Ok(applied) => JobOutcome::Completed(Box::new(applied.report())),
                Err(_) => JobOutcome::Failed,
            }
        })
    }
}

fn engine() -> (Arc<CraftingSessions>, Arc<ManualScheduler>) {
    let scheduler = Arc::new(ManualScheduler::default());
    let sessions = Arc::new(CraftingSessions::new(
        Box::new(scheduler.clone()),
        Box::new(|| Box::new(ScriptedRng::new(vec![0.5]))),
    ));
    (sessions, scheduler)
}

/// Logout between the request and the end of the bar: at the deadline the
/// component is still whole and no product exists. The same flow without
/// the logout does consume, so the first half is not vacuous.
#[tokio::test]
async fn logout_mid_induction_consumes_nothing() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 10).await;
    let component = f.stack(COMPONENT, INV_CRAFTING, 0, 2).await;
    let plan = CraftTransaction {
        named_items: vec![NamedItem::new(component, COMPONENT)],
        consume_named: vec![],
        consume: vec![(COMPONENT, 1)],
        grant: vec![(BANK_FIRST_PRODUCT, 1)],
        expertise: vec![],
        learn_blueprints: vec![],
        required_knowledge: None,
        research: None,
    };
    let (sessions, scheduler) = engine();

    let started = sessions
        .submit(
            f.entity_id,
            f.player_id,
            Box::new(PlanJob(plan.clone())),
            &f.env,
        )
        .await;
    assert_eq!(started, SubmitOutcome::Started);
    assert_eq!(
        f.row(component).await,
        Some((2, INV_CRAFTING)),
        "nothing is consumed at the request"
    );
    assert_eq!(
        sessions.drop_player(f.entity_id, DropReason::Logout, "log_off"),
        1
    );
    for due in scheduler.take() {
        sessions
            .expire_at(due.entity_id, due.job_id, due.deadline, &f.env)
            .await;
    }
    assert_eq!(f.row(component).await, Some((2, INV_CRAFTING)));
    assert!(f.stacks_of(BANK_FIRST_PRODUCT).await.is_empty());

    // Control: without the logout, the same induction consumes, and
    // `completed` lists the stacks with their before and after sizes.
    let capture = crate::test_support::LogCapture::install();
    sessions
        .submit(f.entity_id, f.player_id, Box::new(PlanJob(plan)), &f.env)
        .await;
    let due = scheduler.take();
    assert_eq!(due.len(), 1);
    sessions
        .expire_at(due[0].entity_id, due[0].job_id, due[0].deadline, &f.env)
        .await;
    assert_eq!(f.row(component).await, Some((1, INV_CRAFTING)));
    let products = f.stacks_of(BANK_FIRST_PRODUCT).await;
    assert_eq!(products.len(), 1);
    let (_, _, bag, slot) = products[0];
    let completed = capture
        .all()
        .into_iter()
        .find(|e| e.target == "crafting" && e.has_field("event", "completed"))
        .expect("completed event");
    assert!(completed.has_field("verb", "test_plan"));
    assert!(completed.has_field("account_id", &f.account_id.to_string()));
    assert!(completed.has_field("player_id", &f.player_id.to_string()));
    assert!(completed.has_field("entity_id", &f.entity_id.to_string()));
    assert!(
        completed.has_field("consumed", &format!("{component}:{COMPONENT}:2→1")),
        "{completed:#?}"
    );
    assert!(
        completed.has_field("granted", &format!("{BANK_FIRST_PRODUCT}:{bag}:{slot}:0→1")),
        "{completed:#?}"
    );
    f.cleanup().await;
}

/// A burst past the limit gets a line per refused request, but only the
/// first refusal resyncs the inventory.
#[tokio::test]
async fn refusals_past_the_limit_resync_the_inventory_once() {
    use crate::base::crafting::session::MAX_INDUCTIONS;
    use crate::base::crafting::test_packets::feedback_text;
    use crate::mercury::method_idx;

    let pool = require_db_or_skip!();
    let f = Fixture::new(&pool, 15).await;
    f.stack(COMPONENT, INV_CRAFTING, 0, 2).await;
    let (sessions, _scheduler) = engine();
    for _ in 0..MAX_INDUCTIONS {
        sessions
            .submit(
                f.entity_id,
                f.player_id,
                Box::new(PlanJob(CraftTransaction::default())),
                &f.env,
            )
            .await;
    }
    f.transport.clear();

    for _ in 0..2 {
        let outcome = sessions
            .submit(
                f.entity_id,
                f.player_id,
                Box::new(PlanJob(CraftTransaction::default())),
                &f.env,
            )
            .await;
        assert_eq!(outcome, SubmitOutcome::QueueFull);
    }

    let calls = f.calls();
    let methods: Vec<u16> = calls.iter().map(|c| c.method).collect();
    assert_eq!(
        methods,
        vec![
            method_idx::ON_PLAYER_COMMUNICATION,
            method_idx::ON_UPDATE_ITEM,
            method_idx::ON_PLAYER_COMMUNICATION,
        ]
    );
    assert_eq!(
        feedback_text(&calls[2]),
        "You can have at most 10 crafting jobs at once."
    );
    f.cleanup().await;
}
