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

use serde_json::{json, Value};

use super::events::KIND_CLICK;
use super::world::memory::PlayerCamera;
use super::Supervisor;
use crate::uat::record::sampler::{BoxedJson, ClientProbe};

/// The event-store cursor the recorder reads `ui.click` through (LR-11).
const RECORD_CURSOR: &str = "record";

/// A running recording: where it writes, its counters, and how to stop it.
#[derive(Debug)]
pub struct RecordHandle {
    /// `recording_id`: the start time and the instance.
    pub recording_id: String,
    /// The recording's directory (its log and screenshots).
    pub dir: PathBuf,
    /// The section being played, as the start line names it.
    pub section: String,
    /// Where stop writes the draft (`<specs_dir>/drafts/`), when start was
    /// given one.
    pub specs_dir: Option<PathBuf>,
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
    /// `done`'s result once a stop has awaited it. A `JoinHandle` gives its
    /// result once, and a recording whose stop failed goes back into the
    /// slot for a retry, so the result is kept here.
    pub ended: Option<Result<PathBuf, String>>,
}

impl RecordHandle {
    /// The task's result: awaited the first time, the kept copy after.
    pub async fn outcome(&mut self) -> Result<PathBuf, String> {
        if self.ended.is_none() {
            let r = match (&mut self.done).await {
                Ok(r) => r,
                Err(e) => Err(format!("the recorder task failed: {e}")),
            };
            self.ended = Some(r);
        }
        self.ended.clone().expect("set above")
    }
}

/// What `lab_record_status` reports about the slot.
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

#[derive(Debug, Default)]
struct Slot {
    handle: Option<RecordHandle>,
    /// A start holds the slot between `reserve` and `arm`.
    reserved: bool,
    /// The player controller and camera `camera_read` found last, so a
    /// tick re-checks two vtables instead of rescanning every actor. Dropped
    /// whenever a read fails.
    camera: Option<PlayerCamera>,
}

/// This instance's recording, if any.
#[derive(Debug, Default)]
pub struct RecordSlot(Mutex<Slot>);

/// The right to arm this instance's slot; dropping it unarmed frees the
/// slot for the next start.
#[derive(Debug)]
pub struct RecordReservation {
    slot: Arc<RecordSlot>,
}

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

    /// Empty the slot whatever it holds (the tests; the stop uses
    /// [`Self::take_ended`]).
    #[cfg(test)]
    pub fn take(&self) -> Option<RecordHandle> {
        self.lock().handle.take()
    }

    /// The stop's first step: send `reason` to the recording
    /// `recording_id`, which stays in the slot (so a start is still refused
    /// while it finishes). Returns its folder. Any other id is refused and
    /// the recording keeps running; the refusal names the instance, never
    /// the live id. `instance` is this slot's label, for the refusals.
    pub fn signal_stop(
        &self,
        recording_id: &str,
        instance: &str,
        reason: &str,
    ) -> Result<PathBuf, String> {
        let slot = self.lock();
        let h = matching(&slot, recording_id, instance)?;
        // Nobody listens once the task has ended by itself: that is fine.
        let _ = h.stop.send(Some(reason.to_string()));
        Ok(h.dir.clone())
    }

    /// Whether the recording `recording_id`'s task has ended; `None` once
    /// the slot no longer holds it.
    pub fn ended(&self, recording_id: &str) -> Option<bool> {
        self.lock()
            .handle
            .as_ref()
            .filter(|h| h.recording_id == recording_id)
            .map(|h| h.done.is_finished())
    }

    /// Empty the slot once the recording `recording_id` has ended: the
    /// stop's gate, refused like [`Self::signal_stop`]. A failed stop puts
    /// it back with [`Self::restore`].
    pub fn take_ended(&self, recording_id: &str, instance: &str) -> Result<RecordHandle, String> {
        let mut slot = self.lock();
        if !matching(&slot, recording_id, instance)?.done.is_finished() {
            return Err(format!(
                "the recording on instance {instance} is still finishing"
            ));
        }
        Ok(slot.handle.take().expect("matched above"))
    }

    /// Put back a recording whose stop failed before its draft was written,
    /// so a second stop (perhaps with other options) can retry. A recording
    /// armed meanwhile wins and `h` is dropped; its raw log stays on disk.
    pub fn restore(&self, h: RecordHandle) -> bool {
        let mut slot = self.lock();
        if slot.handle.is_some() {
            return false;
        }
        slot.handle = Some(h);
        true
    }

    /// The camera pair the last good `camera_read` used.
    pub(super) fn cached_camera(&self) -> Option<PlayerCamera> {
        self.lock().camera
    }

    /// Keep `c` for the next `camera_read`; `None` after a failed read.
    pub(super) fn cache_camera(&self, c: Option<PlayerCamera>) {
        self.lock().camera = c;
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

/// The slot's recording when it is `recording_id`. The stop gate guards
/// against accidents (a stale id, the wrong instance), not against a caller:
/// `lab_record_status` hands the id to anyone who asks.
fn matching<'a>(
    slot: &'a Slot,
    recording_id: &str,
    instance: &str,
) -> Result<&'a RecordHandle, String> {
    match &slot.handle {
        Some(h) if h.recording_id == recording_id => Ok(h),
        Some(_) => Err(format!(
            "that recording_id is not the running recording on instance {instance}"
        )),
        None => Err(format!(
            "no recording on instance {instance}; pass `instance` to pick another"
        )),
    }
}

/// Why the slot cannot take a new recording: one is still running.
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

    fn clicks(&self) -> BoxedJson<'_> {
        Box::pin(async move {
            // A store read makes no bridge call, so the permit is checked
            // here, as `camera_read` does.
            crate::lease::permit::ensure("record clicks")?;
            let (events, gap) = self
                .events
                .inner
                .lock()
                .await
                .take_kind(RECORD_CURSOR, KIND_CLICK);
            let clicks: Vec<Value> = events.into_iter().map(|e| e.fields).collect();
            Ok(json!({ "clicks": clicks, "gap": gap }))
        })
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
            specs_dir: None,
            started_ms: 1,
            ticks: Arc::new(AtomicU32::new(3)),
            anchors: Arc::new(AtomicU32::new(1)),
            stop: tokio::sync::watch::channel(None).0,
            done: task,
            ended: None,
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

    /// LR-07's stop gate: a wrong id leaves the recording armed, and no
    /// refusal names the live id; a running task is not taken out.
    #[tokio::test]
    async fn only_the_ended_running_id_empties_the_slot() {
        let slot = Arc::new(RecordSlot::default());
        let err = slot.signal_stop("a", "p2", "stopped").unwrap_err();
        assert!(err.contains("no recording on instance p2"), "{err}");
        let (tx, mut rx) = tokio::sync::watch::channel(None);
        let mut h = handle("a-live", forever());
        h.stop = tx;
        slot.reserve().unwrap().arm(h).unwrap();
        let err = slot.signal_stop("b", "p2", "stopped").unwrap_err();
        assert!(err.contains("instance p2"), "{err}");
        assert!(!err.contains("a-live"), "the live id leaked: {err}");
        assert!(rx.borrow_and_update().is_none(), "a wrong id sent nothing");

        slot.signal_stop("a-live", "p2", "stopped").unwrap();
        assert_eq!(rx.borrow_and_update().as_deref(), Some("stopped"));
        assert_eq!(slot.ended("a-live"), Some(false));
        let err = slot.take_ended("a-live", "p2").unwrap_err();
        assert!(err.contains("still finishing"), "{err}");
        assert_eq!(slot.status().unwrap().recording_id, "a-live");
        slot.take().unwrap().done.abort();
        assert_eq!(slot.ended("a-live"), None);
    }

    /// A failed stop puts the recording back, and its result survives the
    /// first await; a recording armed meanwhile is not displaced.
    #[tokio::test]
    async fn a_restored_recording_keeps_its_outcome() {
        let slot = Arc::new(RecordSlot::default());
        let ended = tokio::spawn(async { Err::<PathBuf, _>("write_failed".to_string()) });
        while !ended.is_finished() {
            tokio::task::yield_now().await;
        }
        slot.reserve().unwrap().arm(handle("a", ended)).unwrap();
        let mut h = slot.take_ended("a", "p2").unwrap();
        assert_eq!(h.outcome().await.unwrap_err(), "write_failed");
        assert!(slot.restore(h));
        let mut h = slot.take_ended("a", "p2").unwrap();
        assert_eq!(h.outcome().await.unwrap_err(), "write_failed", "kept");

        slot.reserve().unwrap().arm(handle("b", forever())).unwrap();
        assert!(!slot.restore(h), "the newer recording wins");
        assert_eq!(slot.status().unwrap().recording_id, "b");
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
