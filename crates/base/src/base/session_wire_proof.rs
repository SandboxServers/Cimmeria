//! Bounded, lab-only evidence of successful origin-tagged Base UDP sends.

use std::collections::{HashMap, VecDeque};
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use cimmeria_base_session::base::ConnectedClientState;
use cimmeria_mercury::instrumentation::wire_fingerprint;
use cimmeria_mercury::transport::{BidirectionalTransport, SendOrigin, Transport};

const CAPACITY: usize = 256;
const MAX_AGE: Duration = Duration::from_secs(30);
type Connected = Mutex<HashMap<SocketAddr, ConnectedClientState>>;

/// A process boot and monotonically increasing Phase 3 generation issuer.
pub(crate) struct SessionOriginIssuer {
    boot_id: Arc<str>,
    next_generation: AtomicU64,
}

impl SessionOriginIssuer {
    /// Mint a process-independent boot using the operating system CSPRNG.
    pub(crate) fn new() -> io::Result<Self> {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(io::Error::other)?;
        let boot_id: Arc<str> = bytes
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            .into();
        Ok(Self {
            boot_id,
            next_generation: AtomicU64::new(0),
        })
    }

    /// Issue a ticket after the two pre-insertion handshake sends succeed.
    pub(crate) fn issue(
        &self,
        peer: SocketAddr,
        connected_at: Instant,
    ) -> cimmeria_base_session::base::SessionOriginSeed {
        let counter = self
            .next_generation
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .expect("Phase 3 generation counter exhausted")
            + 1;
        cimmeria_base_session::base::SessionOriginSeed {
            server_boot_id: Arc::clone(&self.boot_id),
            session_generation: format!("{}:{counter}", self.boot_id).into(),
            peer,
            connected_at,
        }
    }
}

/// One successful, full-length UDP send; the wire bytes are never retained.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProofRow {
    pub sequence: u64,
    pub sent_at_ms: u64,
    pub server_boot_id: Arc<str>,
    pub session_generation: Arc<str>,
    pub peer: SocketAddr,
    pub player_entity_id: Option<u32>,
    pub length: usize,
    pub fingerprint: String,
}

/// Named diagnostics. Totals are monotonic per boot; pending counts are
/// current gauges. Eligibility uses the latest gap in the bounded window.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ProofGaps {
    pub untagged: u64,
    pub pre_phase3_exempt: u64,
    pub origin_peer_mismatch: u64,
    pub pending_tagged: u64,
    pub pending_untagged: u64,
    pub cancelled_send: u64,
    pub failed_send: u64,
    pub partial_send: u64,
    pub chaos_simulated: u64,
    pub ring_overflow: u64,
    pub ring_expired: u64,
}

/// A bounded snapshot. Only generations still in the connected map appear.
#[derive(Clone, Debug)]
#[allow(dead_code)] // DW-03S-S owns read-only snapshot plumbing.
pub(crate) struct ProofSnapshot {
    pub rows: Vec<ProofRow>,
    pub next_sequence: u64,
    pub dropped: u64,
    pub gaps: ProofGaps,
    pub coverage_complete: bool,
}

struct TimedRow {
    added: Instant,
    #[allow(dead_code)] // Read by the DW-03S-S snapshot endpoint.
    row: ProofRow,
}

#[derive(Default)]
struct Ring {
    rows: VecDeque<TimedRow>,
    next_sequence: u64,
    dropped: u64,
    gaps: ProofGaps,
    last_gap_at: Option<Instant>,
}

/// A dropped send future must not disappear from coverage accounting.
struct SendAttempt<'a> {
    ring: &'a Mutex<Ring>,
    tagged: bool,
    completed: bool,
}

impl<'a> SendAttempt<'a> {
    fn begin(ring: &'a Mutex<Ring>, tagged: bool) -> Self {
        let mut guard = ring.lock().unwrap_or_else(PoisonError::into_inner);
        if tagged {
            guard.gaps.pending_tagged += 1;
        } else {
            guard.gaps.pending_untagged += 1;
        }
        Self {
            ring,
            tagged,
            completed: false,
        }
    }
}

impl Drop for SendAttempt<'_> {
    fn drop(&mut self) {
        let mut ring = self.ring.lock().unwrap_or_else(PoisonError::into_inner);
        if self.tagged {
            ring.gaps.pending_tagged -= 1;
        } else {
            ring.gaps.pending_untagged -= 1;
        }
        if !self.completed {
            ring.gaps.cancelled_send += 1;
            ring.note_gap(Instant::now());
        }
    }
}

impl Ring {
    fn note_gap(&mut self, now: Instant) {
        self.last_gap_at = Some(self.last_gap_at.map_or(now, |previous| previous.max(now)));
    }

    fn coverage_incomplete(&self, now: Instant) -> bool {
        self.gaps.pending_tagged != 0
            || self.gaps.pending_untagged != 0
            || self
                .last_gap_at
                .is_some_and(|gap| now.saturating_duration_since(gap) < MAX_AGE)
    }

    fn expire(&mut self, now: Instant) {
        while self
            .rows
            .front()
            .is_some_and(|row| now.saturating_duration_since(row.added) >= MAX_AGE)
        {
            self.rows.pop_front();
            self.dropped += 1;
            self.gaps.ring_expired += 1;
        }
    }

    fn record(&mut self, bytes: &[u8], origin: &SendOrigin, now: Instant) {
        self.expire(now);
        if self.rows.len() == CAPACITY {
            self.rows.pop_front();
            self.dropped += 1;
            self.gaps.ring_overflow += 1;
            self.note_gap(now);
        }
        self.next_sequence += 1;
        let sent_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.rows.push_back(TimedRow {
            added: now,
            row: ProofRow {
                sequence: self.next_sequence,
                sent_at_ms,
                server_boot_id: Arc::clone(&origin.server_boot_id),
                session_generation: Arc::clone(&origin.session_generation),
                peer: origin.peer,
                player_entity_id: origin.player_entity_id,
                length: bytes.len(),
                fingerprint: wire_fingerprint(bytes),
            },
        });
    }
}

/// Shared send observer around the exact transport selected for Base recv.
pub(crate) struct ProofTransport {
    inner: Arc<dyn BidirectionalTransport>,
    #[allow(dead_code)] // Read by the DW-03S-S snapshot endpoint.
    connected: Weak<Connected>,
    chaos_unsafe: bool,
    ring: Mutex<Ring>,
}

impl ProofTransport {
    pub(crate) fn new(
        inner: Arc<dyn BidirectionalTransport>,
        connected: &Arc<Connected>,
        chaos_unsafe: bool,
    ) -> Self {
        Self {
            inner,
            connected: Arc::downgrade(connected),
            chaos_unsafe,
            ring: Mutex::new(Ring::default()),
        }
    }

    /// Read the current generations without exposing stale disconnect rows.
    #[allow(dead_code)] // DW-03S-S exposes this through its authorized read path.
    pub(crate) fn snapshot(&self) -> ProofSnapshot {
        self.snapshot_after_lock(|| {})
    }

    fn snapshot_after_lock(&self, after_lock: impl FnOnce()) -> ProofSnapshot {
        let connected = self.connected.upgrade();
        // Lock order is connected -> ring. Transport sends take ring only;
        // session insertion/eviction takes connected only, and releases it
        // before any awaited send. Keep the connected guard through row
        // filtering so a relog cannot publish B before A's snapshot returns.
        let sessions = connected
            .as_ref()
            .and_then(|connected| connected.lock().ok());
        let active = sessions.as_ref().map(|sessions| {
            sessions
                .iter()
                .filter_map(|(addr, session)| {
                    let origin = session.wire_send_origin()?;
                    (origin.peer == *addr).then_some((
                        origin.server_boot_id,
                        origin.session_generation,
                        origin.peer,
                    ))
                })
                .collect::<Vec<_>>()
        });
        after_lock();
        let mut ring = self.ring.lock().unwrap_or_else(PoisonError::into_inner);
        let now = Instant::now();
        ring.expire(now);
        let rows = active
            .as_ref()
            .map(|active| {
                ring.rows
                    .iter()
                    .filter(|timed| {
                        active.iter().any(|(boot, generation, peer)| {
                            *boot == timed.row.server_boot_id
                                && *generation == timed.row.session_generation
                                && *peer == timed.row.peer
                        })
                    })
                    .map(|timed| timed.row.clone())
                    .collect()
            })
            .unwrap_or_default();
        ProofSnapshot {
            rows,
            next_sequence: ring.next_sequence,
            dropped: ring.dropped,
            gaps: ring.gaps.clone(),
            coverage_complete: active.is_some()
                && !self.chaos_unsafe
                && !ring.coverage_incomplete(now),
        }
    }
}

#[async_trait]
impl Transport for ProofTransport {
    async fn send_to(&self, bytes: &[u8], addr: SocketAddr) -> io::Result<usize> {
        {
            let mut ring = self.ring.lock().unwrap_or_else(PoisonError::into_inner);
            ring.gaps.untagged += 1;
            ring.note_gap(Instant::now());
        }
        let mut attempt = SendAttempt::begin(&self.ring, false);
        let result = self.inner.send_to(bytes, addr).await;
        self.ring
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .note_gap(Instant::now());
        attempt.completed = true;
        result
    }

    async fn send_to_pre_phase3(&self, bytes: &[u8], addr: SocketAddr) -> io::Result<usize> {
        self.ring
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .gaps
            .pre_phase3_exempt += 1;
        self.inner.send_to(bytes, addr).await
    }

    async fn send_to_with_origin(
        &self,
        bytes: &[u8],
        addr: SocketAddr,
        origin: &SendOrigin,
    ) -> io::Result<usize> {
        let ticket = origin.clone();
        let peer_matches = addr == ticket.peer;
        {
            let mut ring = self.ring.lock().unwrap_or_else(PoisonError::into_inner);
            if !peer_matches {
                ring.gaps.origin_peer_mismatch += 1;
                ring.note_gap(Instant::now());
            }
            if self.chaos_unsafe {
                ring.gaps.chaos_simulated += 1;
                ring.note_gap(Instant::now());
            }
        }
        let mut attempt = SendAttempt::begin(&self.ring, true);
        let result = self.inner.send_to(bytes, addr).await;
        {
            let mut ring = self.ring.lock().unwrap_or_else(PoisonError::into_inner);
            match &result {
                Err(_) => {
                    ring.gaps.failed_send += 1;
                    ring.note_gap(Instant::now());
                }
                Ok(n) if *n != bytes.len() => {
                    ring.gaps.partial_send += 1;
                    ring.note_gap(Instant::now());
                }
                Ok(_) if peer_matches && !self.chaos_unsafe => {
                    ring.record(bytes, &ticket, Instant::now())
                }
                Ok(_) => ring.note_gap(Instant::now()),
            }
        }
        attempt.completed = true;
        result
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }
}

#[async_trait]
impl BidirectionalTransport for ProofTransport {
    async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.inner.recv_from(buf).await
    }
}

#[cfg(test)]
#[path = "session_wire_proof_tests.rs"]
mod tests;
