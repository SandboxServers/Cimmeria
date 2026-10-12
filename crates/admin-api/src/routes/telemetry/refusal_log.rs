//! One throttled row per refused upload (negative-logging Pattern D). The
//! row is a `warn`, except `busy`, which is `info`: it is retried
//! back-pressure (503 + Retry-After) and an operator has nothing to act on.
//!
//! A refused uploader usually keeps trying: the launcher and the DLL retry
//! every flush, and an unauthenticated caller can repeat as fast as it
//! likes. So the first refusal for an `(uploader, reason)` pair writes a
//! row at once, the repeats inside the pair's window are counted, and the
//! next row for the pair carries the count in `suppressed`.
//!
//! The window starts at [`WINDOW`] and doubles with every row the pair
//! writes (10 s, 20 s, 40 s, ...) up to [`MAX_WINDOW`]. A fixed window
//! never bites on a steady retrier slower than it: on 2026-10-12 an
//! abandoned lab client retried an expired token every ~12 s for 4.5 h and
//! got one row per retry, each with `suppressed=2`. With the doubling, a
//! pair refused every 12 s writes about ten rows in its first hour and
//! then four an hour. The window goes back to [`WINDOW`] once the pair has
//! been quiet (no refusal at all, written or counted) for [`RESET_QUIET`].
//! That is measured from the pair's last refusal rather than compared to
//! its current window, because a reset tied to a 10 s window would let any
//! retrier slower than about 20 s through on every retry, forever.
//!
//! On top of that, at most [`GLOBAL_ROWS_PER_WINDOW`] rows are written per
//! [`WINDOW`] across every uploader, so many distinct uploaders cannot turn
//! refusals into a log flood either; a row held back by that ceiling is
//! counted in its pair's `suppressed` like any other, and does not widen
//! the pair's window (no row was written).
//!
//! A refusal of the uploader's own credential (no token, a bad token, an
//! expired one: [`IngestError::is_credential_refusal`]) carries
//! `event = "upload_auth_refused"`; every other refusal carries
//! `event = "upload_refused"`. The Discord layer keeps the first in SigNoz
//! only (`SIGNOZ_ONLY_EVENTS`): it is the uploader's state, not a server
//! fault. Both stay at their level, so SigNoz sees every row.
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

/// A pair's first window: how long its repeats are counted instead of
/// written after its first row. Also the global ceiling's window.
pub(super) const WINDOW: Duration = Duration::from_secs(10);

/// The widest a pair's window grows by doubling.
pub(super) const MAX_WINDOW: Duration = Duration::from_secs(15 * 60);

/// A pair quiet this long since its last refusal starts again at
/// [`WINDOW`].
pub(super) const RESET_QUIET: Duration = MAX_WINDOW;

/// Rows written per [`WINDOW`] across every uploader.
pub(super) const GLOBAL_ROWS_PER_WINDOW: u32 = 50;

/// `event` on a refusal of the uploader's credential. SigNoz-only: the
/// Discord layer's `SIGNOZ_ONLY_EVENTS` lists `("launcher.ingest", this)`.
pub(super) const AUTH_REFUSED_EVENT: &str = "upload_auth_refused";

/// `event` on every other refusal. These still post to Discord at WARN.
pub(super) const REFUSED_EVENT: &str = "upload_refused";

/// Slots in the per-pair table.
const SLOTS: usize = 512;

#[derive(Debug, Clone, Copy)]
struct Slot {
    key: u64,
    /// When the pair's current window opened (its last row, or the last
    /// refusal the global ceiling held back).
    opened: Instant,
    /// The pair's current window, [`WINDOW`] to [`MAX_WINDOW`].
    window: Duration,
    /// The pair's latest refusal, written or not.
    last_seen: Instant,
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
        // `held_window` is the pair's window if the ceiling holds this one
        // back (unchanged: no row is written); `next_window` if it emits.
        let (carried, held_window, next_window) = match inner.slots[idx] {
            Some(s) if s.key == key && now.saturating_duration_since(s.opened) < s.window => {
                inner.slots[idx] = Some(Slot {
                    suppressed: s.suppressed + 1,
                    last_seen: now,
                    ..s
                });
                return Decision::Suppress;
            }
            Some(s) if s.key == key => {
                if now.saturating_duration_since(s.last_seen) >= RESET_QUIET {
                    (s.suppressed, WINDOW, WINDOW)
                } else {
                    (s.suppressed, s.window, (s.window * 2).min(MAX_WINDOW))
                }
            }
            // A new pair, or a takeover of a slot another pair held.
            _ => (0, WINDOW, WINDOW),
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
                window: held_window,
                last_seen: now,
                suppressed: carried + 1,
            });
            return Decision::Suppress;
        }
        global.rows += 1;
        inner.slots[idx] = Some(Slot {
            key,
            opened: now,
            window: next_window,
            last_seen: now,
            suppressed: 0,
        });
        Decision::Emit {
            suppressed: carried,
        }
    }

    /// Log `err`, a refusal of `route` to `who`, unless the throttle holds
    /// it back. At WARN, except `busy`, which is INFO: it is retried
    /// back-pressure, not a fault.
    pub(super) fn report(&self, route: Route, who: &Uploader, err: &IngestError, now: Instant) {
        let reason = err.reason();
        let peer = who.peer.to_string();
        let subject = who.session_id.as_deref().unwrap_or(&peer);
        let Decision::Emit { suppressed } = self.decide(reason, subject, now) else {
            return;
        };
        // A credential refusal is the uploader's state, not a server fault:
        // its own `event` keeps it out of the Discord errors channel, at the
        // same WARN level so SigNoz parity holds.
        let event = if err.is_credential_refusal() {
            AUTH_REFUSED_EVENT
        } else {
            REFUSED_EVENT
        };
        // `tracing` fixes an event's level at the call site, so one call per
        // level; the macro keeps the field list in one place.
        macro_rules! refusal {
            ($level:expr) => {
                tracing::event!(
                    target: "launcher.ingest",
                    $level,
                    event,
                    route = route.path(),
                    reason,
                    budget = err.budget(),
                    pool = err.pool(),
                    limit = err.limit(),
                    session_id = who.session_id.as_deref(), // nt:id-only telemetry session UUID from the token; it names nothing
                    install_id = who.install_id.as_deref(), // nt:id-only launcher install UUID from the token; it names nothing
                    cimmeria.session_kind = who.session_kind.as_deref(),
                    peer = %who.peer,
                    suppressed,
                    "telemetry upload refused: {reason}"
                )
            };
        }
        if matches!(err, IngestError::Busy { .. }) {
            refusal!(tracing::Level::INFO);
        } else {
            refusal!(tracing::Level::WARN);
        }
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
