//! Lock-free MPMC event queue.
//!
//! Producers are hook handlers running on whatever game thread caught
//! the event (CME EventSignal dispatch, an inline-hooked Tick, etc.).
//! The consumer is a dedicated uploader thread spawned by
//! `bootstrap_main`.
//!
//! # Why a bounded ring with drop-on-full
//!
//! Hook handlers run inside SGW.exe's hot paths — `FEngineLoop::Tick`
//! is on the render thread, `Mercury::Nub::handleMessage` is on the
//! network thread. They MUST NOT block. The classic options:
//!
//! - **Unbounded mpsc**: producers never block, but the queue can
//!   grow without bound under upload back-pressure (network blip,
//!   server down). Eventually OOMs the host process.
//! - **Bounded channel + `try_send`**: producer drops on full. Fine
//!   semantically but you lose track of how many you dropped.
//! - **Bounded ring + atomic drop counter**: producer drops on full
//!   AND increments a counter the uploader includes in periodic
//!   "queue.health" events. This is the shape we use.
//!
//! `crossbeam_channel::bounded` is the sync (non-async) MPMC channel.
//! `try_send` returns immediately with the value back on full.
//!
//! # Sequence number
//!
//! Allocated at enqueue from a single `AtomicU64`. Strictly
//! increasing across all threads. The server side will eventually
//! sort by `seq` to recover producer-order; until then it's mainly
//! useful for "we dropped events with seq 1003..1042" reconstruction
//! when the drop counter advances.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};

use crate::events::{ClientNativeEvent, EventBuilder};

/// Capacity of the event ring. Chosen for one second of frame-tick
/// telemetry at 60 Hz plus headroom for bursty network events; the
/// uploader's 2 s flush cadence should drain it well before it
/// fills under any realistic load. Hits cap → drop with counter.
///
/// 4096 slots × ~256 bytes/event ≈ 1 MiB resident. Cheap.
pub const RING_CAPACITY: usize = 4096;

/// Producer handle. Cloneable across threads — each hook handler
/// holds a clone.
#[derive(Clone)]
pub struct Producer {
    sender: Sender<ClientNativeEvent>,
    seq: std::sync::Arc<AtomicU64>,
    dropped: std::sync::Arc<AtomicU64>,
}

/// Consumer handle. Owned by the single uploader thread.
pub struct Consumer {
    receiver: Receiver<ClientNativeEvent>,
    dropped: std::sync::Arc<AtomicU64>,
}

/// Construct a fresh (producer, consumer) pair. Capacity is
/// [`RING_CAPACITY`]; the underlying shared state (seq counter,
/// drop counter) is allocated here and shared via Arc with every
/// producer clone.
pub fn channel() -> (Producer, Consumer) {
    let (sender, receiver) = bounded::<ClientNativeEvent>(RING_CAPACITY);
    let seq = std::sync::Arc::new(AtomicU64::new(0));
    let dropped = std::sync::Arc::new(AtomicU64::new(0));
    (
        Producer {
            sender,
            seq,
            dropped: dropped.clone(),
        },
        Consumer { receiver, dropped },
    )
}

impl Producer {
    /// Stamp an `EventBuilder` with `ts_ms` + `seq` and try to push
    /// it. On full, increments the drop counter and returns
    /// `false` — never blocks. Returns `true` when the event made
    /// it into the ring.
    pub fn try_emit(&self, builder: EventBuilder) -> bool {
        let event = ClientNativeEvent {
            ts_ms: now_ms(),
            seq: self.seq.fetch_add(1, Ordering::Relaxed),
            target: builder.target,
            level: builder.level,
            fields: builder.fields,
        };
        match self.sender.try_send(event) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                false
            }
            Err(TrySendError::Disconnected(_)) => {
                // Uploader thread crashed / quit. From the
                // producer's perspective the event is dropped; we
                // count it so the next health emit shows it.
                self.dropped.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
    }

    /// Read the running drop counter without resetting it. Useful
    /// for the uploader's periodic "queue.health" emit.
    pub fn dropped_total(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Read the running sequence counter (next id to allocate).
    pub fn next_seq(&self) -> u64 {
        self.seq.load(Ordering::Relaxed)
    }
}

impl Consumer {
    /// Block-with-timeout receive. Returns `Some(event)` on a
    /// successful pop, `None` on timeout. The uploader uses this
    /// to wake up periodically (flush cadence) even when the ring
    /// is empty so it can ship the dropped-counter snapshot.
    pub fn recv_timeout(&self, timeout: std::time::Duration) -> Option<ClientNativeEvent> {
        self.receiver.recv_timeout(timeout).ok()
    }

    /// Non-blocking pop. Returns `None` immediately if the ring is
    /// empty. Used in the uploader's drain loop after a recv.
    pub fn try_recv(&self) -> Option<ClientNativeEvent> {
        self.receiver.try_recv().ok()
    }

    /// Crate-internal raw accessor — exposes the underlying
    /// channel receiver so the uploader thread can call
    /// `recv_timeout` and distinguish Timeout from Disconnected
    /// (the public `recv_timeout` collapses both to `None`).
    pub(crate) fn raw(&self) -> &Receiver<ClientNativeEvent> {
        &self.receiver
    }

    /// Read the running drop counter without resetting it.
    pub fn dropped_total(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

/// Milliseconds since the Unix epoch. Wraps `SystemTime` so the
/// producer's emit path doesn't have to re-derive this every time.
/// Returns 0 if the system clock is before the epoch (impossible
/// in practice; defensive only).
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as O};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn channel_pair_starts_empty() {
        let (p, c) = channel();
        assert_eq!(p.next_seq(), 0);
        assert_eq!(p.dropped_total(), 0);
        assert!(c.try_recv().is_none());
    }

    #[test]
    fn emit_then_recv_roundtrips() {
        let (p, c) = channel();
        let b = ClientNativeEvent::builder("client.test", "info").field("k", serde_json::json!(1));
        assert!(p.try_emit(b));
        let got = c.try_recv().expect("event should be popped");
        assert_eq!(got.target, "client.test");
        assert_eq!(got.level, "info");
        assert_eq!(got.seq, 0);
        assert!(got.ts_ms > 0);
        assert_eq!(got.fields["k"], serde_json::json!(1));
    }

    #[test]
    fn seq_strictly_increases_across_emits() {
        let (p, c) = channel();
        for i in 0..5 {
            assert!(p.try_emit(
                ClientNativeEvent::builder("t", "info").field("i", serde_json::json!(i))
            ));
        }
        for i in 0..5 {
            let got = c.try_recv().unwrap();
            assert_eq!(got.seq, i as u64);
        }
    }

    /// **Producer non-blocking on full.** Fill the ring, then attempt
    /// to push one more — must return false and bump the drop
    /// counter. Reverting the `try_send` to a blocking `send` trips
    /// this test by hanging the thread.
    #[test]
    fn full_ring_drops_and_increments_counter() {
        let (p, _c) = channel();
        // Fill exactly to capacity.
        for _ in 0..RING_CAPACITY {
            assert!(p.try_emit(ClientNativeEvent::builder("t", "info")));
        }
        // Next emit drops.
        assert!(!p.try_emit(ClientNativeEvent::builder("t", "info")));
        assert_eq!(p.dropped_total(), 1);

        // Multiple subsequent drops accumulate.
        for _ in 0..10 {
            assert!(!p.try_emit(ClientNativeEvent::builder("t", "info")));
        }
        assert_eq!(p.dropped_total(), 11);
    }

    /// recv_timeout returns None on empty + elapsed timeout, doesn't
    /// hang. Pin: a regression that swaps to a blocking `recv` would
    /// hang this test until the runner kills it.
    #[test]
    fn recv_timeout_returns_none_when_empty() {
        let (_p, c) = channel();
        let start = std::time::Instant::now();
        assert!(c.recv_timeout(Duration::from_millis(50)).is_none());
        assert!(start.elapsed() >= Duration::from_millis(40));
        // Tolerate a few ms of jitter on either side.
        assert!(start.elapsed() < Duration::from_millis(500));
    }

    /// Producer clones see the same seq + drop counters. Hooks
    /// across multiple threads need this to share state.
    #[test]
    fn cloned_producers_share_counters() {
        let (p1, c) = channel();
        let p2 = p1.clone();
        p1.try_emit(ClientNativeEvent::builder("t", "info"));
        p2.try_emit(ClientNativeEvent::builder("t", "info"));
        assert_eq!(p1.next_seq(), 2);
        assert_eq!(p2.next_seq(), 2);
        // Drain so the consumer doesn't leak across tests.
        c.try_recv().unwrap();
        c.try_recv().unwrap();
    }

    /// Multi-threaded smoke: N producer threads each push K events.
    /// All N×K must come out, seq values are unique and contiguous.
    #[test]
    fn multi_thread_producers_yield_unique_seq() {
        const N: usize = 4;
        const K: usize = 100;
        let (p, c) = channel();
        let recv_count = std::sync::Arc::new(AtomicUsize::new(0));

        let mut handles = vec![];
        for _ in 0..N {
            let p2 = p.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..K {
                    p2.try_emit(ClientNativeEvent::builder("t", "info"));
                }
            }));
        }
        let drain = {
            let recv_count = recv_count.clone();
            thread::spawn(move || {
                let mut seen = std::collections::HashSet::new();
                while seen.len() < N * K {
                    if let Some(ev) = c.recv_timeout(Duration::from_millis(100)) {
                        seen.insert(ev.seq);
                        recv_count.fetch_add(1, O::SeqCst);
                    }
                }
                seen
            })
        };
        for h in handles {
            h.join().unwrap();
        }
        let seen = drain.join().unwrap();
        assert_eq!(seen.len(), N * K, "all events seen with unique seq");
        // seq values are 0..N*K (no holes, no duplicates).
        let min = *seen.iter().min().unwrap();
        let max = *seen.iter().max().unwrap();
        assert_eq!(min, 0);
        assert_eq!(max, (N * K) as u64 - 1);
    }
}
