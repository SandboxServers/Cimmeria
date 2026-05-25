//! Dev-session telemetry pipeline.

pub mod events;
pub mod queue;

use crate::config::exe_dir;
use events::TelemetryEvent;
use queue::DiskQueue;

/// Drain any telemetry events left on disk from a previous launcher
/// run that crashed or was killed before its bundle could upload.
/// Logs a summary so the dev can see what's about to be re-sent next
/// time the uploader runs.
///
/// Returns the count drained. Best-effort: a read error logs and
/// returns 0 — recovery is never load-bearing for game launch.
pub fn recover_pending_on_startup() -> u64 {
    let q = DiskQueue::new(&exe_dir());
    recover_pending_at(&q)
}

fn recover_pending_at(q: &DiskQueue) -> u64 {
    match q.drain::<TelemetryEvent>() {
        Ok(events) if events.is_empty() => 0,
        Ok(events) => {
            let n = events.len() as u64;
            tracing::info!(
                pending = n,
                dropped_since_last_drain = q.dropped_count(),
                "Recovered telemetry events from previous session — will replay on next flush"
            );
            // Re-enqueue so the next uploader run picks them up. A
            // write failure here is logged but not fatal; the events
            // are still in `events` on the stack and would be lost
            // on this drop — acceptable because telemetry is
            // supplementary.
            for ev in &events {
                if let Err(e) = q.enqueue(ev) {
                    tracing::warn!(error = %e, "failed to re-enqueue recovered event");
                    break;
                }
            }
            n
        }
        Err(e) => {
            tracing::warn!(error = %e, "telemetry recovery drain failed");
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::events::{ClientLogEvent, TelemetryEvent};

    #[test]
    fn recover_pending_returns_zero_on_empty_queue() {
        let dir = tempfile::tempdir().unwrap();
        let q = DiskQueue::new(dir.path());
        assert_eq!(recover_pending_at(&q), 0);
    }

    #[test]
    fn recover_pending_drains_and_reenqueues() {
        let dir = tempfile::tempdir().unwrap();
        let q = DiskQueue::new(dir.path());
        for i in 0..3 {
            q.enqueue(&TelemetryEvent::ClientLog(ClientLogEvent {
                ts_ms: i,
                seq: i as u64,
                source_file: "x.log".into(),
                level: "info".into(),
                category: "raw".into(),
                packet_no: None,
                message: format!("e-{i}"),
            }))
            .unwrap();
        }
        let n = recover_pending_at(&q);
        assert_eq!(n, 3);
        // After recovery the events are back on disk so the next
        // uploader run can flush them.
        let drained: Vec<TelemetryEvent> = q.drain().unwrap();
        assert_eq!(drained.len(), 3);
    }
}
