//! Every player's crafting session, and the wake-ups that complete them.
//!
//! [`CraftingSessions::submit`] queues a job and, when it becomes active,
//! sends the induction bar and asks the [`InductionScheduler`] to call
//! [`CraftingSessions::expire`] at its deadline. `expire` runs the job, then
//! starts the next one. [`CraftingSessions::drop_player`] forgets a
//! player's queue without running anything; the base's logout, disconnect
//! and world-change paths call it through [`drop_player_inductions`].
//!
//! Before a job runs, `expire` also checks that the entity still belongs to
//! a connected session playing the same character, with the world name
//! the queue started under. A teardown path that forgot to drop the queue
//! therefore still consumes nothing, and a recycled entity id never runs
//! another character's job. The session's world name is set at world
//! entry and is not rewritten by gate travel, so a gate-travel world
//! change is covered only by the drop in `handle_gate_travel`.
//!
//! Every job logs `queued` (if it waits), `induction_started`,
//! `induction_expired` and `completed`, or `queue_dropped`, all under
//! target `crafting` with its `job_id`, `verb` and the player's
//! `account_id`, `player_id` and `entity_id`. `crafting_jobs_total` counts
//! each job once when it ends. There is no span here: this is per-job
//! timer work, not a dispatch entrypoint.

use crate::base::crafting::telemetry as crafting_telemetry;
use cimmeria_entity::known_names;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use tokio::time::Instant;

pub use super::scheduler::{DueInduction, InductionScheduler, ManualScheduler, TokioScheduler};
use super::state::{CraftingSession, Due, Enqueued, Started};
use super::timer::send_induction_timer;
use super::{Completion, InductionEnv, InductionJob, JobOutcome, MAX_INDUCTIONS};
use crate::base::crafting::feedback::{reject, CraftReject};
use crate::base::crafting::rng::{CraftRng, OsSeededRng};
use crate::base::crafting::telemetry::{count_job, JobEnd, JobIds};
use crate::base::crafting::transaction::resync_inventory;

type RngFactory = Box<dyn Fn() -> Box<dyn CraftRng> + Send + Sync>;

/// What [`CraftingSessions::submit`] did with a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmitOutcome {
    /// The job is running; its bar was sent.
    Started,
    /// The job waits; `position` 1 is next.
    Queued { position: usize },
    /// Refused: the player already has [`MAX_INDUCTIONS`]. The player was
    /// told.
    QueueFull,
    /// The entity has no session playing this character; the job was
    /// dropped.
    NotConnected,
}

/// Why a player's queue was dropped: the `reason` of `queue_dropped`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropReason {
    /// The session ended: disconnect, timeout, kick or `logOff`.
    Logout,
    /// Gate travel to another world.
    WorldChange,
    /// At a deadline the entity no longer belonged to a session playing
    /// the same character under the same world name (a teardown that
    /// skipped the drop, or a recycled entity id).
    SessionChanged,
    /// A request found a leftover queue from another character or world
    /// on the same entity id.
    StaleSession,
    /// A job was submitted for an entity with no session playing the
    /// character.
    NotConnected,
    /// The player confirmed a crafting respec: the disciplines a queued job
    /// would use are about to be cleared.
    Respec,
}

impl DropReason {
    pub fn label(self) -> &'static str {
        match self {
            DropReason::Logout => "logout",
            DropReason::WorldChange => "world_change",
            DropReason::SessionChanged => "session_changed",
            DropReason::StaleSession => "stale_session",
            DropReason::NotConnected => "not_connected",
            DropReason::Respec => "respec",
        }
    }
}

/// Every player's [`CraftingSession`], keyed by player entity id.
pub struct CraftingSessions {
    sessions: Mutex<HashMap<u32, CraftingSession>>,
    next_job_id: AtomicU64,
    scheduler: Box<dyn InductionScheduler>,
    rng: RngFactory,
}

static SESSIONS: LazyLock<Arc<CraftingSessions>> = LazyLock::new(|| {
    Arc::new(CraftingSessions::new(
        Box::new(TokioScheduler),
        Box::new(|| Box::new(OsSeededRng::new())),
    ))
});

/// The server's crafting sessions.
pub fn crafting_sessions() -> &'static Arc<CraftingSessions> {
    &SESSIONS
}

/// Drop `entity_id`'s inductions without running them. Called from the
/// base's logout, disconnect and world-change paths; `cause` is the
/// caller's own label (the disconnect reason, `log_off`, `gate_travel`).
pub fn drop_player_inductions(entity_id: u32, reason: DropReason, cause: &'static str) {
    SESSIONS.drop_player(entity_id, reason, cause);
}

/// An optional id as an event field: the number, or empty.
fn opt<T: Display>(v: Option<T>) -> String {
    v.map(|v| v.to_string()).unwrap_or_default()
}

fn log_dropped(
    ids: &JobIds,
    reason: DropReason,
    cause: &'static str,
    jobs: &[(u64, &'static str)],
) {
    let job_ids = jobs
        .iter()
        .map(|(id, verb)| format!("{id}:{verb}"))
        .collect::<Vec<_>>()
        .join(",");
    let player_label = known_names::player_name(ids.player_id);
    tracing::info!(
        target: "crafting",
        event = "queue_dropped",
        account_id = ids.account_id,
        account_name = known_names::account_name(ids.account_id),
        player_id = ids.player_id,
        player_name = player_label,
        entity_id = ids.entity_id,
        entity_name = player_label,
        reason = reason.label(),
        cause,
        jobs_dropped = jobs.len(),
        job_ids = %job_ids,
        "crafting queue dropped; nothing was consumed"
    );
    for (_, verb) in jobs {
        count_job(verb, JobEnd::Dropped);
    }
}

impl CraftingSessions {
    pub fn new(scheduler: Box<dyn InductionScheduler>, rng: RngFactory) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            next_job_id: AtomicU64::new(1),
            scheduler,
            rng,
        }
    }

    /// Inductions `entity_id` holds, the active one included.
    pub fn pending(&self, entity_id: u32) -> usize {
        self.sessions
            .lock()
            .unwrap()
            .get(&entity_id)
            .map_or(0, CraftingSession::len)
    }

    /// Queue `job` for the player. Starts it (bar and wake-up) when nothing
    /// else is running; refuses it, with feedback, past [`MAX_INDUCTIONS`].
    pub async fn submit(
        self: &Arc<Self>,
        entity_id: u32,
        player_id: i32,
        job: Box<dyn InductionJob>,
        env: &InductionEnv,
    ) -> SubmitOutcome {
        let job_id = self.next_job_id.fetch_add(1, Ordering::Relaxed);
        let verb = job.verb();
        let Some(owner) = env.session_owner(entity_id, player_id) else {
            let ids = JobIds {
                job_id,
                verb,
                account_id: env.account_of(entity_id),
                player_id,
                entity_id,
                gm_entity_id: None,
                gm_name: None,
            };
            log_dropped(&ids, DropReason::NotConnected, "submit", &[(job_id, verb)]);
            return SubmitOutcome::NotConnected;
        };
        let ids = JobIds {
            job_id,
            verb,
            account_id: owner.account_id,
            player_id,
            entity_id,
            gm_entity_id: None,
            gm_name: None,
        };
        let (enqueued, resync, queue_len, stale) = {
            let mut sessions = self.sessions.lock().unwrap();
            let session = sessions.entry(entity_id).or_insert_with(|| {
                CraftingSession::new(owner.account_id, player_id, owner.world.clone())
            });
            let mut stale = None;
            if session.player_id() != player_id || session.world() != owner.world.as_deref() {
                // A leftover from an earlier character or world on a
                // recycled entity id: never run it for this one. The drop
                // is logged under the session that queued it.
                let held = session.held_jobs();
                if !held.is_empty() {
                    let old = JobIds {
                        job_id: held[0].0,
                        verb: held[0].1,
                        account_id: session.account_id(),
                        player_id: session.player_id(),
                        entity_id,
                        gm_entity_id: None,
                        gm_name: None,
                    };
                    stale = Some((old, held));
                }
                *session = CraftingSession::new(owner.account_id, player_id, owner.world.clone());
            }
            let enqueued = session.enqueue(job, job_id, Instant::now());
            let resync = enqueued == Enqueued::Full && session.note_refusal();
            (enqueued, resync, session.len(), stale)
        };
        if let Some((old, held)) = stale {
            log_dropped(&old, DropReason::StaleSession, "submit", &held);
        }
        match enqueued {
            Enqueued::Started(started) => {
                self.start(&ids, started, queue_len, env).await;
                SubmitOutcome::Started
            }
            Enqueued::Queued { position } => {
                let player_label = known_names::player_name(player_id);
                tracing::info!(
                    target: "crafting",
                    event = "queued",
                    job_id, // nt:id-only induction job counter, unnamed
                    verb,
                    account_id = ids.account_id,
                    account_name = known_names::account_name(ids.account_id),
                    player_id,
                    player_name = player_label,
                    entity_id,
                    entity_name = player_label,
                    position,
                    queue_len,
                    "crafting induction queued"
                );
                SubmitOutcome::Queued { position }
            }
            Enqueued::Full => {
                let why = CraftReject::QueueFull {
                    limit: MAX_INDUCTIONS,
                };
                reject(verb, entity_id, player_id, &why, env.client()).await;
                if let (true, Some(pool)) = (resync, &env.db_pool) {
                    resync_inventory(env, pool, &ids).await;
                }
                SubmitOutcome::QueueFull
            }
        }
    }

    /// Complete `job_id` if it is still `entity_id`'s active induction and
    /// its deadline has passed, then start the next queued job.
    pub async fn expire(self: &Arc<Self>, entity_id: u32, job_id: u64, env: &InductionEnv) {
        self.expire_at(entity_id, job_id, Instant::now(), env).await;
    }

    /// [`CraftingSessions::expire`] at an explicit time, for tests.
    pub async fn expire_at(
        self: &Arc<Self>,
        entity_id: u32,
        job_id: u64,
        now: Instant,
        env: &InductionEnv,
    ) {
        let (due, ids, world, queue_len) = {
            let mut sessions = self.sessions.lock().unwrap();
            let Some(session) = sessions.get_mut(&entity_id) else {
                return;
            };
            let ids = JobIds {
                job_id,
                verb: "",
                account_id: session.account_id(),
                player_id: session.player_id(),
                entity_id,
                gm_entity_id: None,
                gm_name: None,
            };
            (
                session.take_due(job_id, now),
                ids,
                session.world().map(str::to_owned),
                session.len(),
            )
        };
        let job = match due {
            Due::Ready(job) => job,
            Due::Stale => return,
            Due::NotYet(deadline) => {
                self.scheduler.schedule(
                    self.clone(),
                    env.clone(),
                    DueInduction {
                        entity_id,
                        job_id,
                        deadline,
                    },
                );
                return;
            }
        };
        let verb = job.verb();
        let ids = JobIds { verb, ..ids };
        let player_label = known_names::player_name(ids.player_id);
        tracing::info!(
            target: "crafting",
            event = "induction_expired",
            job_id, // nt:id-only induction job counter, unnamed
            verb,
            account_id = ids.account_id,
            account_name = known_names::account_name(ids.account_id),
            player_id = ids.player_id,
            player_name = player_label,
            entity_id,
            entity_name = player_label,
            queue_len,
            "crafting induction expired"
        );

        let owner = env.session_owner(entity_id, ids.player_id);
        if owner.map(|o| o.world) != Some(world) {
            let mut dropped = vec![(job_id, verb)];
            if let Some(session) = self.sessions.lock().unwrap().remove(&entity_id) {
                dropped.extend(session.held_jobs());
            }
            log_dropped(&ids, DropReason::SessionChanged, "expire", &dropped);
            return;
        }

        let mut rng = (self.rng)();
        let outcome = job
            .complete(Completion {
                ids,
                env,
                rng: rng.as_mut(),
            })
            .await;
        match outcome {
            JobOutcome::Completed(report) => {
                let player_label = known_names::player_name(ids.player_id);
                tracing::info!(
                    target: "crafting",
                    event = "completed",
                    job_id, // nt:id-only induction job counter, unnamed
                    verb,
                    account_id = ids.account_id,
                    account_name = known_names::account_name(ids.account_id),
                    player_id = ids.player_id,
                    player_name = player_label,
                    entity_id,
                    entity_name = player_label,
                    blueprint_id = %opt(report.blueprint_id),
                    blueprint_name = crafting_telemetry::blueprint_name(report.blueprint_id),
                    item_id = %opt(report.item_id), // nt:id-only instance, see consumed
                    consumed = %report.consumed,
                    granted = %report.granted,
                    expertise = %report.expertise,
                    result = report.result.unwrap_or(""),
                    chance = %opt(report.chance),
                    roll = %opt(report.roll),
                    component_set_id = %opt(report.component_set_id), // nt:id-only recipe index
                    discipline_id = %opt(report.discipline_id),
                    discipline_name = crafting_telemetry::discipline_name(report.discipline_id),
                    eligible_disciplines = %report.eligible_disciplines,
                    bias = %opt(report.bias),
                    rolls = %report.rolls,
                    blueprints_learned = %report.blueprints_learned,
                    quality_bucket = report.quality_bucket.unwrap_or(""),
                    elementary = %report.elementary,
                    quantity = %opt(report.quantity),
                    "crafting induction completed"
                );
                count_job(verb, JobEnd::Completed);
            }
            JobOutcome::Failed => count_job(verb, JobEnd::Failed),
        }

        let (next, queue_len) = {
            let mut sessions = self.sessions.lock().unwrap();
            let Some(session) = sessions.get_mut(&entity_id) else {
                return;
            };
            let next = session.finish(job_id, Instant::now());
            let queue_len = session.len();
            if session.is_empty() {
                sessions.remove(&entity_id);
            }
            (next, queue_len)
        };
        if let Some(started) = next {
            let next_ids = JobIds {
                job_id: started.job_id,
                verb: started.verb,
                ..ids
            };
            self.start(&next_ids, started, queue_len, env).await;
        }
    }

    /// Forget `entity_id`'s inductions without running them. Returns how
    /// many were dropped. A job whose completion is already running
    /// finishes and is counted by its own outcome; its transaction is
    /// atomic either way.
    pub fn drop_player(&self, entity_id: u32, reason: DropReason, cause: &'static str) -> usize {
        let Some(session) = self.sessions.lock().unwrap().remove(&entity_id) else {
            return 0;
        };
        let held = session.held_jobs();
        let ids = JobIds {
            job_id: held.first().map_or(0, |(id, _)| *id),
            verb: held.first().map_or("", |(_, verb)| *verb),
            account_id: session.account_id(),
            player_id: session.player_id(),
            entity_id,
            gm_entity_id: None,
            gm_name: None,
        };
        log_dropped(&ids, reason, cause, &held);
        held.len()
    }

    /// Whether `job_id` is still the active induction of `ids`'s session,
    /// and the entity still belongs to that session's character, account
    /// and world.
    fn still_active(&self, ids: &JobIds, job_id: u64, env: &InductionEnv) -> bool {
        let world = {
            let sessions = self.sessions.lock().unwrap();
            match sessions.get(&ids.entity_id) {
                Some(s) if s.active_job_id() == Some(job_id) && s.player_id() == ids.player_id => {
                    s.world().map(str::to_owned)
                }
                _ => return false,
            }
        };
        env.session_owner(ids.entity_id, ids.player_id)
            .is_some_and(|o| o.account_id == ids.account_id && o.world == world)
    }

    /// Send a newly active induction's bar and schedule its wake-up. The
    /// session lock was released before this runs, so a logout, a world
    /// change or an entity recycle may have happened in between: the job
    /// is re-checked first, and a job no longer active for the same
    /// session sends nothing (its drop was already logged and counted).
    pub(super) async fn start(
        self: &Arc<Self>,
        ids: &JobIds,
        started: Started,
        queue_len: usize,
        env: &InductionEnv,
    ) {
        if !self.still_active(ids, started.job_id, env) {
            tracing::debug!(
                target: "crafting",
                event = "induction_start_skipped",
                job_id = started.job_id, // nt:id-only induction job counter, unnamed
                verb = started.verb,
                account_id = ids.account_id,
                account_name = known_names::account_name(ids.account_id),
                player_id = ids.player_id,
                player_name = known_names::player_name(ids.player_id),
                entity_id = ids.entity_id,
                entity_name = known_names::player_name(ids.player_id),
                "crafting induction no longer active; no bar sent"
            );
            return;
        }
        let expires_at = send_induction_timer(env, ids, started.timer_id).await;
        let player_label = known_names::player_name(ids.player_id);
        tracing::info!(
            target: "crafting",
            event = "induction_started",
            job_id = started.job_id, // nt:id-only induction job counter, unnamed
            verb = started.verb,
            account_id = ids.account_id,
            account_name = known_names::account_name(ids.account_id),
            player_id = ids.player_id,
            player_name = player_label,
            entity_id = ids.entity_id,
            entity_name = player_label,
            timer_id = started.timer_id, // nt:id-only client timer id, unnamed
            queue_len,
            expires_at,
            "crafting induction started"
        );
        self.scheduler.schedule(
            self.clone(),
            env.clone(),
            DueInduction {
                entity_id: ids.entity_id,
                job_id: started.job_id,
                deadline: started.deadline,
            },
        );
    }
}
