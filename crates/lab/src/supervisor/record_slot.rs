//! One recording per lab instance (lab-record LR-06): the slot the record
//! tools (LR-07) arm, read for status and empty on stop, and the
//! supervisor's side of the sampler's [`ClientProbe`].

// Dead until LR-07 wires the record tools in (cimmeria-lab is a binary);
// the tests below do not read every field either.
#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use super::Supervisor;
use crate::uat::record::sampler::{BoxedJson, ClientProbe};

/// A running recording: where it writes, its counters, and how to stop it.
#[derive(Debug)]
pub struct RecordHandle {
    pub recording_id: String,
    pub dir: PathBuf,
    pub section: String,
    pub started_ms: i64,
    pub ticks: Arc<AtomicU32>,
    pub anchors: Arc<AtomicU32>,
    /// Send `Some(reason)` to stop the sampler loop.
    pub stop: tokio::sync::watch::Sender<Option<String>>,
    /// The sampler task; its result is the log's path.
    pub done: tokio::task::JoinHandle<Result<PathBuf, String>>,
}

/// What `lab_record_status` reports about the slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordStatus {
    pub recording_id: String,
    pub section: String,
    pub started_ms: i64,
    pub ticks: u32,
    pub anchors: u32,
    /// False once the sampler task has ended (stopped by a cap or a fault)
    /// but nobody has collected it yet.
    pub running: bool,
}

/// This instance's recording, if any.
#[derive(Debug, Default)]
pub struct RecordSlot(Mutex<Option<RecordHandle>>);

impl RecordSlot {
    fn lock(&self) -> MutexGuard<'_, Option<RecordHandle>> {
        // A panic while holding the lock leaves a valid Option behind.
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Arm `h`. While another recording is still running here, `h` is
    /// refused: its task is aborted and the error names the running one.
    /// A recording whose task has ended is replaced (its log stays on disk).
    pub fn arm(&self, h: RecordHandle) -> Result<(), String> {
        let mut slot = self.lock();
        if let Some(cur) = slot.as_ref().filter(|c| !c.done.is_finished()) {
            let why = format!(
                "recording {} ({}) is already running on this instance",
                cur.recording_id, cur.section
            );
            h.done.abort();
            return Err(why);
        }
        *slot = Some(h);
        Ok(())
    }

    /// Empty the slot (stop collects the handle and awaits its task).
    pub fn take(&self) -> Option<RecordHandle> {
        self.lock().take()
    }

    pub fn status(&self) -> Option<RecordStatus> {
        self.lock().as_ref().map(|h| RecordStatus {
            recording_id: h.recording_id.clone(),
            section: h.section.clone(),
            started_ms: h.started_ms,
            ticks: h.ticks.load(Ordering::Relaxed),
            anchors: h.anchors.load(Ordering::Relaxed),
            running: !h.done.is_finished(),
        })
    }
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

    #[tokio::test]
    async fn one_recording_per_instance() {
        let slot = RecordSlot::default();
        assert!(slot.status().is_none());
        slot.arm(handle("a", forever())).unwrap();

        let err = slot.arm(handle("b", forever())).unwrap_err();
        assert!(err.contains("recording a (first-session)"), "{err}");
        let st = slot.status().unwrap();
        assert_eq!(st.recording_id, "a");
        assert_eq!((st.ticks, st.anchors, st.running), (3, 1, true));

        let a = slot.take().unwrap();
        assert!(slot.status().is_none());
        a.done.abort();
    }

    #[tokio::test]
    async fn an_ended_recording_is_replaced() {
        let slot = RecordSlot::default();
        let ended = tokio::spawn(async { Err::<PathBuf, _>("lease_lost".to_string()) });
        while !ended.is_finished() {
            tokio::task::yield_now().await;
        }
        slot.arm(handle("a", ended)).unwrap();
        assert!(!slot.status().unwrap().running);
        slot.arm(handle("b", forever())).unwrap();
        assert_eq!(slot.status().unwrap().recording_id, "b");
        slot.take().unwrap().done.abort();
    }
}
