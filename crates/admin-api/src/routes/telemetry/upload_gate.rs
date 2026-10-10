//! What every chunk and bundle upload passes before its body is read: the
//! kill switch, the bearer token, the per-session and per-address rate
//! limits, and a concurrency slot.
//!
//! The order matters. Each step costs less than the next, and none of them
//! touches the body: an upload refused here has had nothing buffered,
//! decompressed or parsed. The body is read only by a request holding a
//! slot, so at most [`UploadLimits::chunk_slots`] chunks and
//! [`UploadLimits::bundle_slots`] bundles are buffered or expanded at once.

use std::future::poll_fn;
use std::net::IpAddr;
use std::pin::pin;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use axum::body::{Body, HttpBody};
use axum::http::HeaderMap;
#[cfg(test)]
use tokio::sync::OwnedSemaphorePermit;
use tokio::sync::Semaphore;

use crate::routes::dev_session::quota::{install_key, ip_key, WindowTable};
use crate::routes::dev_session::{
    decode_token, env_u32, kill_switch_active, AuthError, TokenClaims, SCOPE_TELEMETRY_WRITE,
};

use super::dto::IngestError;
use super::refusal_log::RefusalLog;
#[cfg(test)]
use super::upload_slots::PeerSlot;
use super::upload_slots::{take_slot, PeerSlots, UploadSlot};
use super::{
    BODY_TIMEOUT, BUNDLE_SLOTS, CHUNK_SLOTS, MAX_BUNDLE_BYTES, MAX_BUNDLE_ENTRIES,
    MAX_BUNDLE_ENTRIES_HARD, MAX_BUNDLE_EXPANDED_BYTES, MAX_BUNDLE_LINES,
    MAX_BUNDLE_METADATA_BYTES, MAX_BUNDLE_PARTS, MAX_CHUNK_BYTES, MAX_CHUNK_DECOMPRESSED_BYTES,
    MAX_CHUNK_ROWS, SLOTS_PER_PEER,
};

/// Which upload route a request is for: each has its own rate limits and
/// its own pool of slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Route {
    Chunk,
    Bundle,
}

impl Route {
    pub(super) fn path(self) -> &'static str {
        match self {
            Route::Chunk => "upload-chunk",
            Route::Bundle => "upload-bundle",
        }
    }
}

/// The chunk rate window. A launcher and its DLL each post about every
/// 2 s on one token, so a session sends about 60 chunks a minute.
const CHUNK_QUOTA_WINDOW: Duration = Duration::from_secs(60);

/// The bundle rate window. A launcher sends one bundle when the game
/// exits.
const BUNDLE_QUOTA_WINDOW: Duration = Duration::from_secs(3_600);

/// Chunks per session per minute: twice what a launcher and its DLL send
/// together, so a retry burst after a network blip still fits.
const DEFAULT_CHUNK_PER_SESSION: u32 = 120;

/// Chunks per peer address per minute: ten sessions' worth, for a lab
/// machine or a household behind one address.
const DEFAULT_CHUNK_PER_IP: u32 = 600;

/// Bundles per session per hour: the one at exit and a few retries.
const DEFAULT_BUNDLE_PER_SESSION: u32 = 6;

/// Bundles per peer address per hour.
const DEFAULT_BUNDLE_PER_IP: u32 = 30;

/// The operator-tunable part of the policy, read per request like the
/// mint's `QuotaPolicy`: change the env and restart. A limit of 0 disables
/// that counter.
#[derive(Debug, Clone)]
pub(super) struct UploadPolicy {
    /// `CIMMERIA_TELEMETRY_KILL_SWITCH=1`: uploads are refused with 503,
    /// like mint and refresh.
    pub kill_switch: bool,
    /// `CIMMERIA_TELEMETRY_UPLOAD_QUOTA_PER_SESSION`, per minute.
    pub chunk_per_session: u32,
    /// `CIMMERIA_TELEMETRY_UPLOAD_QUOTA_PER_IP`, per minute.
    pub chunk_per_ip: u32,
    /// `CIMMERIA_TELEMETRY_BUNDLE_QUOTA_PER_SESSION`, per hour.
    pub bundle_per_session: u32,
    /// `CIMMERIA_TELEMETRY_BUNDLE_QUOTA_PER_IP`, per hour.
    pub bundle_per_ip: u32,
}

impl UploadPolicy {
    pub(super) fn from_env() -> Self {
        Self {
            kill_switch: kill_switch_active(),
            chunk_per_session: env_u32(
                "CIMMERIA_TELEMETRY_UPLOAD_QUOTA_PER_SESSION",
                DEFAULT_CHUNK_PER_SESSION,
            ),
            chunk_per_ip: env_u32(
                "CIMMERIA_TELEMETRY_UPLOAD_QUOTA_PER_IP",
                DEFAULT_CHUNK_PER_IP,
            ),
            bundle_per_session: env_u32(
                "CIMMERIA_TELEMETRY_BUNDLE_QUOTA_PER_SESSION",
                DEFAULT_BUNDLE_PER_SESSION,
            ),
            bundle_per_ip: env_u32(
                "CIMMERIA_TELEMETRY_BUNDLE_QUOTA_PER_IP",
                DEFAULT_BUNDLE_PER_IP,
            ),
        }
    }

    /// The defaults with the kill switch off, for tests.
    #[cfg(test)]
    pub(super) fn defaults() -> Self {
        Self {
            kill_switch: false,
            chunk_per_session: DEFAULT_CHUNK_PER_SESSION,
            chunk_per_ip: DEFAULT_CHUNK_PER_IP,
            bundle_per_session: DEFAULT_BUNDLE_PER_SESSION,
            bundle_per_ip: DEFAULT_BUNDLE_PER_IP,
        }
    }
}

/// The size budgets. Fixed in code (see the constants in the module root
/// for why each value); a struct so tests can shrink them.
#[derive(Debug, Clone)]
pub(super) struct UploadLimits {
    pub chunk_body_bytes: usize,
    pub chunk_decompressed_bytes: u64,
    pub chunk_rows: usize,
    pub bundle_body_bytes: usize,
    pub bundle_expanded_bytes: u64,
    pub bundle_entries: usize,
    pub bundle_lines: u64,
    pub bundle_metadata_bytes: usize,
    pub bundle_parts: usize,
    /// Zip entries past which a bundle is refused outright, from its end
    /// record, before the archive is opened. Between `bundle_entries` and
    /// this, the newest `bundle_entries` are replayed.
    pub bundle_entries_hard: usize,
    pub chunk_slots: usize,
    pub bundle_slots: usize,
    pub chunk_slots_per_peer: usize,
    pub bundle_slots_per_peer: usize,
    /// How long one request may take to deliver its body.
    pub body_timeout: Duration,
}

impl Default for UploadLimits {
    fn default() -> Self {
        Self {
            chunk_body_bytes: MAX_CHUNK_BYTES,
            chunk_decompressed_bytes: MAX_CHUNK_DECOMPRESSED_BYTES,
            chunk_rows: MAX_CHUNK_ROWS,
            bundle_body_bytes: MAX_BUNDLE_BYTES,
            bundle_expanded_bytes: MAX_BUNDLE_EXPANDED_BYTES,
            bundle_entries: MAX_BUNDLE_ENTRIES,
            bundle_lines: MAX_BUNDLE_LINES,
            bundle_metadata_bytes: MAX_BUNDLE_METADATA_BYTES,
            bundle_parts: MAX_BUNDLE_PARTS,
            bundle_entries_hard: MAX_BUNDLE_ENTRIES_HARD,
            chunk_slots: CHUNK_SLOTS,
            bundle_slots: BUNDLE_SLOTS,
            chunk_slots_per_peer: SLOTS_PER_PEER,
            bundle_slots_per_peer: SLOTS_PER_PEER,
            body_timeout: BODY_TIMEOUT,
        }
    }
}

/// What the upload routes remember between requests. It lives for the
/// process and is passed in, so each test gets a fresh one.
pub(super) struct UploadState {
    pub limits: UploadLimits,
    chunk_session: WindowTable,
    chunk_ip: WindowTable,
    bundle_session: WindowTable,
    bundle_ip: WindowTable,
    chunk_slots: Arc<Semaphore>,
    bundle_slots: Arc<Semaphore>,
    peer_slots: Arc<PeerSlots>,
    pub refusals: RefusalLog,
}

impl UploadState {
    pub(super) fn new(limits: UploadLimits) -> Self {
        Self {
            chunk_slots: Arc::new(Semaphore::new(limits.chunk_slots)),
            bundle_slots: Arc::new(Semaphore::new(limits.bundle_slots)),
            peer_slots: Arc::default(),
            limits,
            chunk_session: WindowTable::new(),
            chunk_ip: WindowTable::new(),
            bundle_session: WindowTable::new(),
            bundle_ip: WindowTable::new(),
            refusals: RefusalLog::new(),
        }
    }

    /// Take a server-wide chunk slot as an upload in progress would, for
    /// tests.
    #[cfg(test)]
    pub(super) fn hold_chunk_slot(&self) -> OwnedSemaphorePermit {
        Arc::clone(&self.chunk_slots)
            .try_acquire_owned()
            .expect("a free chunk slot")
    }

    /// Take `peer`'s share of `route`'s slots as an upload in progress
    /// would, for tests.
    #[cfg(test)]
    pub(super) fn hold_peer_slot(&self, route: Route, peer: IpAddr) -> PeerSlot {
        self.peer_slots
            .try_take(route, ip_key(peer), 1)
            .expect("a free peer slot")
    }

    /// Addresses with an upload in flight, for tests.
    #[cfg(test)]
    pub(super) fn peers_in_flight(&self) -> usize {
        self.peer_slots.tracked()
    }
}

/// The process-wide state the two handlers use.
pub(super) fn upload_state() -> &'static UploadState {
    static STATE: OnceLock<UploadState> = OnceLock::new();
    STATE.get_or_init(|| UploadState::new(UploadLimits::default()))
}

/// Who sent a request, as far as the gate got: the peer always, the
/// token's identity once it verified. The refusal log keys and labels its
/// rows with it.
#[derive(Debug, Clone)]
pub(super) struct Uploader {
    pub peer: IpAddr,
    pub session_id: Option<String>,
    pub install_id: Option<String>,
}

impl Uploader {
    pub(super) fn anonymous(peer: IpAddr) -> Self {
        Self {
            peer,
            session_id: None,
            install_id: None,
        }
    }
}

/// A request that passed the gate: its claims and the slot it holds. The
/// slot is released when it drops, so the caller keeps it for as long as
/// it holds the body or works on it.
#[derive(Debug)]
pub(super) struct Admitted {
    pub claims: TokenClaims,
    pub slot: UploadSlot,
}

/// Run the gate for one request: kill switch (503), token (401), session
/// rate (429), address rate (429), slot (503). `who` is filled in as soon
/// as the token verifies.
///
/// The session's quota is charged before the address's so a session that
/// is over its own allowance does not also spend its neighbours'.
pub(super) fn admit(
    state: &UploadState,
    policy: &UploadPolicy,
    route: Route,
    headers: &HeaderMap,
    who: &mut Uploader,
    now: Instant,
) -> Result<Admitted, IngestError> {
    if policy.kill_switch {
        return Err(IngestError::Auth(AuthError::KillSwitchActive));
    }
    let claims = verify_bearer(headers)?;
    who.session_id = Some(claims.sid.clone());
    who.install_id = Some(claims.sub.clone());

    let (session_table, ip_table, per_session, per_ip, window, slots) = match route {
        Route::Chunk => (
            &state.chunk_session,
            &state.chunk_ip,
            policy.chunk_per_session,
            policy.chunk_per_ip,
            CHUNK_QUOTA_WINDOW,
            &state.chunk_slots,
        ),
        Route::Bundle => (
            &state.bundle_session,
            &state.bundle_ip,
            policy.bundle_per_session,
            policy.bundle_per_ip,
            BUNDLE_QUOTA_WINDOW,
            &state.bundle_slots,
        ),
    };
    // `install_key` is a seeded hash of any caller string; the session id
    // is one, and is the token's own.
    session_table
        .check_and_record(
            install_key(&claims.sid),
            per_session,
            window,
            "upload/session",
            now,
        )
        .map_err(|q| IngestError::Auth(AuthError::QuotaExceeded(q)))?;
    ip_table
        .check_and_record(ip_key(who.peer), per_ip, window, "upload/ip", now)
        .map_err(|q| IngestError::Auth(AuthError::QuotaExceeded(q)))?;
    // Refuse rather than queue: a queued request would still hold its
    // connection and, for a bundle, a body the client keeps sending. The
    // uploaders retry on their next flush.
    let per_peer = match route {
        Route::Chunk => state.limits.chunk_slots_per_peer,
        Route::Bundle => state.limits.bundle_slots_per_peer,
    };
    let slot = take_slot(&state.peer_slots, slots, route, ip_key(who.peer), per_peer)
        .ok_or(IngestError::Busy)?;
    Ok(Admitted { claims, slot })
}

/// Await `fut` until `deadline`; past it the body read is refused as
/// [`IngestError::Body`] (`body_read_failed`), so a client that sends its
/// body slowly cannot hold a slot indefinitely.
pub(super) async fn by_deadline<T>(
    deadline: tokio::time::Instant,
    fut: impl std::future::Future<Output = Result<T, IngestError>>,
) -> Result<T, IngestError> {
    tokio::time::timeout_at(deadline, fut)
        .await
        .map_err(|_| IngestError::Body)?
}

/// Read a request body, refusing it as soon as it passes `cap`: the frame
/// that would cross the cap is never copied in. There is no shortcut on
/// `Content-Length`, so a length-prefixed body and a chunked one take the
/// same path.
pub(super) async fn read_body_capped(body: Body, cap: usize) -> Result<Vec<u8>, IngestError> {
    let mut body = pin!(body);
    let mut bytes = Vec::new();
    while let Some(frame) = poll_fn(|cx| body.as_mut().poll_frame(cx)).await {
        let frame = frame.map_err(|_| IngestError::Body)?;
        let Ok(data) = frame.into_data() else {
            continue;
        };
        if bytes.len() + data.len() > cap {
            return Err(IngestError::TooLarge(bytes.len() + data.len(), cap));
        }
        bytes.extend_from_slice(&data);
    }
    Ok(bytes)
}

pub(super) fn verify_bearer(headers: &HeaderMap) -> Result<TokenClaims, IngestError> {
    let raw = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(IngestError::MissingAuth)?;
    let token = raw
        .strip_prefix("Bearer ")
        .ok_or(IngestError::MissingAuth)?
        .trim();
    if token.is_empty() {
        return Err(IngestError::MissingAuth);
    }
    // Single source of truth for the HMAC secret — `dev_session::mint`
    // signs with this same loader, so any drift between the two paths
    // would cause every launcher upload to fail HMAC verification.
    let secret = crate::routes::dev_session::load_secret().map_err(IngestError::Auth)?;
    let claims = decode_token(token, &secret).map_err(IngestError::Auth)?;
    let now = chrono::Utc::now().timestamp();
    if claims.exp <= now {
        return Err(IngestError::Auth(AuthError::Expired {
            exp: claims.exp,
            now,
        }));
    }
    // The scope is the only thing that keeps a minted token from
    // being a general-purpose credential, so it has to be checked
    // here rather than assumed from the mint path.
    if !claims.has_scope(SCOPE_TELEMETRY_WRITE) {
        return Err(IngestError::Auth(AuthError::MissingScope {
            wanted: SCOPE_TELEMETRY_WRITE,
        }));
    }
    Ok(claims)
}
