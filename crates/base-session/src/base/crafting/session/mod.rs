//! Per-player crafting inductions: one runs, the rest wait in a FIFO queue.
//!
//! A crafting verb validates its request, then hands the base a job. The job
//! waits its turn here, runs for [`INDUCTION_DURATION`] with the client's
//! induction bar on screen, and only then does its work (for the item verbs,
//! one [`super::transaction`] that consumes and grants). Nothing is consumed
//! at the request, so a logout, a world change or a crash during the bar
//! loses nothing.
//!
//! - [`state`]: [`CraftingSession`], the pure state machine for one player:
//!   the active induction and its monotonic deadline, and the queue. Time is
//!   a parameter, so tests drive it without sleeping.
//! - [`engine`] holds every player's session, starts the bar ([`timer`]),
//!   wakes at each deadline, runs the job, and drops a player's queue on
//!   logout or world change.
//!
//! The session stores the deadline as a monotonic `Instant`, never the
//! absolute game time sent to the client: the game clock restarts with the
//! server.

use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;
use tokio::time::Duration;

use super::request::CraftCtx;
use super::rng::CraftRng;
use super::sync::CraftClient;
use super::telemetry::JobIds;
use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;

pub mod engine;
pub mod scheduler;
pub mod state;
pub mod timer;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_telemetry;

pub use engine::{
    crafting_sessions, drop_player_inductions, CraftingSessions, DropReason, DueInduction,
    InductionScheduler, ManualScheduler, SubmitOutcome, TokioScheduler,
};
pub use state::{CraftingSession, Due, Enqueued, Started};

/// Most inductions a player may have at once, the running one included.
/// The reverse-engineering page sends up to 10 requests in one burst, so
/// all 10 are accepted and the 11th is refused.
pub const MAX_INDUCTIONS: usize = 10;

/// Length of every induction, as sent in the timer's `TotalTime`.
pub const INDUCTION_SECS: f32 = 3.0;

/// [`INDUCTION_SECS`] as the server-side wait.
pub const INDUCTION_DURATION: Duration = Duration::from_secs(3);

/// The future a job's completion returns.
pub type JobFuture<'a> = Pin<Box<dyn Future<Output = JobOutcome> + Send + 'a>>;

/// One queued unit of crafting work. The verbs implement it; the engine
/// only times it.
pub trait InductionJob: Send + 'static {
    /// The verb, for the `verb` event field and the metric labels: the
    /// cell method name (`CraftVerb::method_name`), so a job's counts line
    /// up with its request's.
    fn verb(&self) -> &'static str;

    /// The `onTimerUpdate` `ID`: the blueprint for a craft, 0 for the
    /// verbs that have none.
    fn timer_id(&self) -> i32;

    /// Do the work once the induction has run its time. Runs only while
    /// the same character is still connected on the entity, under the
    /// world name the queue started with.
    fn complete<'a>(self: Box<Self>, done: Completion<'a>) -> JobFuture<'a>;
}

/// What a completing job gets.
pub struct Completion<'a> {
    pub ids: JobIds,
    pub env: &'a InductionEnv,
    pub rng: &'a mut dyn CraftRng,
}

/// How a job's work ended. The engine logs `completed` from the report and
/// counts `crafting_jobs_total` once from either arm.
#[derive(Debug, Clone, PartialEq)]
pub enum JobOutcome {
    /// Boxed: the report carries every verb's fields, and `Failed` none.
    Completed(Box<JobReport>),
    /// Refused or rolled back; the job already told the player and logged
    /// why (`rejected` or `persist_failed`).
    Failed,
}

/// The `completed` event's verb-specific fields. Strings are the
/// transaction's before/after lists ([`super::transaction::CraftApplied`]);
/// empty when the verb has none.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct JobReport {
    pub blueprint_id: Option<i32>,
    pub item_id: Option<i32>,
    /// `item_id:type_id:qty_before→qty_after`, comma-separated.
    pub consumed: String,
    /// `type_id:bag:slot:qty_before→qty_after`, comma-separated.
    pub granted: String,
    /// `discipline_id:expertise_before→expertise_after`, comma-separated.
    pub expertise: String,
    /// `success` or `failure` for the verbs that roll.
    pub result: Option<&'static str>,
    pub chance: Option<f64>,
    pub roll: Option<f64>,
    /// The component set a verb used or picked.
    pub component_set_id: Option<i32>,
    /// The discipline a research roll was made for.
    pub discipline_id: Option<i32>,
    /// The disciplines a research could roll for, comma-separated.
    pub eligible_disciplines: String,
    /// The reverse-engineering recovery bias, `min(1, max(expertise, 1) /
    /// tech competency)`.
    pub bias: Option<f64>,
    /// Reverse engineering, per component of the picked set:
    /// `design_id:roll:recovered/quantity`, comma-separated.
    pub rolls: String,
    /// `blueprint_id:known_before→known_after` per blueprint taught,
    /// comma-separated.
    pub blueprints_learned: String,
}

/// The base handles an induction needs after the request that queued it
/// has returned: owned clones of [`CraftCtx`]'s borrows.
#[derive(Clone)]
pub struct InductionEnv {
    pub db_pool: Option<Arc<PgPool>>,
    pub cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    pub transport: Arc<dyn Transport>,
    pub connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

/// The session a job belongs to, read from the connected-client state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionOwner {
    pub account_id: u32,
    pub world: Option<String>,
}

impl InductionEnv {
    pub fn from_ctx(ctx: &CraftCtx<'_>) -> Self {
        Self {
            db_pool: ctx.db_pool.clone(),
            cell_tx: ctx.cell_tx.clone(),
            transport: ctx.transport.clone(),
            connected: ctx.connected.clone(),
            entity_to_addr: ctx.entity_to_addr.clone(),
        }
    }

    /// The player-client half, for the refusal line.
    pub fn client(&self) -> CraftClient<'_> {
        CraftClient {
            transport: &self.transport,
            connected: &self.connected,
            entity_to_addr: &self.entity_to_addr,
        }
    }

    /// The session of `player_id`'s character, if `entity_id` still maps
    /// to a connected session playing that character. `None` once the
    /// player logged out, disconnected or went back to character select,
    /// and when a recycled entity id now belongs to another character.
    pub fn session_owner(&self, entity_id: u32, player_id: i32) -> Option<SessionOwner> {
        let addr = *self.entity_to_addr.lock().ok()?.get(&entity_id)?;
        let clients = self.connected.lock().ok()?;
        let client = clients.get(&addr)?;
        (client.player_entity_id == Some(entity_id) && client.active_player_id == Some(player_id))
            .then(|| SessionOwner {
                account_id: client.account_id,
                world: client.world_name.clone(),
            })
    }

    /// The account of whatever session `entity_id` maps to, for logging a
    /// refusal that has no owner. 0 when there is none.
    pub fn account_of(&self, entity_id: u32) -> u32 {
        let Some(addr) = self
            .entity_to_addr
            .lock()
            .ok()
            .and_then(|m| m.get(&entity_id).copied())
        else {
            return 0;
        };
        self.connected
            .lock()
            .ok()
            .and_then(|c| c.get(&addr).map(|c| c.account_id))
            .unwrap_or(0)
    }
}
