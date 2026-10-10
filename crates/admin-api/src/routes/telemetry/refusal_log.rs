//! One throttled `warn` per refused upload (negative-logging Pattern D).
//!
//! A refused uploader usually keeps trying: the launcher and the DLL retry
//! every flush, and an unauthenticated caller can repeat as fast as it
//! likes. So the first refusal for an `(uploader, reason)` pair writes a
//! row at once, the repeats inside [`WINDOW`] are counted, and the next row
//! for the pair carries the count in `suppressed`. On top of that, at most
//! [`GLOBAL_ROWS_PER_WINDOW`] rows are written per window across every
//! uploader, so many distinct uploaders cannot turn refusals into a log
//! flood either; a row held back by that ceiling is counted in its pair's
//! `suppressed` like any other.
//!
//! The rows carry the route, the reason, the budget and limit that refused
//! the upload, and the uploader's identity (the token's ids once it
//! verified, and the peer address). Never the payload or the parser's error
//! text.
//!
//! The table has a fixed number of slots keyed by a hash of the pair. Two
//! pairs that land on one slot take it in turn, and each takeover writes a
//! row: a collision can cost log volume (bounded by the global ceiling) but
//! never swallows another uploader's first refusal.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use super::dto::IngestError;
use super::upload_gate::{Route, Uploader};

/// How long a pair's repeats are counted instead of written.
pub(super) const WINDOW: Duration = Duration::from_secs(10);

/// Rows written per [`WINDOW`] across every uploader.
pub(super) const GLOBAL_ROWS_PER_WINDOW: u32 = 50;

/// Slots in the per-pair table.
const SLOTS: usize = 512;

#[derive(Debug, Clone, Copy)]
struct Slot {
    key: u64,
    opened: Instant,
    suppressed: u64,
}

#[derive(Debug)]
struct Global {
    opened: Instant,
    rows: u32,
}

#[derive(Debug)]
struct Inner {
    slots: Box<[Option<Slot>]>,
    global: Option<Global>,
}

/// The throttle. One per [`super::upload_gate::UploadState`].
#[derive(Debug)]
pub(super) struct RefusalLog {
    inner: Mutex<Inner>,
}

/// What to do with one refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Decision {
    /// Write a row carrying this many earlier, unwritten refusals.
    Emit { suppressed: u64 },
    /// Count it; write nothing.
    Suppress,
}

impl RefusalLog {
    pub(super) fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                slots: vec![None; SLOTS].into_boxed_slice(),
                global: None,
            }),
        }
    }

    /// Decide for one refusal of `reason` to `subject` at `now`.
    pub(super) fn decide(&self, reason: &'static str, subject: &str, now: Instant) -> Decision {
        let key = {
            let mut h = DefaultHasher::new();
            reason.hash(&mut h);
            subject.hash(&mut h);
            h.finish()
        };
        let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        // Borrow the fields apart: the global counter and a slot are
        // updated together below.
        let inner = &mut *guard;
        let idx = (key % SLOTS as u64) as usize;
        let carried = match inner.slots[idx] {
            Some(s) if s.key == key && now.saturating_duration_since(s.opened) < WINDOW => {
                inner.slots[idx] = Some(Slot {
                    suppressed: s.suppressed + 1,
                    ..s
                });
                return Decision::Suppress;
            }
            Some(s) if s.key == key => s.suppressed,
            _ => 0,
        };
        let global = inner.global.get_or_insert(Global {
            opened: now,
            rows: 0,
        });
        if now.saturating_duration_since(global.opened) >= WINDOW {
            *global = Global {
                opened: now,
                rows: 0,
            };
        }
        if global.rows >= GLOBAL_ROWS_PER_WINDOW {
            // Held back by the ceiling: the pair's next row reports it.
            inner.slots[idx] = Some(Slot {
                key,
                opened: now,
                suppressed: carried + 1,
            });
            return Decision::Suppress;
        }
        global.rows += 1;
        inner.slots[idx] = Some(Slot {
            key,
            opened: now,
            suppressed: 0,
        });
        Decision::Emit {
            suppressed: carried,
        }
    }

    /// Log `err`, a refusal of `route` to `who`, unless the throttle holds
    /// it back.
    pub(super) fn report(&self, route: Route, who: &Uploader, err: &IngestError, now: Instant) {
        let reason = err.reason();
        let peer = who.peer.to_string();
        let subject = who.session_id.as_deref().unwrap_or(&peer);
        let Decision::Emit { suppressed } = self.decide(reason, subject, now) else {
            return;
        };
        tracing::warn!(
            target: "launcher.ingest",
            route = route.path(),
            reason,
            budget = err.budget(),
            limit = err.limit(),
            session_id = who.session_id.as_deref(), // nt:id-only telemetry session UUID from the token; it names nothing
            install_id = who.install_id.as_deref(), // nt:id-only launcher install UUID from the token; it names nothing
            peer = %who.peer,
            suppressed,
            "telemetry upload refused: {reason}"
        );
    }

    /// Log a truncated upload (`reason` = `chunk_truncated` or
    /// `bundle_truncated`), unless the throttle holds it back. The upload
    /// was accepted up to `budget`; `kept` units were replayed and about
    /// `dropped_estimate` were not.
    pub(super) fn report_truncated(
        &self,
        route: Route,
        who: &Uploader,
        t: &Truncation,
        now: Instant,
    ) {
        let reason = match route {
            Route::Chunk => "chunk_truncated",
            Route::Bundle => "bundle_truncated",
        };
        let peer = who.peer.to_string();
        let subject = who.session_id.as_deref().unwrap_or(&peer);
        let Decision::Emit { suppressed } = self.decide(reason, subject, now) else {
            return;
        };
        tracing::warn!(
            target: "launcher.ingest",
            route = route.path(),
            reason,
            budget = t.budget,
            limit = t.limit,
            kept = t.kept,
            dropped_estimate = t.dropped_estimate,
            session_id = who.session_id.as_deref(), // nt:id-only telemetry session UUID from the token; it names nothing
            install_id = who.install_id.as_deref(), // nt:id-only launcher install UUID from the token; it names nothing
            peer = %who.peer,
            suppressed,
            "telemetry upload truncated at its {} budget",
            t.budget
        );
    }
}

impl RefusalLog {
    /// Log a chunk that was accepted with `bad` rows skipped because they
    /// did not parse (`reason = bad_rows`), unless the throttle holds it
    /// back. Never the rows themselves.
    pub(super) fn report_bad_rows(&self, who: &Uploader, bad: u64, now: Instant) {
        let reason = "bad_rows";
        let peer = who.peer.to_string();
        let subject = who.session_id.as_deref().unwrap_or(&peer);
        let Decision::Emit { suppressed } = self.decide(reason, subject, now) else {
            return;
        };
        tracing::warn!(
            target: "launcher.ingest",
            route = Route::Chunk.path(),
            reason,
            bad_rows = bad,
            session_id = who.session_id.as_deref(), // nt:id-only telemetry session UUID from the token; it names nothing
            install_id = who.install_id.as_deref(), // nt:id-only launcher install UUID from the token; it names nothing
            peer = %who.peer,
            suppressed,
            "telemetry upload-chunk skipped rows that did not parse"
        );
    }
}

/// Where an accepted upload was cut short.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Truncation {
    /// The budget that was hit: `decompressed bytes` or `rows` for a
    /// chunk, `zip entries`, `expanded bytes` or `lines` for a bundle.
    pub budget: &'static str,
    pub limit: u64,
    /// Rows (chunk) or lines (bundle) replayed.
    pub kept: u64,
    /// What was dropped, in the budget's own unit: rows for a chunk's
    /// `decompressed bytes` and `rows` budgets (estimated from the
    /// compression ratio past the expansion cap), files for a bundle's
    /// `zip entries`, **bytes** for its `expanded bytes` (the declared or
    /// read sizes of the files not replayed), lines for its `lines`.
    pub dropped_estimate: u64,
}
