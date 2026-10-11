//! `.spawnrandom` spawn batches (lab-chaos CH-13, D-CH9).
//!
//! `.spawnrandom` enqueues one `GmSpawnNpc` per copy, and each copy's entity
//! id is only allocated later, when the base's `GmSpawnNpcReady` comes back
//! to the cell. To report every id in one line, the command opens a batch
//! here and tags each request with its id; the cell's `GmSpawnNpcReady` /
//! `GmSpawnNpcFailed` handlers record each copy's result against it, and the
//! batch closes with one summary line once every copy is accounted for or
//! [`SPAWN_BATCH_TIMEOUT`] passes ([`spawn_batch_tick`]).
//!
//! Nothing waits: the batch is plain state in [`SpaceManager::resources`],
//! advanced by messages and the 1 Hz tick, so the cell loop and the console
//! keep running for everyone else.
//!
//! The lab console tee (`server_console_exec`) captures only the lines a
//! command emits while it runs. When the command opened a batch, the tee
//! parks its reply on the batch ([`attach_lab`]) instead of answering at
//! once; the summary line is appended to the captured lines and the reply
//! sent when the batch closes. In-world, the summary goes to the GM as an
//! ordinary feedback line.
//!
//! The summary format is a contract with the lab runner's `burst` parser
//! (CH-08), documented in `docs/analysis/lab-chaos/worknotes/CH-13.md`:
//!
//! ```text
//! spawnrandom: spawned <n>/<count> x template <T>: ids <id,id,...|none>[; <f> failed (<reason>[; <reason>])][; <t> timed out]
//! ```

use std::collections::HashMap;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;

use super::super::send_gm_feedback;
use crate::cell::messages::{CellToBaseMsg, LabConsoleResult};
use crate::cell::space_manager::SpaceManager;

/// How long a batch waits for its copies before reporting the missing ones
/// as timed out. A base round trip is one template query, normally a few
/// milliseconds; five seconds is long enough for a loaded database and
/// short enough for a lab step to wait on.
pub const SPAWN_BATCH_TIMEOUT: Duration = Duration::from_secs(5);

/// A lab console reply parked on a batch: the lines the command captured
/// while it ran, and the reply to send once the summary is known.
#[derive(Debug)]
pub struct LabWaiter {
    pub lines: Vec<String>,
    pub reply_tx: oneshot::Sender<LabConsoleResult>,
}

#[derive(Debug)]
struct Batch {
    requester: u32,
    template_id: i32,
    /// Copies the GM asked for (the command's `count`).
    requested: u32,
    /// Copies sent to the base whose result has not arrived yet.
    outstanding: u32,
    /// Entity ids of the copies that spawned, in arrival order.
    ids: Vec<u32>,
    failed: u32,
    /// Distinct failure reasons, in first-seen order.
    reasons: Vec<String>,
    deadline: Instant,
    lab: Option<LabWaiter>,
}

impl Batch {
    fn fail(&mut self, n: u32, reason: &str) {
        self.failed += n;
        if !self.reasons.iter().any(|r| r == reason) {
            self.reasons.push(reason.to_string());
        }
    }

    fn summary(&self) -> String {
        let ids = if self.ids.is_empty() {
            "none".to_string()
        } else {
            self.ids
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(",")
        };
        let mut s = format!(
            "spawnrandom: spawned {}/{} x template {}: ids {ids}",
            self.ids.len(),
            self.requested,
            self.template_id
        );
        if self.failed > 0 {
            s.push_str(&format!(
                "; {} failed ({})",
                self.failed,
                self.reasons.join("; ")
            ));
        }
        if self.outstanding > 0 {
            s.push_str(&format!("; {} timed out", self.outstanding));
        }
        s
    }

    fn close(self) -> Completed {
        Completed {
            requester: self.requester,
            summary: self.summary(),
            lab: self.lab,
        }
    }
}

/// A closed batch, ready to report: [`deliver`] sends it.
#[derive(Debug)]
pub struct Completed {
    pub requester: u32,
    pub summary: String,
    pub lab: Option<LabWaiter>,
}

/// What recording one copy's result did.
#[derive(Debug)]
pub enum Recorded {
    /// Other copies are still outstanding.
    Pending,
    /// That was the last copy; the batch closed.
    Completed(Completed),
    /// No open batch has this id: it already closed (timed out) or never
    /// existed. The caller reports the copy on its own.
    UnknownBatch,
}

/// The open `.spawnrandom` batches, stored in [`SpaceManager::resources`].
#[derive(Debug, Default)]
pub struct SpawnBatches {
    next_id: u32,
    open: HashMap<u32, Batch>,
}

impl SpawnBatches {
    /// Open a batch for `requested` copies, all of them outstanding until
    /// [`Self::undelivered`] says otherwise. Returns its id.
    pub fn open(&mut self, requester: u32, template_id: i32, requested: u32, now: Instant) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.open.insert(
            id,
            Batch {
                requester,
                template_id,
                requested,
                outstanding: requested,
                ids: Vec::new(),
                failed: 0,
                reasons: Vec::new(),
                deadline: now + SPAWN_BATCH_TIMEOUT,
                lab: None,
            },
        );
        id
    }

    /// The id the next [`Self::open`] will hand out.
    pub fn next_id(&self) -> u32 {
        self.next_id
    }

    /// Discard a batch none of whose requests reached the base.
    pub fn discard(&mut self, id: u32) {
        self.open.remove(&id);
    }

    /// `n` copies never reached the base: count them failed now rather than
    /// let them time out.
    pub fn undelivered(&mut self, id: u32, n: u32, reason: &str) {
        if let Some(b) = self.open.get_mut(&id) {
            let n = n.min(b.outstanding);
            b.outstanding -= n;
            b.fail(n, reason);
        }
    }

    /// One copy spawned as entity `npc`.
    pub fn spawned(&mut self, id: u32, npc: u32) -> Recorded {
        self.record(id, |b| b.ids.push(npc))
    }

    /// One copy failed for `reason`.
    pub fn failed(&mut self, id: u32, reason: &str) -> Recorded {
        self.record(id, |b| b.fail(1, reason))
    }

    fn record(&mut self, id: u32, apply: impl FnOnce(&mut Batch)) -> Recorded {
        let Some(b) = self.open.get_mut(&id) else {
            return Recorded::UnknownBatch;
        };
        // A result beyond the copies sent can only be a duplicate; it must
        // not underflow the count or close the batch early.
        if b.outstanding == 0 {
            return Recorded::UnknownBatch;
        }
        b.outstanding -= 1;
        apply(b);
        if b.outstanding > 0 {
            return Recorded::Pending;
        }
        match self.open.remove(&id) {
            Some(b) => Recorded::Completed(b.close()),
            None => Recorded::UnknownBatch,
        }
    }

    /// Close every batch whose deadline has passed; the copies still
    /// outstanding are reported as timed out.
    pub fn expire(&mut self, now: Instant) -> Vec<Completed> {
        let due: Vec<u32> = self
            .open
            .iter()
            .filter(|(_, b)| b.deadline <= now)
            .map(|(&id, _)| id)
            .collect();
        due.into_iter()
            .filter_map(|id| self.open.remove(&id))
            .map(Batch::close)
            .collect()
    }

    /// Park `waiter` on the batch `requester` opened at or after `mark` (a
    /// [`Self::next_id`] taken before the command ran). Hands the waiter
    /// back when the command opened none.
    pub fn attach_lab(
        &mut self,
        requester: u32,
        mark: u32,
        waiter: LabWaiter,
    ) -> Result<(), LabWaiter> {
        let opened = self.next_id.wrapping_sub(mark);
        let batch = self.open.iter_mut().find(|(&id, b)| {
            id.wrapping_sub(mark) < opened && b.requester == requester && b.lab.is_none()
        });
        match batch {
            Some((_, b)) => {
                b.lab = Some(waiter);
                Ok(())
            }
            None => Err(waiter),
        }
    }

    /// Batches still open (tests and diagnostics).
    pub fn open_count(&self) -> usize {
        self.open.len()
    }
}

/// The batch table, created on first use.
pub fn batches(space_mgr: &mut SpaceManager) -> &mut SpawnBatches {
    if !space_mgr.resources.contains::<SpawnBatches>() {
        space_mgr.resources.insert(SpawnBatches::default());
    }
    space_mgr
        .resources
        .get_mut::<SpawnBatches>()
        .expect("inserted above")
}

/// Send a closed batch's summary: into the parked lab reply when the lab
/// console ran the command, else to the GM as a feedback line.
pub async fn deliver(
    completed: Completed,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    tracing::info!(
        entity_id = completed.requester,
        entity_name = space_mgr.entity_label(completed.requester),
        summary = %completed.summary,
        lab = completed.lab.is_some(),
        "spawnrandom: batch reported"
    );
    match completed.lab {
        Some(mut waiter) => {
            waiter.lines.push(completed.summary);
            let _ = waiter.reply_tx.send(Ok(waiter.lines));
        }
        None => send_gm_feedback(completed.requester, &completed.summary, tx).await,
    }
}

/// 1 Hz: report every batch whose [`SPAWN_BATCH_TIMEOUT`] has passed.
/// Returns at once when no batch is open.
pub async fn spawn_batch_tick(tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    let Some(table) = space_mgr.resources.get_mut::<SpawnBatches>() else {
        return;
    };
    if table.open.is_empty() {
        return;
    }
    let done = table.expire(Instant::now());
    for completed in done {
        deliver(completed, tx, space_mgr).await;
    }
}
