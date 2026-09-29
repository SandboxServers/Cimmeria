//! Urgent-flush handshake between an event producer that is about to
//! lose the process (a crash filter, an exit hook) and the uploader
//! thread.
//!
//! The uploader normally ships on its flush cadence (2 s by default). A
//! crash filter cannot wait for the cadence and cannot do the POST
//! itself: it runs on a thread that just faulted, so a TLS handshake
//! there is the last thing to try. Instead it queues its event, asks
//! for a flush up to that event's `seq` with [`FlushSignal::request`],
//! and polls [`wait_delivered`] for a bounded time while the healthy
//! uploader thread does the network work.
//!
//! Everything here is atomics: no lock a faulting thread could be
//! holding.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Shared between the uploader thread and anything that needs its
/// events shipped now. `0` in each counter means "nothing yet"; the
/// counters hold `seq + 1` so seq 0 is representable.
#[derive(Debug, Default)]
pub struct FlushSignal {
    /// Highest `seq + 1` a producer asked to have shipped now.
    requested_through: AtomicU64,
    /// Highest `seq + 1` a successful POST has carried.
    delivered_through: AtomicU64,
    /// OS thread id of the uploader, so a crash *on* the uploader
    /// thread does not wait for itself. 0 = unknown.
    uploader_thread: AtomicU32,
}

impl FlushSignal {
    pub const fn new() -> Self {
        Self {
            requested_through: AtomicU64::new(0),
            delivered_through: AtomicU64::new(0),
            uploader_thread: AtomicU32::new(0),
        }
    }

    /// Ask the uploader to ship everything up to and including `seq`
    /// without waiting for its cadence.
    pub fn request(&self, seq: u64) {
        self.requested_through.fetch_max(seq + 1, Ordering::AcqRel);
    }

    /// Whether a requested flush has not been delivered yet. The
    /// uploader checks this each loop.
    pub fn pending(&self) -> bool {
        self.requested_through.load(Ordering::Acquire)
            > self.delivered_through.load(Ordering::Acquire)
    }

    /// Record a successful POST that carried events up to `max_seq`.
    pub fn mark_delivered(&self, max_seq: u64) {
        self.delivered_through
            .fetch_max(max_seq + 1, Ordering::AcqRel);
    }

    /// Whether the event with `seq` has been shipped.
    pub fn is_delivered(&self, seq: u64) -> bool {
        self.delivered_through.load(Ordering::Acquire) > seq
    }

    pub fn set_uploader_thread(&self, os_thread_id: u32) {
        self.uploader_thread.store(os_thread_id, Ordering::Release);
    }

    /// True when `os_thread_id` is the uploader itself: waiting there
    /// would only burn the whole timeout.
    pub fn is_uploader_thread(&self, os_thread_id: u32) -> bool {
        os_thread_id != 0 && self.uploader_thread.load(Ordering::Acquire) == os_thread_id
    }
}

/// Outcome of [`wait_delivered`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushWait {
    Delivered,
    TimedOut,
    /// Not attempted: the caller is the uploader thread.
    SkippedOnUploader,
}

impl FlushWait {
    pub fn as_str(self) -> &'static str {
        match self {
            FlushWait::Delivered => "delivered",
            FlushWait::TimedOut => "timed_out",
            FlushWait::SkippedOnUploader => "skipped_on_uploader",
        }
    }
}

/// Request a flush through `seq` and poll until the uploader reports it
/// delivered or `timeout` passes. `os_thread_id` is the caller's thread
/// (skips the wait on the uploader thread); `pause` is the sleep between
/// polls, injectable for tests.
pub fn wait_delivered(
    signal: &FlushSignal,
    seq: u64,
    os_thread_id: u32,
    timeout: Duration,
    mut pause: impl FnMut(),
) -> FlushWait {
    signal.request(seq);
    if signal.is_uploader_thread(os_thread_id) {
        return FlushWait::SkippedOnUploader;
    }
    let deadline = Instant::now() + timeout;
    loop {
        if signal.is_delivered(seq) {
            return FlushWait::Delivered;
        }
        if Instant::now() >= deadline {
            return FlushWait::TimedOut;
        }
        pause();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::ClientNativeEvent;
    use crate::queue::channel;
    use crate::uploader::{run_uploader_with, UploaderConfig};
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn fresh_signal_has_nothing_pending_or_delivered() {
        let s = FlushSignal::new();
        assert!(!s.pending());
        assert!(!s.is_delivered(0));
    }

    #[test]
    fn request_then_deliver_clears_pending() {
        let s = FlushSignal::new();
        s.request(4);
        assert!(s.pending());
        s.mark_delivered(3);
        assert!(s.pending(), "seq 4 is not covered by a batch ending at 3");
        s.mark_delivered(4);
        assert!(!s.pending());
        assert!(s.is_delivered(4));
        assert!(!s.is_delivered(5));
    }

    /// A late, smaller delivery (a retried older batch) must not move
    /// the delivered mark backwards.
    #[test]
    fn delivered_mark_never_goes_backwards() {
        let s = FlushSignal::new();
        s.mark_delivered(10);
        s.mark_delivered(2);
        assert!(s.is_delivered(10));
    }

    #[test]
    fn wait_returns_delivered_once_marked() {
        let s = FlushSignal::new();
        let mut polls = 0;
        let got = wait_delivered(&s, 7, 1, Duration::from_secs(5), || {
            polls += 1;
            if polls == 3 {
                s.mark_delivered(7);
            }
        });
        assert_eq!(got, FlushWait::Delivered);
        assert_eq!(polls, 3);
    }

    #[test]
    fn wait_times_out_when_nothing_ships() {
        let s = FlushSignal::new();
        let start = Instant::now();
        let got = wait_delivered(&s, 0, 1, Duration::from_millis(60), || {
            thread::sleep(Duration::from_millis(5))
        });
        assert_eq!(got, FlushWait::TimedOut);
        assert!(start.elapsed() >= Duration::from_millis(60));
        assert!(s.pending(), "the request stays armed for the uploader");
    }

    /// A crash on the uploader thread must not wait for itself.
    #[test]
    fn wait_is_skipped_on_the_uploader_thread() {
        let s = FlushSignal::new();
        s.set_uploader_thread(42);
        let got = wait_delivered(&s, 0, 42, Duration::from_secs(5), || {
            panic!("must not poll on the uploader thread")
        });
        assert_eq!(got, FlushWait::SkippedOnUploader);
        assert!(s.pending(), "the request is still recorded");
    }

    #[test]
    fn unknown_thread_id_is_never_the_uploader() {
        let s = FlushSignal::new();
        assert!(!s.is_uploader_thread(0));
    }

    /// **The uploader ships a requested flush without waiting for its
    /// cadence.** The flush interval is 30 s; the request must get the
    /// event to the server well inside the crash filter's time box.
    /// Reverting the `pending()` checks in the uploader loop, or the
    /// `URGENT_RETRY` cap on its wait while it holds a batch, makes this
    /// time out.
    #[test]
    fn uploader_ships_a_requested_flush_before_its_cadence() {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let addr = server.server_addr().to_ip().unwrap();
        let url = format!("http://{}:{}/upload-chunk", addr.ip(), addr.port());
        let server = Arc::new(server);
        let srv = server.clone();
        let server_thread = thread::spawn(move || {
            while let Ok(Some(req)) = srv.recv_timeout(Duration::from_secs(5)) {
                req.respond(tiny_http::Response::from_string("ok")).ok();
            }
        });

        let signal: &'static FlushSignal = Box::leak(Box::new(FlushSignal::new()));
        let (p, c) = channel();
        let uploader = thread::spawn(move || {
            let cfg = UploaderConfig {
                upload_endpoint: url,
                token: "t".into(),
                flush_interval: Duration::from_secs(30),
                max_batch: 100,
            };
            run_uploader_with(c, cfg, || false, signal)
        });

        // Let the uploader block in its (30 s) receive first.
        thread::sleep(Duration::from_millis(100));
        let seq = p
            .try_emit_seq(ClientNativeEvent::builder("client.crash", "error"))
            .expect("queued");
        // The crash path requests the flush only after queueing: let the
        // event wake the uploader and put it back to sleep first, so the
        // request lands while it waits with a non-empty batch.
        thread::sleep(Duration::from_millis(200));
        let start = Instant::now();
        let got = wait_delivered(signal, seq, 1, Duration::from_secs(3), || {
            thread::sleep(Duration::from_millis(10))
        });
        assert_eq!(got, FlushWait::Delivered);
        assert!(start.elapsed() < Duration::from_secs(3));

        drop(p);
        uploader.join().unwrap();
        server.unblock();
        server_thread.join().unwrap();
    }
}
