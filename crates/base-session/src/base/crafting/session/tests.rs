//! Induction engine tests. The engine runs on a [`ManualScheduler`], so
//! every expiry is fired by hand with an explicit `now`; no test sleeps
//! for an induction. One test drives the production [`TokioScheduler`]
//! on tokio's paused clock.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use tokio::time::{Duration, Instant};

use super::timer::induction_timer_args;
use super::*;
use crate::base::crafting::rng::ScriptedRng;
use crate::base::crafting::test_packets::{decode_all, feedback_text, MethodCall};
use crate::mercury::game_clock;
use crate::mercury::method_idx;
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
use cimmeria_wire::cell::client_methods::being::ON_TIMER_UPDATE;

pub(super) const ENTITY: u32 = 4270;
pub(super) const PLAYER_ID: i32 = 4271;
pub(super) const ACCOUNT_ID: u32 = 4272;
const WORLD: &str = "CombatSim";

pub(super) type Log = Arc<Mutex<Vec<&'static str>>>;

/// A job that records its name when it completes.
struct FakeJob {
    name: &'static str,
    timer_id: i32,
    log: Log,
}

impl InductionJob for FakeJob {
    fn verb(&self) -> &'static str {
        "fake"
    }

    fn timer_id(&self) -> i32 {
        self.timer_id
    }

    fn complete<'a>(self: Box<Self>, _done: Completion<'a>) -> JobFuture<'a> {
        Box::pin(async move {
            self.log.lock().unwrap().push(self.name);
            JobOutcome::Completed(Box::default())
        })
    }
}

pub(super) fn job(name: &'static str, timer_id: i32, log: &Log) -> Box<dyn InductionJob> {
    Box::new(FakeJob {
        name,
        timer_id,
        log: log.clone(),
    })
}

pub(super) struct Harness {
    pub(super) sessions: Arc<CraftingSessions>,
    pub(super) scheduler: Arc<ManualScheduler>,
    pub(super) env: InductionEnv,
    pub(super) transport: Arc<TestTransport>,
    pub(super) addr: SocketAddr,
    pub(super) log: Log,
}

impl Harness {
    pub(super) fn new() -> Self {
        let scheduler = Arc::new(ManualScheduler::default());
        Self::with_scheduler(scheduler.clone(), Box::new(scheduler))
    }

    fn with_scheduler(scheduler: Arc<ManualScheduler>, boxed: Box<dyn InductionScheduler>) -> Self {
        let sessions = Arc::new(CraftingSessions::new(
            boxed,
            Box::new(|| Box::new(ScriptedRng::new(vec![0.5]))),
        ));
        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();
        let addr: SocketAddr = "127.0.0.1:55740".parse().unwrap();
        let mut state = test_default_connected_client_state();
        state.player_entity_id = Some(ENTITY);
        state.active_player_id = Some(PLAYER_ID);
        state.account_id = ACCOUNT_ID;
        state.world_name = Some(WORLD.to_string());
        let env = InductionEnv {
            db_pool: None,
            cell_tx: None,
            transport: dyn_transport,
            connected: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
            entity_to_addr: Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)]))),
        };
        Self {
            sessions,
            scheduler,
            env,
            transport,
            addr,
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub(super) async fn submit(&self, name: &'static str, timer_id: i32) -> SubmitOutcome {
        self.sessions
            .submit(ENTITY, PLAYER_ID, job(name, timer_id, &self.log), &self.env)
            .await
    }

    /// Fire the one scheduled wake-up at its deadline.
    pub(super) async fn fire_next(&self) {
        let due = self.scheduler.take();
        assert_eq!(due.len(), 1, "exactly one wake-up scheduled: {due:?}");
        let due = due[0];
        self.sessions
            .expire_at(due.entity_id, due.job_id, due.deadline, &self.env)
            .await;
    }

    pub(super) fn calls(&self) -> Vec<MethodCall> {
        decode_all(&self.transport.filter_to(self.addr))
    }

    pub(super) fn timer_ids(&self) -> Vec<i32> {
        self.calls()
            .iter()
            .filter(|c| c.method == ON_TIMER_UPDATE)
            .map(|c| i32::from_le_bytes(c.args[0..4].try_into().unwrap()))
            .collect()
    }

    pub(super) fn completed(&self) -> Vec<&'static str> {
        self.log.lock().unwrap().clone()
    }
}

// ── The state machine ────────────────────────────────────────────────────

#[test]
fn first_job_starts_and_the_rest_queue_in_order() {
    let log = Log::default();
    let now = Instant::now();
    let mut s = CraftingSession::new(ACCOUNT_ID, PLAYER_ID, Some(WORLD.into()));
    let Enqueued::Started(started) = s.enqueue(job("a", 1, &log), 10, now) else {
        panic!("first job must start");
    };
    assert_eq!(started.job_id, 10);
    assert_eq!(started.deadline, now + INDUCTION_DURATION);
    assert_eq!(started.timer_id, 1);
    assert_eq!(
        s.enqueue(job("b", 2, &log), 11, now),
        Enqueued::Queued { position: 1 }
    );
    assert_eq!(
        s.enqueue(job("c", 3, &log), 12, now),
        Enqueued::Queued { position: 2 }
    );
    assert_eq!(s.len(), 3);
}

/// Ten are held, the running one included; the eleventh is refused.
#[test]
fn the_eleventh_job_is_refused() {
    let log = Log::default();
    let now = Instant::now();
    let mut s = CraftingSession::new(ACCOUNT_ID, PLAYER_ID, None);
    for seq in 0..MAX_INDUCTIONS as u64 {
        assert_ne!(s.enqueue(job("j", 0, &log), seq, now), Enqueued::Full);
    }
    assert_eq!(s.len(), 10);
    assert_eq!(s.enqueue(job("j", 0, &log), 99, now), Enqueued::Full);
    assert_eq!(s.len(), 10, "a refused job is not held");
}

#[test]
fn a_job_is_due_only_at_its_deadline_and_only_once() {
    let log = Log::default();
    let now = Instant::now();
    let mut s = CraftingSession::new(ACCOUNT_ID, PLAYER_ID, None);
    s.enqueue(job("a", 0, &log), 5, now);
    let deadline = now + INDUCTION_DURATION;
    assert!(matches!(
        s.take_due(5, deadline - Duration::from_millis(1)),
        Due::NotYet(d) if d == deadline
    ));
    assert!(matches!(s.take_due(4, deadline), Due::Stale), "wrong seq");
    assert!(matches!(s.take_due(5, deadline), Due::Ready(_)));
    assert!(matches!(s.take_due(5, deadline), Due::Stale), "taken once");
    assert_eq!(s.len(), 1, "busy until finished");
}

/// A burst of refused requests resyncs the client once; a freed slot
/// re-arms the resync.
#[test]
fn only_the_first_refusal_after_a_free_slot_resyncs() {
    let log = Log::default();
    let now = Instant::now();
    let mut s = CraftingSession::new(ACCOUNT_ID, PLAYER_ID, None);
    for seq in 0..MAX_INDUCTIONS as u64 {
        s.enqueue(job("j", 0, &log), seq, now);
    }
    assert!(s.note_refusal());
    assert!(!s.note_refusal());
    let deadline = now + INDUCTION_DURATION;
    assert!(matches!(s.take_due(0, deadline), Due::Ready(_)));
    s.finish(0, deadline);
    assert!(s.note_refusal(), "a freed slot re-arms the resync");
}

/// While a job's completion runs, a new job queues behind the waiting
/// ones instead of starting ahead of them.
#[test]
fn a_new_job_waits_while_a_completion_runs() {
    let log = Log::default();
    let now = Instant::now();
    let mut s = CraftingSession::new(ACCOUNT_ID, PLAYER_ID, None);
    s.enqueue(job("a", 1, &log), 1, now);
    s.enqueue(job("b", 2, &log), 2, now);
    let deadline = now + INDUCTION_DURATION;
    assert!(matches!(s.take_due(1, deadline), Due::Ready(_)));
    assert_eq!(
        s.enqueue(job("c", 3, &log), 3, deadline),
        Enqueued::Queued { position: 2 }
    );
    let next = s.finish(1, deadline).expect("b starts next");
    assert_eq!((next.job_id, next.timer_id), (2, 2));
    assert_eq!(next.deadline, deadline + INDUCTION_DURATION);
    assert_eq!(s.finish(3, deadline), None, "only the active job finishes");
}

// ── The engine ───────────────────────────────────────────────────────────

/// The client bursts up to ten requests; all ten are held and the
/// eleventh is refused with a visible line. Only the running job has a
/// bar.
#[tokio::test]
async fn eleventh_request_is_rejected_with_feedback() {
    let h = Harness::new();
    assert_eq!(h.submit("j0", 100).await, SubmitOutcome::Started);
    for k in 1..MAX_INDUCTIONS {
        assert_eq!(
            h.submit("j", 100 + k as i32).await,
            SubmitOutcome::Queued { position: k }
        );
    }
    assert_eq!(h.sessions.pending(ENTITY), 10);
    assert_eq!(h.timer_ids(), vec![100], "one bar, for the running job");

    assert_eq!(h.submit("j10", 110).await, SubmitOutcome::QueueFull);
    assert_eq!(h.sessions.pending(ENTITY), 10);
    let calls = h.calls();
    assert_eq!(calls.len(), 2, "the bar, then the refusal: {calls:?}");
    assert_eq!(calls[1].method, method_idx::ON_PLAYER_COMMUNICATION);
    assert_eq!(
        feedback_text(&calls[1]),
        "You can have at most 10 crafting jobs at once."
    );
}

/// Jobs complete one at a time, oldest first, and each gets its bar when
/// it becomes the running one.
#[tokio::test]
async fn queued_jobs_run_one_at_a_time_in_order() {
    let h = Harness::new();
    h.submit("a", 1).await;
    h.submit("b", 2).await;
    h.submit("c", 3).await;
    assert_eq!(h.timer_ids(), vec![1]);

    h.fire_next().await;
    assert_eq!(h.completed(), vec!["a"]);
    assert_eq!(h.timer_ids(), vec![1, 2]);

    h.fire_next().await;
    h.fire_next().await;
    assert_eq!(h.completed(), vec!["a", "b", "c"]);
    assert_eq!(h.timer_ids(), vec![1, 2, 3]);
    assert!(h.scheduler.take().is_empty(), "nothing left to wake");
    assert_eq!(h.sessions.pending(ENTITY), 0);
}

#[tokio::test]
async fn an_early_wake_up_runs_nothing_and_waits_again() {
    let h = Harness::new();
    h.submit("a", 1).await;
    let due = h.scheduler.take()[0];
    h.sessions
        .expire_at(
            ENTITY,
            due.job_id,
            due.deadline - Duration::from_millis(1),
            &h.env,
        )
        .await;
    assert!(h.completed().is_empty());
    assert_eq!(
        h.scheduler.take(),
        vec![due],
        "rescheduled for the same deadline"
    );
}

/// Logout drops the queue: the running job and the waiting ones never
/// complete, and the player can start again afterwards.
#[tokio::test]
async fn logout_mid_induction_runs_nothing() {
    let h = Harness::new();
    h.submit("a", 1).await;
    h.submit("b", 2).await;
    assert_eq!(
        h.sessions.drop_player(ENTITY, DropReason::Logout, "test"),
        2
    );
    h.fire_next().await;
    assert!(h.completed().is_empty(), "a dropped induction never runs");
    assert_eq!(h.sessions.pending(ENTITY), 0);
}

/// A teardown path that forgot to drop the queue still runs nothing: the
/// entity no longer maps to a connected session at the deadline. The
/// drop is logged with its own reason, so it is told apart from a hook.
#[tokio::test]
async fn a_disconnected_player_is_never_completed() {
    let capture = LogCapture::install();
    let h = Harness::new();
    h.submit("a", 1).await;
    h.submit("b", 2).await;
    h.env.entity_to_addr.lock().unwrap().remove(&ENTITY);
    h.fire_next().await;
    assert!(h.completed().is_empty());
    assert_eq!(h.sessions.pending(ENTITY), 0, "the queue is dropped");
    assert!(h.scheduler.take().is_empty(), "the next job never starts");
    let event = capture
        .find_event(
            tracing::Level::INFO,
            "crafting queue dropped",
            "session_changed",
        )
        .expect("queue_dropped reason=session_changed");
    assert_eq!(event.target, "crafting");
    assert!(event.has_field("jobs_dropped", "2"), "{event:#?}");
}

/// The entity id was recycled to another character before the deadline
/// (a teardown that missed the drop): the first character's job never
/// runs against the second.
#[tokio::test]
async fn a_recycled_entity_id_never_runs_another_characters_job() {
    let h = Harness::new();
    h.submit("a", 1).await;
    h.env
        .connected
        .lock()
        .unwrap()
        .get_mut(&h.addr)
        .unwrap()
        .active_player_id = Some(PLAYER_ID + 1);
    h.fire_next().await;
    assert!(h.completed().is_empty());
    assert_eq!(h.sessions.pending(ENTITY), 0);
}

/// A request for a character the session is not playing is not queued.
#[tokio::test]
async fn a_request_for_another_character_is_not_queued() {
    let h = Harness::new();
    let outcome = h
        .sessions
        .submit(ENTITY, PLAYER_ID + 1, job("a", 1, &h.log), &h.env)
        .await;
    assert_eq!(outcome, SubmitOutcome::NotConnected);
    assert_eq!(h.sessions.pending(ENTITY), 0);
}

#[tokio::test]
async fn a_world_change_mid_induction_runs_nothing() {
    let h = Harness::new();
    h.submit("a", 1).await;
    h.env
        .connected
        .lock()
        .unwrap()
        .get_mut(&h.addr)
        .unwrap()
        .world_name = Some("Harset".to_string());
    h.fire_next().await;
    assert!(h.completed().is_empty());
    assert_eq!(h.sessions.pending(ENTITY), 0);
}

#[tokio::test]
async fn a_request_from_an_unconnected_entity_is_not_queued() {
    let h = Harness::new();
    h.env.entity_to_addr.lock().unwrap().clear();
    assert_eq!(h.submit("a", 1).await, SubmitOutcome::NotConnected);
    assert_eq!(h.sessions.pending(ENTITY), 0);
    assert!(h.transport.is_empty());
}

/// The production scheduler on tokio's paused clock: nothing completes
/// before three seconds, the job completes at three.
#[tokio::test(start_paused = true)]
async fn tokio_scheduler_completes_at_the_deadline() {
    let h = Harness::new();
    let sessions = Arc::new(CraftingSessions::new(
        Box::new(TokioScheduler),
        Box::new(|| Box::new(ScriptedRng::new(vec![0.5]))),
    ));
    sessions
        .submit(ENTITY, PLAYER_ID, job("a", 1, &h.log), &h.env)
        .await;
    tokio::time::sleep(Duration::from_millis(2_900)).await;
    assert!(h.completed().is_empty(), "not before the deadline");
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(h.completed(), vec!["a"]);
}

// ── The bar ──────────────────────────────────────────────────────────────

/// `onTimerUpdate` for an induction started at game time 100.0: ID 412,
/// Type 16, SourceID = the player, SecondaryID 0, TotalTime 3.0,
/// BigWorldTimeComplete 103.0, little-endian.
#[test]
fn induction_timer_args_are_byte_exact() {
    let args = induction_timer_args(0x0102_0304, 412, 100.0);
    let mut expected = Vec::new();
    expected.extend_from_slice(&412i32.to_le_bytes());
    expected.push(16);
    expected.extend_from_slice(&0x0102_0304i32.to_le_bytes());
    expected.extend_from_slice(&0i32.to_le_bytes());
    expected.extend_from_slice(&3.0f32.to_le_bytes());
    expected.extend_from_slice(&103.0f32.to_le_bytes());
    assert_eq!(args, expected);
    assert_eq!(
        args,
        [
            0x9C, 0x01, 0x00, 0x00, // ID 412
            0x10, // Type 16
            0x04, 0x03, 0x02, 0x01, // SourceID
            0x00, 0x00, 0x00, 0x00, // SecondaryID
            0x00, 0x00, 0x40, 0x40, // TotalTime 3.0
            0x00, 0x00, 0xCE, 0x42, // BigWorldTimeComplete 103.0
        ]
    );
}

/// The bar a started induction sends expires three seconds from now on
/// the server-wide game clock (absolute, not the relative 3.0 and not 0).
#[tokio::test]
async fn started_induction_sends_the_type_16_bar_with_an_absolute_expiry() {
    // The clock starts at its first read; `d + 1e-7` rounds to `d` in
    // f32, so sample only once it is clearly past its epoch.
    game_clock::init();
    while game_clock::game_time_secs() < 0.01 {
        tokio::task::yield_now().await;
    }
    let h = Harness::new();
    let before = game_clock::game_time_secs();
    h.submit("a", 412).await;
    let after = game_clock::game_time_secs();

    let calls = h.calls();
    assert_eq!(calls.len(), 1);
    let call = &calls[0];
    assert_eq!(call.method, ON_TIMER_UPDATE);
    assert_eq!(call.entity_id, ENTITY);
    let a = &call.args;
    assert_eq!(a.len(), 21);
    assert_eq!(i32::from_le_bytes(a[0..4].try_into().unwrap()), 412);
    assert_eq!(a[4], 16, "TIMER_CRAFT_INDUCTION");
    assert_eq!(u32::from_le_bytes(a[5..9].try_into().unwrap()), ENTITY);
    assert_eq!(i32::from_le_bytes(a[9..13].try_into().unwrap()), 0);
    assert_eq!(f32::from_le_bytes(a[13..17].try_into().unwrap()), 3.0);
    let complete = f32::from_le_bytes(a[17..21].try_into().unwrap());
    assert!(
        before + 3.0 <= complete && complete <= after + 3.0,
        "expiry {complete} must be now + 3 (now in [{before}, {after}])"
    );
}

// ── Teardown hook ────────────────────────────────────────────────────────

/// Every disconnect path funnels through `destroy_client_entities`; it
/// must drop the player's queue on the server's sessions.
#[tokio::test]
async fn destroying_the_client_drops_its_crafting_queue() {
    // A private entity id: the server-wide sessions are shared by every
    // test in this process.
    const HOOK_ENTITY: u32 = 4290;
    let h = Harness::new();
    h.env.entity_to_addr.lock().unwrap().clear();
    h.env
        .entity_to_addr
        .lock()
        .unwrap()
        .insert(HOOK_ENTITY, h.addr);
    h.env
        .connected
        .lock()
        .unwrap()
        .get_mut(&h.addr)
        .unwrap()
        .player_entity_id = Some(HOOK_ENTITY);
    // `active_player_id` is already `PLAYER_ID`.
    let global = crafting_sessions();
    assert_eq!(
        global
            .submit(HOOK_ENTITY, PLAYER_ID, job("a", 1, &h.log), &h.env)
            .await,
        SubmitOutcome::Started
    );
    assert_eq!(global.pending(HOOK_ENTITY), 1);

    let entity_manager = Arc::new(Mutex::new(cimmeria_entity::manager::EntityManager::new()));
    crate::base::helpers::destroy_client_entities(
        &h.env.connected,
        &entity_manager,
        h.addr,
        &None,
        &h.env.entity_to_addr,
        &(std::sync::Arc::new(crate::test_support::TestTransport::new())
            as std::sync::Arc<dyn cimmeria_mercury::transport::Transport>),
        &None,
        "client_disconnect",
    )
    .await;
    assert_eq!(global.pending(HOOK_ENTITY), 0);
}
