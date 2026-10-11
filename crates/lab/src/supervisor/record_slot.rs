//! One recording per lab instance (lab-record LR-06): the slot the record
//! tools (LR-07) reserve and arm, read for status and empty on stop, and
//! the supervisor's side of the sampler's [`ClientProbe`].
//!
//! The order LR-07 follows: [`RecordSlot::reserve`] first (refused while a
//! recording runs, so a refused start never touches the server), then spawn
//! the sampler task, then [`RecordReservation::arm`] with its handle.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use super::Supervisor;
use crate::uat::record::sampler::{BoxedJson, ClientProbe};

// The items below are dead until LR-07 wires the record tools in
// (cimmeria-lab is a binary); the tests use them all.

/// A running recording: where it writes, its counters, and how to stop it.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug)]
pub struct RecordHandle {
    /// `recording_id`: the start time and the instance.
    pub recording_id: String,
    /// The recording's directory (its log and screenshots).
    pub dir: PathBuf,
    /// The section being played, as the start line names it.
    pub section: String,
    /// Host clock at begin, ms since the epoch.
    pub started_ms: i64,
    /// Ticks written so far (the sampler task counts).
    pub ticks: Arc<AtomicU32>,
    /// Anchors written so far.
    pub anchors: Arc<AtomicU32>,
    /// Send `Some(reason)` to stop the sampler loop; it runs `finish`,
    /// which stops the tap.
    pub stop: tokio::sync::watch::Sender<Option<String>>,
    /// The sampler task; its result is the log's path.
    pub done: tokio::task::JoinHandle<Result<PathBuf, String>>,
}

/// What `lab_record_status` reports about the slot.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordStatus {
    /// The recording's id.
    pub recording_id: String,
    /// The section being played.
    pub section: String,
    /// Host clock at begin, ms since the epoch.
    pub started_ms: i64,
    /// Ticks written so far.
    pub ticks: u32,
    /// Anchors written so far.
    pub anchors: u32,
    /// False once the sampler task has ended (stopped by a cap or a fault)
    /// but nobody has collected it yet.
    pub running: bool,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Default)]
struct Slot {
    handle: Option<RecordHandle>,
    /// A start holds the slot between `reserve` and `arm`.
    reserved: bool,
}

/// This instance's recording, if any.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Default)]
pub struct RecordSlot(Mutex<Slot>);

/// The right to arm this instance's slot; dropping it unarmed frees the
/// slot for the next start.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug)]
pub struct RecordReservation {
    slot: Arc<RecordSlot>,
}

#[cfg_attr(not(test), allow(dead_code))]
impl RecordSlot {
    fn lock(&self) -> MutexGuard<'_, Slot> {
        // A panic while holding the lock leaves a valid slot behind.
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Hold the slot for one start, before anything is spawned. Refused
    /// while a recording runs here (the error names it) or another start
    /// holds it. A recording whose task has ended does not block: arming
    /// replaces it (its log stays on disk).
    pub fn reserve(self: &Arc<Self>) -> Result<RecordReservation, String> {
        let mut slot = self.lock();
        if let Some(why) = busy(&slot) {
            return Err(why);
        }
        if slot.reserved {
            return Err("another recording is starting on this instance".to_string());
        }
        slot.reserved = true;
        Ok(RecordReservation {
            slot: Arc::clone(self),
        })
    }

    /// Empty the slot (stop collects the handle and awaits its task).
    pub fn take(&self) -> Option<RecordHandle> {
        self.lock().handle.take()
    }

    /// The armed recording's id, section, counters and whether its task is
    /// still running; `None` when nothing is armed.
    pub fn status(&self) -> Option<RecordStatus> {
        self.lock().handle.as_ref().map(|h| RecordStatus {
            recording_id: h.recording_id.clone(),
            section: h.section.clone(),
            started_ms: h.started_ms,
            ticks: h.ticks.load(Ordering::Relaxed),
            anchors: h.anchors.load(Ordering::Relaxed),
            running: !h.done.is_finished(),
        })
    }
}

#[cfg_attr(not(test), allow(dead_code))]
impl RecordReservation {
    /// Arm `h`, the final check: if a recording is somehow running here,
    /// `h` is refused and told to stop (not aborted), so its `finish`
    /// stops the tap it started.
    pub fn arm(self, h: RecordHandle) -> Result<(), String> {
        let mut slot = self.slot.lock();
        if let Some(why) = busy(&slot) {
            let _ = h.stop.send(Some(format!("refused: {why}")));
            return Err(why);
        }
        slot.handle = Some(h);
        Ok(())
    }
}

impl Drop for RecordReservation {
    fn drop(&mut self) {
        self.slot.lock().reserved = false;
    }
}

/// Why the slot cannot take a new recording: one is still running.
#[cfg_attr(not(test), allow(dead_code))]
fn busy(slot: &Slot) -> Option<String> {
    slot.handle
        .as_ref()
        .filter(|c| !c.done.is_finished())
        .map(|cur| {
            format!(
                "recording {} ({}) is already running on this instance",
                cur.recording_id, cur.section
            )
        })
}

impl Supervisor {
    /// This instance's recording slot.
    // Unused until LR-07, in the test build too: no test here builds a
    // whole Supervisor.
    #[allow(dead_code)]
    pub fn record_slot(&self) -> &Arc<RecordSlot> {
        &self.record
    }
}

/// The sampler's client reads, straight to the supervisor: never through
/// the tool router, never the lease-gated tools (D-LR2).
impl ClientProbe for Supervisor {
    fn camera(&self) -> BoxedJson<'_> {
        Box::pin(self.camera_read())
    }

    fn ui(&self) -> BoxedJson<'_> {
        Box::pin(self.ui_state(Some(0)))
    }

    fn pump(&self) -> BoxedJson<'_> {
        Box::pin(async move { self.pump_events(true).await.map(|r| r.to_json()) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handle(id: &str, task: tokio::task::JoinHandle<Result<PathBuf, String>>) -> RecordHandle {
        RecordHandle {
            recording_id: id.to_string(),
            dir: PathBuf::from("records").join(id),
            section: "first-session".to_string(),
            started_ms: 1,
            ticks: Arc::new(AtomicU32::new(3)),
            anchors: Arc::new(AtomicU32::new(1)),
            stop: tokio::sync::watch::channel(None).0,
            done: task,
        }
    }

    fn forever() -> tokio::task::JoinHandle<Result<PathBuf, String>> {
        tokio::spawn(async {
            std::future::pending::<()>().await;
            Ok(PathBuf::new())
        })
    }

    /// LR-07's start, in its order: reserve, then spawn the sampler (here,
    /// count the tap start it would issue), then arm.
    fn start(slot: &Arc<RecordSlot>, id: &str, tap_starts: &AtomicU32) -> Result<(), String> {
        let r = slot.reserve()?;
        tap_starts.fetch_add(1, Ordering::SeqCst);
        r.arm(handle(id, forever()))
    }

    #[tokio::test]
    async fn one_recording_per_instance() {
        let slot = Arc::new(RecordSlot::default());
        let tap_starts = AtomicU32::new(0);
        assert!(slot.status().is_none());
        start(&slot, "a", &tap_starts).unwrap();

        let err = start(&slot, "b", &tap_starts).unwrap_err();
        assert!(err.contains("recording a (first-session)"), "{err}");
        assert_eq!(
            tap_starts.load(Ordering::SeqCst),
            1,
            "a refused start spawns nothing, so no tap starts"
        );
        let st = slot.status().unwrap();
        assert_eq!(st.recording_id, "a");
        assert_eq!((st.ticks, st.anchors, st.running), (3, 1, true));

        let a = slot.take().unwrap();
        assert_eq!(a.dir, PathBuf::from("records").join("a"));
        assert!(slot.status().is_none());
        a.done.abort();
    }

    #[tokio::test]
    async fn one_start_holds_the_slot_until_it_arms_or_drops() {
        let slot = Arc::new(RecordSlot::default());
        let r = slot.reserve().unwrap();
        let err = slot.reserve().unwrap_err();
        assert!(err.contains("starting"), "{err}");
        drop(r);
        // An unarmed reservation frees the slot.
        let r = slot.reserve().unwrap();
        r.arm(handle("a", forever())).unwrap();
        assert!(slot.reserve().is_err());
        slot.take().unwrap().done.abort();
        assert!(slot.reserve().is_ok());
    }

    #[tokio::test]
    async fn a_refused_arm_stops_the_task_instead_of_aborting_it() {
        let slot = Arc::new(RecordSlot::default());
        slot.reserve().unwrap().arm(handle("a", forever())).unwrap();
        // A reservation that got past `reserve` some other way: arm is the
        // final check.
        slot.lock().reserved = true;
        let r = RecordReservation {
            slot: Arc::clone(&slot),
        };
        let b = handle("b", forever());
        let mut stop_rx = b.stop.subscribe();
        let b_task = b.done.abort_handle();
        let err = r.arm(b).unwrap_err();
        assert!(err.contains("recording a"), "{err}");
        // The refused handle (the sender) is dropped by now; the value it
        // sent stays readable, as the sampler loop would read it.
        let reason = stop_rx.borrow_and_update().clone().expect("stop sent");
        assert!(reason.starts_with("refused: recording a"), "{reason}");
        tokio::task::yield_now().await;
        assert!(
            !b_task.is_finished(),
            "not aborted: its finish stops the tap"
        );
        b_task.abort();
        slot.take().unwrap().done.abort();
    }

    #[tokio::test]
    async fn an_ended_recording_is_replaced() {
        let slot = Arc::new(RecordSlot::default());
        let ended = tokio::spawn(async { Err::<PathBuf, _>("lease_lost".to_string()) });
        while !ended.is_finished() {
            tokio::task::yield_now().await;
        }
        slot.reserve().unwrap().arm(handle("a", ended)).unwrap();
        assert!(!slot.status().unwrap().running);
        slot.reserve().unwrap().arm(handle("b", forever())).unwrap();
        assert_eq!(slot.status().unwrap().recording_id, "b");
        slot.take().unwrap().done.abort();
    }
}
