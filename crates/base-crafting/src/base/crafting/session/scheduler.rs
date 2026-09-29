//! How the engine waits for an induction's deadline.

use std::sync::{Arc, Mutex};

use super::engine::CraftingSessions;
use super::InductionEnv;

/// An induction's wake-up: call [`CraftingSessions::expire`] for `job_id`
/// at `deadline`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DueInduction {
    pub entity_id: u32,
    pub job_id: u64,
    pub deadline: tokio::time::Instant,
}

/// Production sleeps on the tokio clock ([`TokioScheduler`]); tests record
/// the wake-ups and fire them by hand ([`ManualScheduler`]).
pub trait InductionScheduler: Send + Sync {
    fn schedule(&self, sessions: Arc<CraftingSessions>, env: InductionEnv, due: DueInduction);
}

/// Sleeps until the deadline on a spawned task, then expires the induction.
#[derive(Debug, Default)]
pub struct TokioScheduler;

impl InductionScheduler for TokioScheduler {
    fn schedule(&self, sessions: Arc<CraftingSessions>, env: InductionEnv, due: DueInduction) {
        tokio::spawn(async move {
            tokio::time::sleep_until(due.deadline).await;
            sessions.expire(due.entity_id, due.job_id, &env).await;
        });
    }
}

/// Records every wake-up; a test fires them with
/// [`CraftingSessions::expire_at`].
#[derive(Debug, Default)]
pub struct ManualScheduler {
    due: Mutex<Vec<DueInduction>>,
}

impl ManualScheduler {
    /// The wake-ups scheduled since the last call, oldest first.
    pub fn take(&self) -> Vec<DueInduction> {
        std::mem::take(&mut *self.due.lock().unwrap())
    }
}

impl InductionScheduler for ManualScheduler {
    fn schedule(&self, _sessions: Arc<CraftingSessions>, _env: InductionEnv, due: DueInduction) {
        self.due.lock().unwrap().push(due);
    }
}

impl<T: InductionScheduler + ?Sized> InductionScheduler for Arc<T> {
    fn schedule(&self, sessions: Arc<CraftingSessions>, env: InductionEnv, due: DueInduction) {
        (**self).schedule(sessions, env, due);
    }
}
