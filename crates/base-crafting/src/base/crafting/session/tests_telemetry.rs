//! The induction engine's telemetry: every job event carries the job id,
//! the verb and the player's identity; drops name their reason and count;
//! `crafting_jobs_total` counts each job exactly once.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tracing::Level;

use super::tests::{job, Harness, ACCOUNT_ID, ENTITY, PLAYER_ID};
use super::*;
use crate::base::crafting::telemetry::{
    send_to_player, JobIds, METRIC_JOBS, METRIC_REJECTIONS, METRIC_REQUESTS,
};
use crate::test_support::{Captured, LogCapture, TestTransport};
use cimmeria_observability::testing::{counter_total, install as install_meter};

/// The captured `crafting` events named `event`, oldest first.
fn events(capture: &crate::test_support::LogCaptureGuard, event: &str) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == "crafting" && c.has_field("event", event))
        .collect()
}

fn assert_identity(e: &Captured) {
    assert!(e.has_field("account_id", &ACCOUNT_ID.to_string()), "{e:#?}");
    assert!(e.has_field("player_id", &PLAYER_ID.to_string()), "{e:#?}");
    assert!(e.has_field("entity_id", &ENTITY.to_string()), "{e:#?}");
}

/// Two jobs, the second queued: `queued`, `induction_started`,
/// `induction_expired` and `completed` each carry the job id, the verb
/// and all three identity fields, and one job's events share its id.
#[tokio::test]
async fn job_events_carry_the_job_id_verb_and_identity() {
    let capture = LogCapture::install();
    let h = Harness::new();
    h.submit("a", 1).await;
    h.submit("b", 2).await;
    h.fire_next().await;
    h.fire_next().await;

    let queued = events(&capture, "queued");
    let started = events(&capture, "induction_started");
    let expired = events(&capture, "induction_expired");
    let completed = events(&capture, "completed");
    assert_eq!(queued.len(), 1);
    assert_eq!(started.len(), 2);
    assert_eq!(expired.len(), 2);
    assert_eq!(completed.len(), 2);
    for e in queued
        .iter()
        .chain(&started)
        .chain(&expired)
        .chain(&completed)
    {
        assert_identity(e);
        assert!(e.has_field("verb", "fake"), "{e:#?}");
        assert!(e.fields.contains_key("job_id"), "{e:#?}");
    }
    let second = queued[0].fields["job_id"].clone();
    assert!(started[1].has_field("job_id", &second));
    assert!(expired[1].has_field("job_id", &second));
    assert!(completed[1].has_field("job_id", &second));
    assert_ne!(started[0].fields["job_id"], second);
    assert!(started[0].fields.contains_key("expires_at"));
    assert!(queued[0].has_field("queue_len", "2"));
}

#[tokio::test]
async fn queue_dropped_names_the_reason_and_the_jobs() {
    let capture = LogCapture::install();
    let h = Harness::new();
    h.submit("a", 1).await;
    h.submit("b", 2).await;
    h.sessions
        .drop_player(ENTITY, DropReason::Logout, "client_disconnect");
    h.submit("c", 3).await;
    h.sessions
        .drop_player(ENTITY, DropReason::WorldChange, "gate_travel");

    let dropped = events(&capture, "queue_dropped");
    assert_eq!(dropped.len(), 2);
    assert_identity(&dropped[0]);
    assert!(dropped[0].has_field("reason", "logout"));
    assert!(dropped[0].has_field("cause", "client_disconnect"));
    assert!(dropped[0].has_field("jobs_dropped", "2"));
    assert!(dropped[1].has_field("reason", "world_change"));
    assert!(dropped[1].has_field("jobs_dropped", "1"));
    assert_eq!(dropped[0].level, Level::INFO);
}

/// A job with a verb no other test uses, so the process-wide meter's
/// counts for it change only here.
struct ProbeJob {
    fails: bool,
}

const PROBE: &str = "jobs_total_probe";

impl InductionJob for ProbeJob {
    fn verb(&self) -> &'static str {
        PROBE
    }

    fn timer_id(&self) -> i32 {
        0
    }

    fn complete<'a>(self: Box<Self>, _done: Completion<'a>) -> JobFuture<'a> {
        let outcome = if self.fails {
            JobOutcome::Failed
        } else {
            JobOutcome::Completed(Box::default())
        };
        Box::pin(async move { outcome })
    }
}

/// `crafting_jobs_total{verb = PROBE}` per outcome: completed, failed,
/// dropped.
fn probe_counts() -> [u64; 3] {
    ["completed", "failed", "dropped"]
        .map(|outcome| counter_total(METRIC_JOBS, &[("verb", PROBE), ("outcome", outcome)]))
}

/// The change in [`probe_counts`] since `before`.
fn probe_delta(before: &mut [u64; 3]) -> [u64; 3] {
    let now = probe_counts();
    let delta = [0, 1, 2].map(|i| now[i] - before[i]);
    *before = now;
    delta
}

async fn submit_probe(h: &Harness, fails: bool) -> SubmitOutcome {
    h.sessions
        .submit(ENTITY, PLAYER_ID, Box::new(ProbeJob { fails }), &h.env)
        .await
}

/// `crafting_jobs_total` is counted once per job when it ends: completed,
/// failed, dropped by a logout (running and queued), dropped by the
/// completion guard (the taken job too), and dropped unqueued.
#[tokio::test]
async fn each_job_is_counted_once_when_it_ends() {
    install_meter();
    let mut before = probe_counts();
    let h = Harness::new();
    submit_probe(&h, false).await;
    submit_probe(&h, true).await;
    submit_probe(&h, false).await;
    h.fire_next().await;
    h.fire_next().await;
    assert_eq!(probe_delta(&mut before), [1, 1, 0]);

    submit_probe(&h, false).await;
    h.sessions.drop_player(ENTITY, DropReason::Logout, "test");
    assert_eq!(
        probe_delta(&mut before),
        [0, 0, 2],
        "the running third job and the queued fourth"
    );
    h.scheduler.take();

    submit_probe(&h, false).await;
    submit_probe(&h, false).await;
    h.env.entity_to_addr.lock().unwrap().remove(&ENTITY);
    h.fire_next().await;
    assert_eq!(
        probe_delta(&mut before),
        [0, 0, 2],
        "the guard drops the taken job and the queued one"
    );

    submit_probe(&h, false).await;
    assert_eq!(probe_delta(&mut before), [0, 0, 1], "dropped unqueued");
}

/// A client update that cannot be sent is logged, never swallowed.
#[tokio::test]
async fn a_failed_client_send_is_a_warning() {
    let capture = LogCapture::install();
    let env = InductionEnv {
        db_pool: None,
        cell_tx: None,
        transport: Arc::new(TestTransport::new()),
        connected: Arc::new(Mutex::new(HashMap::new())),
        entity_to_addr: Arc::new(Mutex::new(HashMap::new())),
    };
    let ids = JobIds {
        job_id: 9,
        verb: "fake",
        account_id: ACCOUNT_ID,
        player_id: PLAYER_ID,
        entity_id: ENTITY,
        gm_entity_id: None,
        gm_name: None,
    };
    assert!(!send_to_player(&env, &ids, 12, &[], "induction_timer").await);
    let e = capture
        .find_event(
            Level::WARN,
            "crafting client update not sent",
            "entity_to_addr_miss",
        )
        .expect("client_sync_failed WARN");
    assert!(e.has_field("event", "client_sync_failed"));
    assert!(e.has_field("what", "induction_timer"));
    assert!(e.has_field("job_id", "9"));
    assert_identity(&e);
}

/// The eleventh request is `rejected` with `reason = queue_full`, naming
/// the player; the held jobs are not dropped.
#[tokio::test]
async fn the_eleventh_request_is_rejected_as_queue_full() {
    let capture = LogCapture::install();
    install_meter();
    let rejections = [("verb", "fake"), ("reason", "queue_full")];
    let answered = [("verb", "fake"), ("outcome", "rejected")];
    let before = (
        counter_total(METRIC_REJECTIONS, &rejections),
        counter_total(METRIC_REQUESTS, &answered),
    );
    let h = Harness::new();
    for k in 0..MAX_INDUCTIONS {
        h.submit("j", k as i32).await;
    }
    assert_eq!(h.submit("j10", 10).await, SubmitOutcome::QueueFull);
    let e = capture
        .find_event(Level::INFO, "crafting request rejected", "queue_full")
        .expect("rejected queue_full");
    assert_eq!(e.target, "crafting");
    assert!(e.has_field("event", "rejected"));
    assert!(e.has_field("player_id", &PLAYER_ID.to_string()), "{e:#?}");
    assert!(e.has_field("entity_id", &ENTITY.to_string()), "{e:#?}");
    assert!(e.has_field("account_id", &ACCOUNT_ID.to_string()), "{e:#?}");
    assert!(
        e.has_field("queue_limit", &MAX_INDUCTIONS.to_string()),
        "{e:#?}"
    );
    assert!(events(&capture, "queue_dropped").is_empty());
    // A full queue answers the request: one rejection, one rejected request.
    assert_eq!(counter_total(METRIC_REJECTIONS, &rejections) - before.0, 1);
    assert_eq!(counter_total(METRIC_REQUESTS, &answered) - before.1, 1);
}

/// `job()` from the engine tests is the verb `fake`; the not-connected
/// drop still names the player.
#[tokio::test]
async fn a_job_for_an_unconnected_entity_is_logged_as_dropped() {
    let capture = LogCapture::install();
    let h = Harness::new();
    h.env.entity_to_addr.lock().unwrap().clear();
    h.sessions
        .submit(ENTITY, PLAYER_ID, job("a", 1, &h.log), &h.env)
        .await;
    let dropped = events(&capture, "queue_dropped");
    assert_eq!(dropped.len(), 1);
    assert!(dropped[0].has_field("reason", "not_connected"));
    assert!(dropped[0].has_field("player_id", &PLAYER_ID.to_string()));
    assert!(dropped[0].has_field("entity_id", &ENTITY.to_string()));
}

/// The recycled entity's next owner: another character on another
/// account, same entity id and address.
fn recycle_entity(h: &Harness) {
    let mut connected = h.env.connected.lock().unwrap();
    let state = connected.get_mut(&h.addr).unwrap();
    state.active_player_id = Some(PLAYER_ID + 1);
    state.account_id = ACCOUNT_ID + 1;
}

/// A job whose start runs after its queue was dropped and the entity
/// recycled (the window between releasing the session lock and sending
/// the bar) sends nothing to the new player and schedules nothing.
#[tokio::test]
async fn a_late_start_for_a_dropped_job_sends_no_bar() {
    let h = Harness::new();
    assert_eq!(h.submit("a", 7).await, SubmitOutcome::Started);
    let scheduled = h.scheduler.take();
    assert_eq!(scheduled.len(), 1);
    let bars = h.timer_ids().len();

    h.sessions.drop_player(ENTITY, DropReason::Logout, "test");
    recycle_entity(&h);
    let ids = JobIds {
        job_id: scheduled[0].job_id,
        verb: "fake",
        account_id: ACCOUNT_ID,
        player_id: PLAYER_ID,
        entity_id: ENTITY,
        gm_entity_id: None,
        gm_name: None,
    };
    let started = Started {
        job_id: scheduled[0].job_id,
        deadline: scheduled[0].deadline,
        timer_id: 7,
        verb: "fake",
    };
    h.sessions.start(&ids, started, 1, &h.env).await;

    assert_eq!(h.timer_ids().len(), bars, "no bar for the new player");
    assert!(h.scheduler.take().is_empty(), "no wake-up scheduled");
}

/// Jobs left on a recycled entity id are dropped under the character and
/// account that queued them, not the new owner's.
#[tokio::test]
async fn stale_jobs_are_dropped_under_the_session_that_queued_them() {
    let capture = LogCapture::install();
    let h = Harness::new();
    h.submit("a", 1).await;
    h.submit("b", 2).await;
    recycle_entity(&h);
    h.sessions
        .submit(ENTITY, PLAYER_ID + 1, job("c", 3, &h.log), &h.env)
        .await;

    let dropped = events(&capture, "queue_dropped");
    assert_eq!(dropped.len(), 1, "{dropped:#?}");
    let e = &dropped[0];
    assert!(e.has_field("reason", "stale_session"), "{e:#?}");
    assert!(e.has_field("jobs_dropped", "2"), "{e:#?}");
    assert_identity(e);
}
