//! The shell's forward loop: receives the layer's records, hands them to the engine
//! `Forwarder` (which coalesces for 2 s or 64 events, spaces sends 7 s apart and sends
//! at once on `launch_finished`), and pumps it. Nothing here builds events or reads
//! consent; that stays behind `ForwardSource`.
use cimmeria_launcher_engine::game_telemetry::launcher_event::LauncherRecord;
use cimmeria_launcher_engine::game_telemetry::launcher_forwarder::{
    ForwardSource, Forwarder, Pumped,
};
use cimmeria_launcher_engine::game_telemetry::launcher_queue::FLUSH_TARGET;
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, TryRecvError};
use std::sync::{Condvar, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

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

/// How long the exit flush may hold up closing the launcher (D-LTL6).
pub(crate) const EXIT_FLUSH_WAIT: Duration = Duration::from_secs(3);

/// A counter of finished flushes, so a waiter sees a flush that finished between its
/// read of the counter and its wait.
pub(crate) struct FlushSignal {
    done: Mutex<u64>,
    moved: Condvar,
}

impl FlushSignal {
    pub(crate) const fn new() -> Self {
        Self {
            done: Mutex::new(0),
            moved: Condvar::new(),
        }
    }

    /// Records one finished flush and wakes the waiters.
    pub(crate) fn notify(&self) {
        let mut done = self.done.lock().unwrap_or_else(PoisonError::into_inner);
        *done += 1;
        self.moved.notify_all();
    }
}

/// Sends the flush sentinel on `tx` and waits up to `wait` for `done` to move on. False
/// on a full or closed channel or a timeout. The sentinel never bypasses consent: the
/// forwarder still checks target and epoch before it sends anything.
pub(crate) fn request_flush(
    tx: &SyncSender<LauncherRecord>,
    done: &FlushSignal,
    wait: Duration,
) -> bool {
    // Read before sending, so a flush that finishes at once is not missed.
    let before = *done.done.lock().unwrap_or_else(PoisonError::into_inner);
    let sentinel = LauncherRecord {
        ts_ms: 0,
        level: "info".into(),
        target: FLUSH_TARGET,
        fields: serde_json::Map::new(),
    };
    if tx.try_send(sentinel).is_err() {
        return false;
    }
    let guard = done.done.lock().unwrap_or_else(PoisonError::into_inner);
    let (_guard, result) = done
        .moved
        .wait_timeout_while(guard, wait, |now| *now == before)
        .unwrap_or_else(PoisonError::into_inner);
    !result.timed_out()
}

static FLUSH_TX: OnceLock<SyncSender<LauncherRecord>> = OnceLock::new();
static FLUSH_DONE: FlushSignal = FlushSignal::new();

/// Keeps a sender for the exit flush; a second call is ignored.
pub(crate) fn remember_sender(tx: SyncSender<LauncherRecord>) {
    let _ = FLUSH_TX.set(tx);
}

/// Asks the forward thread to send what is queued and waits at most `EXIT_FLUSH_WAIT`.
/// False when the telemetry was never initialised, the channel is full, or time ran out.
pub(crate) fn flush_on_exit() -> bool {
    FLUSH_TX
        .get()
        .is_some_and(|tx| request_flush(tx, &FLUSH_DONE, EXIT_FLUSH_WAIT))
}

/// Called by the forward thread's `on_flushed`.
pub(crate) fn notify_flushed() {
    FLUSH_DONE.notify();
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
