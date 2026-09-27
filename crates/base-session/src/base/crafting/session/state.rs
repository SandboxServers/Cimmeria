//! One player's inductions: the pure state machine.

use std::collections::VecDeque;

use tokio::time::Instant;

use super::{InductionJob, INDUCTION_DURATION, MAX_INDUCTIONS};

/// An induction that just became the active one: the engine sends its
/// timer and schedules its wake-up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Started {
    pub job_id: u64,
    pub deadline: Instant,
    pub timer_id: i32,
    pub verb: &'static str,
}

/// Result of [`CraftingSession::enqueue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enqueued {
    /// Nothing was running: this job is active now.
    Started(Started),
    /// Waiting behind others; `position` 1 is next.
    Queued { position: usize },
    /// The session already holds [`MAX_INDUCTIONS`].
    Full,
}

/// Result of [`CraftingSession::take_due`].
pub enum Due {
    /// The active induction is `job_id` and has run its time: run this job.
    Ready(Box<dyn InductionJob>),
    /// `job_id` is active but its deadline is later.
    NotYet(Instant),
    /// `job_id` is not the active induction (dropped, or already taken).
    Stale,
}

struct Active {
    job_id: u64,
    deadline: Instant,
    verb: &'static str,
    /// `None` while the job's completion runs: the session stays busy, so
    /// a new request queues behind the rest instead of jumping ahead.
    job: Option<Box<dyn InductionJob>>,
}

/// One player's inductions.
pub struct CraftingSession {
    account_id: u32,
    player_id: i32,
    world: Option<String>,
    active: Option<Active>,
    queue: VecDeque<(u64, Box<dyn InductionJob>)>,
    /// A full-queue refusal already resynced the client's inventory. Reset
    /// when a slot frees, so a burst of refused requests costs one resync.
    refusal_resynced: bool,
}

impl CraftingSession {
    pub fn new(account_id: u32, player_id: i32, world: Option<String>) -> Self {
        Self {
            account_id,
            player_id,
            world,
            active: None,
            queue: VecDeque::new(),
            refusal_resynced: false,
        }
    }

    pub fn account_id(&self) -> u32 {
        self.account_id
    }

    pub fn player_id(&self) -> i32 {
        self.player_id
    }

    pub fn world(&self) -> Option<&str> {
        self.world.as_deref()
    }

    /// Inductions held, the active one included.
    pub fn len(&self) -> usize {
        usize::from(self.active.is_some()) + self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The active induction's job id, if one is running.
    pub fn active_job_id(&self) -> Option<u64> {
        self.active.as_ref().map(|a| a.job_id)
    }

    /// The active induction's deadline, if one is running.
    pub fn deadline(&self) -> Option<Instant> {
        self.active.as_ref().map(|a| a.deadline)
    }

    /// Record a full-queue refusal. `true` for the first since a slot last
    /// freed: that one resyncs the client's inventory, the rest only get
    /// the feedback line.
    pub fn note_refusal(&mut self) -> bool {
        !std::mem::replace(&mut self.refusal_resynced, true)
    }

    /// Add job `job_id`. If nothing is running it starts now; otherwise it
    /// waits at the back of the queue.
    pub fn enqueue(&mut self, job: Box<dyn InductionJob>, job_id: u64, now: Instant) -> Enqueued {
        if self.len() >= MAX_INDUCTIONS {
            return Enqueued::Full;
        }
        if self.active.is_none() {
            return Enqueued::Started(self.activate(job, job_id, now));
        }
        self.queue.push_back((job_id, job));
        Enqueued::Queued {
            position: self.queue.len(),
        }
    }

    /// Hand out the active job if it is `job_id` and its deadline has
    /// passed. The session stays busy until [`CraftingSession::finish`].
    pub fn take_due(&mut self, job_id: u64, now: Instant) -> Due {
        let Some(active) = self.active.as_mut().filter(|a| a.job_id == job_id) else {
            return Due::Stale;
        };
        if now < active.deadline {
            return Due::NotYet(active.deadline);
        }
        match active.job.take() {
            Some(job) => Due::Ready(job),
            None => Due::Stale,
        }
    }

    /// The active induction `job_id` has completed: clear it and start the
    /// next queued job, if any.
    pub fn finish(&mut self, job_id: u64, now: Instant) -> Option<Started> {
        if self.active.as_ref().map(|a| a.job_id) != Some(job_id) {
            return None;
        }
        self.active = None;
        self.refusal_resynced = false;
        let (next_id, next) = self.queue.pop_front()?;
        Some(self.activate(next, next_id, now))
    }

    /// The jobs a drop discards unrun, as `(job_id, verb)`: the active job
    /// unless its completion is already running, then the queue.
    pub fn held_jobs(&self) -> Vec<(u64, &'static str)> {
        self.active
            .iter()
            .filter(|a| a.job.is_some())
            .map(|a| (a.job_id, a.verb))
            .chain(self.queue.iter().map(|(id, job)| (*id, job.verb())))
            .collect()
    }

    fn activate(&mut self, job: Box<dyn InductionJob>, job_id: u64, now: Instant) -> Started {
        let started = Started {
            job_id,
            deadline: now + INDUCTION_DURATION,
            timer_id: job.timer_id(),
            verb: job.verb(),
        };
        self.active = Some(Active {
            job_id,
            deadline: started.deadline,
            verb: started.verb,
            job: Some(job),
        });
        started
    }
}
