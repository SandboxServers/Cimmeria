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
//! ordinary feedback line, if the GM is still the same character
//! ([`Requester::still_here`]).
//!
//! A closed batch is remembered for [`CLOSED_BATCH_MEMORY`], so a copy that
//! answers after the report ([`Recorded::Late`]) can be handled: the cell
//! despawns it when a lab caller was waiting (the lab never learns its id, so
//! it would be an orphan), and tells an in-world GM on its own line.
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

/// How long a closed batch is remembered for its late copies. A copy later
/// than this is handled as an unknown batch (kept, logged, no chat line).
pub const CLOSED_BATCH_MEMORY: Duration = Duration::from_secs(60);

/// The failure reason for a copy whose request never reached the base.
pub const CHANNEL_ERROR: &str = "cell→base channel error";

/// A lab console reply parked on a batch: the lines the command captured
/// while it ran, and the reply to send once the summary is known.
#[derive(Debug)]
pub struct LabWaiter {
    pub lines: Vec<String>,
    pub reply_tx: oneshot::Sender<LabConsoleResult>,
}

/// Who opened a batch: the entity the GM played and the character behind
/// it. Entity ids are reused after a logout, so a line sent later goes out
/// only while the same character still holds the id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Requester {
    pub entity_id: u32,
    pub account_id: Option<u32>,
    pub player_id: Option<i32>,
}

impl Requester {
    /// The requester as `entity_id` is now.
    pub fn of(entity_id: u32, space_mgr: &SpaceManager) -> Self {
        let id = space_mgr.player_identity(entity_id);
        Self {
            entity_id,
            account_id: id.account_id,
            player_id: id.player_id,
        }
    }

    /// `true` while `entity_id` is in the world as the same character.
    pub fn still_here(&self, space_mgr: &SpaceManager) -> bool {
        space_mgr.get_entity(self.entity_id).is_some()
            && Self::of(self.entity_id, space_mgr) == *self
    }
}

#[derive(Debug)]
struct Batch {
    requester: Requester,
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
            summary: self.summary(),
            requester: self.requester,
            template_id: self.template_id,
            failed: self.failed,
            timed_out: self.outstanding,
            lab: self.lab,
        }
    }
}

/// A closed batch, ready to report: [`deliver`] sends it.
#[derive(Debug)]
pub struct Completed {
    pub requester: Requester,
    pub template_id: i32,
    pub summary: String,
    pub failed: u32,
    pub timed_out: u32,
    pub lab: Option<LabWaiter>,
}

/// A copy that answered after its batch reported.
#[derive(Debug, Clone, Copy)]
pub struct Late {
    pub requester: Requester,
    /// A lab caller was waiting on the batch: nobody will learn this copy's
    /// id, so the cell despawns it.
    pub lab: bool,
}

/// What recording one copy's result did.
#[derive(Debug)]
pub enum Recorded {
    /// Other copies are still outstanding.
    Pending,
    /// That was the last copy; the batch closed.
    Completed(Completed),
    /// The batch already reported (timed out) within the last
    /// [`CLOSED_BATCH_MEMORY`].
    Late(Late),
    /// No batch with this id is open or remembered.
    UnknownBatch,
}

#[derive(Debug)]
struct ClosedBatch {
    late: Late,
    closed_at: Instant,
}

/// The `.spawnrandom` batches, stored in [`SpaceManager::resources`]: the
/// open ones, and the recently closed ones for their late copies.
#[derive(Debug, Default)]
pub struct SpawnBatches {
    next_id: u32,
    open: HashMap<u32, Batch>,
    closed: HashMap<u32, ClosedBatch>,
}

impl SpawnBatches {
    /// Open a batch for `requested` copies, all of them outstanding until
    /// [`Self::undelivered`] says otherwise. Returns its id.
    pub fn open(
        &mut self,
        requester: Requester,
        template_id: i32,
        requested: u32,
        now: Instant,
    ) -> u32 {
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
    /// let them time out. Closes the batch when no copy is left outstanding.
    pub fn undelivered(
        &mut self,
        id: u32,
        n: u32,
        reason: &str,
        now: Instant,
    ) -> Option<Completed> {
        let b = self.open.get_mut(&id)?;
        let n = n.min(b.outstanding);
        b.outstanding -= n;
        b.fail(n, reason);
        if b.outstanding > 0 {
            return None;
        }
        self.close(id, now)
    }

    /// One copy spawned as entity `npc`.
    pub fn spawned(&mut self, id: u32, npc: u32, now: Instant) -> Recorded {
        self.record(id, now, |b| b.ids.push(npc))
    }

    /// One copy failed for `reason`.
    pub fn failed(&mut self, id: u32, reason: &str, now: Instant) -> Recorded {
        self.record(id, now, |b| b.fail(1, reason))
    }

    fn record(&mut self, id: u32, now: Instant, apply: impl FnOnce(&mut Batch)) -> Recorded {
        // An open batch always has a copy outstanding: the result that
        // takes the count to zero closes it, so a duplicate finds it closed.
        let Some(b) = self.open.get_mut(&id) else {
            return match self.closed.get(&id) {
                Some(c) => Recorded::Late(c.late),
                None => Recorded::UnknownBatch,
            };
        };
        b.outstanding -= 1;
        apply(b);
        if b.outstanding > 0 {
            return Recorded::Pending;
        }
        match self.close(id, now) {
            Some(done) => Recorded::Completed(done),
            None => Recorded::UnknownBatch,
        }
    }

    /// Move an open batch to the closed set and report it.
    fn close(&mut self, id: u32, now: Instant) -> Option<Completed> {
        let b = self.open.remove(&id)?;
        self.closed.insert(
            id,
            ClosedBatch {
                late: Late {
                    requester: b.requester,
                    lab: b.lab.is_some(),
                },
                closed_at: now,
            },
        );
        Some(b.close())
    }

    /// Close every batch whose deadline has passed; the copies still
    /// outstanding are reported as timed out. Also forgets closed batches
    /// older than [`CLOSED_BATCH_MEMORY`].
    pub fn expire(&mut self, now: Instant) -> Vec<Completed> {
        self.closed
            .retain(|_, c| now.duration_since(c.closed_at) < CLOSED_BATCH_MEMORY);
        let due: Vec<u32> = self
            .open
            .iter()
            .filter(|(_, b)| b.deadline <= now)
            .map(|(&id, _)| id)
            .collect();
        due.into_iter()
            .filter_map(|id| self.close(id, now))
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
            id.wrapping_sub(mark) < opened && b.requester.entity_id == requester && b.lab.is_none()
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

    /// Closed batches still remembered (tests and diagnostics).
    pub fn closed_count(&self) -> usize {
        self.closed.len()
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

/// Send `text` to the batch's GM, if the entity id still belongs to the
/// same character. Otherwise the line is logged and dropped: the GM logged
/// out, and the id may already be someone else's.
pub async fn send_to_requester(
    requester: Requester,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    if requester.still_here(space_mgr) {
        send_gm_feedback(requester.entity_id, text, tx).await;
        return;
    }
    tracing::info!(
        entity_id = requester.entity_id, // nt:id-only the GM left; the id may be someone else's now
        account_id = requester.account_id, // nt:id-only the GM left; no live session holds the name
        player_id = requester.player_id, // nt:id-only the GM left; no live session holds the name
        text,
        "spawnrandom: GM is no longer in the world; line dropped"
    );
}

/// Send a closed batch's summary: into the parked lab reply when the lab
/// console ran the command, else to the GM as a feedback line.
pub async fn deliver(
    completed: Completed,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let requester = completed.requester.entity_id;
    // A batch that lost copies is worth a look; a clean one is routine.
    if completed.failed > 0 || completed.timed_out > 0 {
        tracing::warn!(
            entity_id = requester,
            entity_name = space_mgr.entity_label(requester),
            template_id = completed.template_id,
            template_name = cimmeria_names::book().template(completed.template_id),
            failed = completed.failed,
            timed_out = completed.timed_out,
            summary = %completed.summary,
            lab = completed.lab.is_some(),
            "spawnrandom: batch reported with failed or timed-out copies"
        );
    } else {
        tracing::info!(
            entity_id = requester,
            entity_name = space_mgr.entity_label(requester),
            template_id = completed.template_id,
            template_name = cimmeria_names::book().template(completed.template_id),
            summary = %completed.summary,
            lab = completed.lab.is_some(),
            "spawnrandom: batch reported"
        );
    }
    match completed.lab {
        Some(mut waiter) => {
            waiter.lines.push(completed.summary);
            let _ = waiter.reply_tx.send(Ok(waiter.lines));
        }
        None => send_to_requester(completed.requester, &completed.summary, tx, space_mgr).await,
    }
}

/// 1 Hz: report every batch whose [`SPAWN_BATCH_TIMEOUT`] has passed, and
/// forget closed batches past [`CLOSED_BATCH_MEMORY`]. Returns at once when
/// there is neither.
pub async fn spawn_batch_tick(tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    let Some(table) = space_mgr.resources.get_mut::<SpawnBatches>() else {
        return;
    };
    if table.open.is_empty() && table.closed.is_empty() {
        return;
    }
    let done = table.expire(Instant::now());
    for completed in done {
        deliver(completed, tx, space_mgr).await;
    }
}
