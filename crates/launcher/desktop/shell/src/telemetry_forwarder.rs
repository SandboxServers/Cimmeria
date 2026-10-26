//! The shell's forward loop: receives the layer's records, hands them to the engine
//! `Forwarder` (which coalesces for 2 s or 64 events, spaces sends 7 s apart and sends
//! at once on `launch_finished`), and pumps it. Nothing here builds events or reads
//! consent; that stays behind `ForwardSource`.
use cimmeria_launcher_engine::game_telemetry::launcher_event::LauncherRecord;
use cimmeria_launcher_engine::game_telemetry::launcher_forwarder::{
    ForwardSource, Forwarder, Pumped,
};
use cimmeria_launcher_engine::game_telemetry::launcher_queue::FLUSH_TARGET;
use std::sync::mpsc::{Receiver, RecvTimeoutError, TryRecvError};
use std::time::Instant;

/// Records taken from the channel per round, so one burst is one `on_records` call.
const RECORD_BATCH: usize = 64;

/// Receives, coalesces and pumps until `records` disconnects, then pumps once more as a
/// flush. `on_flushed` runs whenever the forwarder says a flush finished.
pub(crate) fn forward_loop<S: ForwardSource>(
    records: &Receiver<LauncherRecord>,
    forwarder: &mut Forwarder,
    source: &S,
    on_pumped: &mut dyn FnMut(&Pumped),
    on_flushed: &mut dyn FnMut(),
) {
    loop {
        // Sleep exactly until the forwarder could next send; with nothing queued, until a record.
        let first = match forwarder.next_wake(Instant::now()) {
            None => records.recv().map_err(|_| RecvTimeoutError::Disconnected),
            Some(wait) => records.recv_timeout(wait),
        };
        let mut batch = Vec::new();
        match first {
            Ok(record) => {
                batch.push(record);
                while batch.len() < RECORD_BATCH {
                    match records.try_recv() {
                        Ok(record) => batch.push(record),
                        // A disconnect is seen again by the next `recv`, after the buffer is empty.
                        Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
                    }
                }
            }
            // The wake time came: nothing arrived, but a send may be due.
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                // Last round: the sentinel makes the forwarder send whatever is queued now.
                let sentinel = LauncherRecord {
                    ts_ms: 0,
                    level: "info".into(),
                    target: FLUSH_TARGET,
                    fields: serde_json::Map::new(),
                };
                forwarder.on_records(source, vec![sentinel], Instant::now());
                pump_once(forwarder, source, on_pumped, on_flushed);
                return;
            }
        }
        if !batch.is_empty() {
            forwarder.on_records(source, batch, Instant::now());
        }
        pump_once(forwarder, source, on_pumped, on_flushed);
    }
}

fn pump_once<S: ForwardSource>(
    forwarder: &mut Forwarder,
    source: &S,
    on_pumped: &mut dyn FnMut(&Pumped),
    on_flushed: &mut dyn FnMut(),
) {
    if let Some(pumped) = tauri::async_runtime::block_on(forwarder.pump(source, Instant::now())) {
        on_pumped(&pumped);
    }
    if forwarder.take_flush_done() {
        on_flushed();
    }
}

#[cfg(test)]
#[path = "telemetry_forwarder_tests.rs"]
mod tests;
