//! A confirmed respec drops the player's induction queue, so a craft,
//! research or alloy queued before it can never complete for a discipline
//! the respec cleared.

use super::*;
use crate::base::crafting::request::CraftCtx;
use crate::base::crafting::respec::{confirm_with, with_session, PendingRespec};
use crate::base::crafting::rng::ScriptedRng;
use crate::base::crafting::session::{
    Completion, CraftingSessions, InductionJob, JobFuture, JobOutcome, ManualScheduler,
    SubmitOutcome,
};
use crate::test_support::{require_db_or_skip, LogCapture};

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

/// Queue an induction that would consume a component, grant a product and
/// raise discipline 78, confirm a respec, then fire the deadline: nothing
/// is consumed or granted, the expertise stays cleared, and the queue drop
/// is logged with `reason = respec`.
#[tokio::test]
async fn respec_drops_the_queue_so_nothing_completes() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let f = Fixture::new(&pool, 21).await;
    let component = f.stack(COMPONENT, INV_CRAFTING, 0, 2).await;
    sqlx::query("UPDATE sgw_player SET discipline_ids = '{78}' WHERE player_id = $1")
        .bind(f.player_id)
        .execute(&pool)
        .await
        .expect("know 78");
    sqlx::query(
        "INSERT INTO sgw_player_discipline_expertise (player_id, discipline_id, expertise) \
         VALUES ($1, 78, 40)",
    )
    .bind(f.player_id)
    .execute(&pool)
    .await
    .expect("expertise 40");
    let plan = CraftTransaction {
        named_items: vec![NamedItem::new(component, COMPONENT)],
        consume_named: vec![],
        consume: vec![(COMPONENT, 1)],
        grant: vec![(BANK_FIRST_PRODUCT, 1)],
        expertise: vec![(78, 1)],
        learn_blueprints: vec![],
        required_knowledge: None,
    };
    let scheduler = Arc::new(ManualScheduler::default());
    let sessions = Arc::new(CraftingSessions::new(
        Box::new(scheduler.clone()),
        Box::new(|| Box::new(ScriptedRng::new(vec![0.5]))),
    ));
    let started = sessions
        .submit(f.entity_id, f.player_id, Box::new(PlanJob(plan)), &f.env)
        .await;
    assert_eq!(started, SubmitOutcome::Started);

    with_session(f.entity_id, &f.env.connected, &f.env.entity_to_addr, |s| {
        s.pending_respec = Some(PendingRespec::open(f.player_id, std::time::Instant::now()));
    })
    .expect("session");
    let ctx = CraftCtx {
        db_pool: &f.env.db_pool,
        cell_tx: &f.env.cell_tx,
        transport: &f.env.transport,
        connected: &f.env.connected,
        entity_to_addr: &f.env.entity_to_addr,
    };
    confirm_with(&sessions, f.entity_id, f.player_id, &ctx).await;
    for due in scheduler.take() {
        sessions
            .expire_at(due.entity_id, due.job_id, due.deadline, &f.env)
            .await;
    }

    let row = f.row(component).await;
    let products = f.stacks_of(BANK_FIRST_PRODUCT).await;
    let expertise: Option<i32> = sqlx::query_scalar(
        "SELECT expertise FROM sgw_player_discipline_expertise \
         WHERE player_id = $1 AND discipline_id = 78",
    )
    .bind(f.player_id)
    .fetch_optional(&pool)
    .await
    .expect("read expertise");
    let dropped = capture
        .all()
        .into_iter()
        .find(|e| e.target == "crafting" && e.has_field("event", "queue_dropped"));
    let respecced = capture
        .all()
        .into_iter()
        .any(|e| e.target == "crafting" && e.has_field("event", "respec"));
    f.cleanup().await;

    assert!(respecced, "the respec went through");
    assert_eq!(row, Some((2, INV_CRAFTING)), "nothing consumed");
    assert!(products.is_empty(), "nothing granted");
    assert_eq!(expertise, None, "no expertise came back");
    let dropped = dropped.expect("queue_dropped event");
    assert!(dropped.has_field("reason", "respec"), "{dropped:#?}");
    assert!(dropped.has_field("jobs_dropped", "1"), "{dropped:#?}");
    assert!(
        dropped.has_field("player_id", &f.player_id.to_string()),
        "{dropped:#?}"
    );
}
