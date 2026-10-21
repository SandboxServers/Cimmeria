//! Minigame session registry and room management.
//!
//! Tracks pending minigame sessions (ticket → game params) and active rooms
//! for spectator/helper broadcasting.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use rand::RngExt;
use tokio::sync::{watch, Mutex};
use tokio::time::Instant;

pub use super::session_claim::ClaimRejection;
#[path = "session_admission.rs"]
mod admission;
pub use admission::{AdmissionGuard, AdmissionRefusal, AdmissionRefusalReason};
#[path = "session_result.rs"]
mod session_result;
#[path = "session_sweep.rs"]
mod session_sweep;
use super::session_snapshot::PublishedBoard;
pub use super::session_snapshot::{BoardSnapshot, WireSnapshot};
pub(crate) use session_result::DepositOutcome;
pub use session_result::ResultReceipt;
use session_sweep::expire_pending_locked;

/// How long a session may sit in the registry after `register` before the
/// sweep drops it, *if its SWF client never connected*.
///
/// **This TTL is a backstop, not a port.** The original had no timeout of
/// any kind: `MinigameRequestManager::QueueEntry`
/// (`deprecated/cpp/src/baseapp/minigame.hpp`) carries no timestamp, and
/// entries left the queue only through an explicit remove or cancel. Its
/// recovery path included client-driven cancellation. Cimmeria handles
/// window close through `endCurrentMinigame`; `minigameStartCancel` (cell
/// method 30) remains unimplemented. This TTL backs up a client crash
/// before the SWF connects.
///
/// The value is sized off the handshake it has to outlast — base pushes
/// `onStartMinigame(URL)`, the client opens the Flash surface, the SWF
/// loads and opens a TCP socket to the SmartFox port, all of which takes
/// seconds on a healthy client. Three minutes is generous headroom for a
/// slow load while still being far shorter than any plausible play session.
///
/// Connected sessions are never expired by age — see [`MinigameSession::connected`].
pub const PENDING_SESSION_TTL: Duration = Duration::from_secs(180);

/// How often [`SessionRegistry::spawn_sweep`] runs the expiry pass.
///
/// Sized well below [`PENDING_SESSION_TTL`] so the worst-case extra wait a
/// player sees past the TTL is one interval.
pub const SWEEP_INTERVAL: Duration = Duration::from_secs(60);

/// How young a pending session must be for a second `register` on the same
/// entity to count as a double interact (INFO) rather than a stuck launch
/// (WARN). TG-MG-04.
const DOUBLE_INTERACT_WINDOW: Duration = Duration::from_secs(2);

/// A registered minigame session awaiting client connection.
#[derive(Debug, Clone)]
pub struct MinigameSession {
    pub entity_id: u32,
    pub player_id: i32,
    /// Original Base connection, captured when the launch URL was sent.
    pub session_instance_id: u64,
    pub game_name: String,
    pub difficulty: u32,
    pub tech_competency: u32,
    pub seed: u32,
    pub abilities_mask: u32,
    pub intelligence: u32,
    pub player_level: u32,
    pub ticket: String,
    pub on_victory_chains: Vec<i64>,
    /// Fired by the cell on result codes 2 and 4 (D-MG5).
    pub on_failure_chains: Vec<i64>,
    /// The player's character name, from the base session that started the
    /// game. The minigame server only ever sees the entity id; this is what
    /// names the player in the Discord result (NT-10).
    pub player_name: Option<String>,
    /// The IP of the player's game connection, the only address the minigame
    /// listener admits for this session.
    pub client_ip: Option<std::net::IpAddr>,
    /// `tokio::time::Instant` rather than `std::time::Instant` so
    /// `tokio::time::pause()` / `advance()` drive the TTL deterministically
    /// in tests instead of a wall-clock sleep.
    pub created_at: Instant,
    /// Set by [`SessionRegistry::authenticate_and_claim`] in the same locked
    /// step that validates the ticket, so a connection task can only ever
    /// claim the session it authenticated against.
    ///
    /// A connected session is exempt from age-based expiry: its connection
    /// task removes it when the socket closes, and a long Livewire round can
    /// easily outlive [`PENDING_SESSION_TTL`].
    pub connected: bool,
    /// Persists while this registration is live even after its receipt is spent.
    pub terminal_reported: bool,
    /// The cell's explicit close request reaches the connection task even if
    /// the SWF leaves its SmartFox socket open after hiding the window.
    pub cancel: watch::Sender<bool>,
}

/// Thread-safe session registry shared between BaseApp and the minigame TCP server.
#[derive(Clone)]
pub struct SessionRegistry {
    inner: Arc<Mutex<SessionRegistryInner>>,
    /// In-flight admissions per peer key (#1119). A std mutex because the
    /// guard's Drop cannot await. Lock order: `inner`, then this.
    admitted: Arc<std::sync::Mutex<HashMap<admission::AdmissionKey, usize>>>,
}

struct SessionRegistryInner {
    /// entity_id → session
    sessions: HashMap<u32, MinigameSession>,
    boards: HashMap<u32, PublishedBoard>,
    receipts: HashMap<String, ResultReceipt>,
    receipt_order: VecDeque<String>,
    /// Next room ID (auto-incrementing from 1001)
    next_room_id: u32,
    /// The admission slot of each claimed connection, by entity: the ticket
    /// it belongs to and a weak handle, so a cancel can free the slot at once
    /// (see [`AdmissionGuard`]). At most one entry per entity.
    bound_slots: HashMap<u32, (String, std::sync::Weak<admission::Slot>)>,
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MinigameSession {
    /// The player as a Discord pair: the character's `player_id` and the
    /// name the base registered the session with (NT-10).
    pub fn discord_player(&self) -> cimmeria_discord::Named {
        cimmeria_discord::Named::new(self.player_id, self.player_name.clone())
    }
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SessionRegistryInner {
                sessions: HashMap::new(),
                boards: HashMap::new(),
                receipts: HashMap::new(),
                receipt_order: VecDeque::new(),
                next_room_id: 1001,
                bound_slots: HashMap::new(),
            })),
            admitted: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Register a new minigame session. Returns the generated ticket.
    #[allow(clippy::too_many_arguments)]
    pub async fn register(
        &self,
        entity_id: u32,
        player_id: i32,
        game_name: String,
        difficulty: u32,
        tech_competency: u32,
        seed: u32,
        abilities_mask: u32,
        intelligence: u32,
        player_level: u32,
        on_victory_chains: Vec<i64>,
        on_failure_chains: Vec<i64>,
        player_name: Option<String>,
        client_ip: Option<std::net::IpAddr>,
        session_instance_id: u64,
    ) -> Option<String> {
        let abilities_mask = abilities_mask & crate::minigame::games::ability_bits(&game_name);
        let ticket = generate_ticket();
        let (cancel, _) = watch::channel(false);
        let session = MinigameSession {
            entity_id,
            player_id,
            session_instance_id,
            game_name,
            difficulty,
            tech_competency,
            seed,
            abilities_mask,
            intelligence,
            player_level,
            ticket: ticket.clone(),
            on_victory_chains,
            on_failure_chains,
            player_name,
            client_ip,
            created_at: Instant::now(),
            connected: false,
            terminal_reported: false,
            cancel,
        };

        let mut inner = self.inner.lock().await;
        // Sweep before the duplicate check. An abandoned launch (SWF never
        // connected) otherwise pins this entity id forever and every later
        // interaction with the same object hits the reject below until the
        // player relogs. Doing it here as well as on the periodic sweep
        // means the *next* interaction is the one that recovers, rather
        // than the player having to wait out a sweep tick.
        expire_pending_locked(&mut inner, PENDING_SESSION_TTL);
        inner.expire_receipts(Instant::now());
        if let Some(existing) = inner.sessions.get(&entity_id) {
            // A second interact while the first launch is still loading is a
            // double click: INFO. A live game, or a launch that has sat past
            // the window, is the #1303 shape: WARN.
            let existing_age = existing.created_at.elapsed();
            let existing_age_ms = u64::try_from(existing_age.as_millis()).unwrap_or(u64::MAX);
            if !existing.connected && existing_age < DOUBLE_INTERACT_WINDOW {
                tracing::info!(
                    entity_id,
                    entity_name = session.player_name.as_deref(),
                    reason = "duplicate_session",
                    existing_connected = existing.connected,
                    existing_age_ms,
                    existing_game = %existing.game_name,
                    requested_game = %session.game_name,
                    "Entity already has an active minigame session"
                );
            } else {
                tracing::warn!(
                    entity_id,
                    entity_name = session.player_name.as_deref(),
                    reason = "duplicate_session",
                    existing_connected = existing.connected,
                    existing_age_ms,
                    existing_game = %existing.game_name,
                    requested_game = %session.game_name,
                    "Entity already has an active minigame session"
                );
            }
            return None;
        }
        inner.boards.insert(
            entity_id,
            PublishedBoard::new(ticket.clone(), session.created_at),
        );
        inner.invalidate_entity_receipts(entity_id);
        inner
            .boards
            .get_mut(&entity_id)
            .unwrap()
            .record("registered", None);
        inner.sessions.insert(entity_id, session);
        Some(ticket)
    }

    /// Publish only for the session that produced this board.
    pub async fn publish_board(&self, entity_id: u32, ticket: &str, snapshot: BoardSnapshot) {
        let mut inner = self.inner.lock().await;
        if inner
            .sessions
            .get(&entity_id)
            .is_some_and(|s| s.ticket == ticket && s.connected)
        {
            if let Some(board) = inner.boards.get_mut(&entity_id) {
                if board.ticket == ticket {
                    if board.snapshot.is_none() {
                        board.record("board_started", None);
                    }
                    board.snapshot = Some(snapshot);
                }
            }
        }
    }

    /// Record selected SmartFox commands and outcome rows without storing packet payloads.
    pub async fn record_event(
        &self,
        entity_id: u32,
        ticket: &str,
        event: &str,
        wire_name: Option<&str>,
    ) {
        let mut inner = self.inner.lock().await;
        if inner
            .sessions
            .get(&entity_id)
            .is_some_and(|s| s.ticket == ticket && s.connected)
        {
            if let Some(board) = inner.boards.get_mut(&entity_id) {
                if board.ticket == ticket {
                    board.record(event, wire_name);
                }
            }
        }
    }

    /// Record a sanitized, ordered SmartFox row for the claimed session.
    pub async fn record_traffic(
        &self,
        entity_id: u32,
        ticket: &str,
        dir: &'static str,
        cmd: &str,
        params: BTreeMap<String, serde_json::Value>,
    ) {
        let mut inner = self.inner.lock().await;
        if inner
            .sessions
            .get(&entity_id)
            .is_some_and(|s| s.ticket == ticket && s.connected)
        {
            if let Some(board) = inner.boards.get_mut(&entity_id) {
                if board.ticket == ticket {
                    board.record_traffic(dir, cmd, params);
                }
            }
        }
    }

    pub async fn board_snapshot(&self, entity_id: u32) -> Option<BoardSnapshot> {
        let mut inner = self.inner.lock().await;
        if inner
            .boards
            .get(&entity_id)
            .is_some_and(|board| board.is_expired(Instant::now()))
        {
            inner.boards.remove(&entity_id);
            return None;
        }
        let board = inner.boards.get(&entity_id)?;
        if let Some(session) = inner.sessions.get(&entity_id) {
            if !session.connected || session.ticket != board.ticket {
                return None;
            }
        } else if board.expires_at.is_none() {
            return None;
        }
        board.read()
    }

    /// Authenticate a login attempt **and claim the matched session**, in one
    /// locked step. Returns the claimed session if the ticket and game name
    /// match.
    ///
    /// This is the only entry point a connection task may use. Validating and
    /// claiming separately is a real race, not a theoretical one: `authenticate`
    /// releases the lock after cloning, and between that and a
    /// `mark_connected(entity_id)` the pending entry can cross
    /// [`PENDING_SESSION_TTL`], be swept, and be replaced by a `register` for
    /// the same entity. The claim would then land on the *replacement*, and the
    /// first connection's teardown would later delete a session it never owned.
    /// Doing both under one lock makes the interleaving unrepresentable: either
    /// the claim wins and the session is connected (so the sweep skips it), or
    /// the sweep wins and this returns an error.
    ///
    /// Does not log: the caller has the peer address and logs the rejection
    /// (see [`ClaimRejection`] for why most of them are not WARN).
    pub async fn authenticate_and_claim(
        &self,
        entity_id: u32,
        password: &str,
        game_name: &str,
    ) -> Result<MinigameSession, ClaimRejection> {
        let mut inner = self.inner.lock().await;
        let session = inner
            .sessions
            .get_mut(&entity_id)
            .ok_or(ClaimRejection::NoSession)?;
        if session.ticket != password {
            return Err(ClaimRejection::TicketMismatch {
                player_name: session.player_name.clone(),
            });
        }
        if session.game_name != game_name {
            return Err(ClaimRejection::GameMismatch {
                player_name: session.player_name.clone(),
                expected: session.game_name.clone(),
            });
        }
        // One live connection per ticket: a second login while the first
        // is still playing would run a second game against the same victory
        // chains. The session leaves the registry when its connection task
        // ends, which needs the server to see the first socket go: a FIN or
        // RST, a failed or timed-out send, TCP keepalive giving up, or the
        // idle timeout. Until one of those, a SWF reconnecting after a
        // half-open drop lands here and is refused.
        if session.connected {
            return Err(ClaimRejection::AlreadyClaimed {
                player_name: session.player_name.clone(),
            });
        }
        session.connected = true;
        let claimed = session.clone();
        if let Some(board) = inner.boards.get_mut(&entity_id) {
            board.record_traffic("in", "login", BTreeMap::new());
            board.record("claimed", None);
        }
        Ok(claimed)
    }

    /// Whether a registered session (pending or connected) records exactly
    /// `ip` (after IPv4-mapped canonicalization) as its client address.
    ///
    /// The listener does not use this: it admits through
    /// [`Self::try_admit`], which treats every non-public address as one
    /// class and counts free tickets. This exact-address check is left over
    /// from the pre-ticket listener (follow-up #1968).
    pub async fn expects_peer(&self, ip: std::net::IpAddr) -> bool {
        let ip = ip.to_canonical();
        let inner = self.inner.lock().await;
        inner
            .sessions
            .values()
            .any(|s| s.client_ip.map(|c| c.to_canonical()) == Some(ip))
    }

    /// How many sessions are registered (pending or connected). Only tests
    /// call it; the listener does not, because [`Self::try_admit`] picks the
    /// refusal reason under its own lock.
    pub async fn session_count(&self) -> usize {
        self.inner.lock().await.sessions.len()
    }

    /// A copy of the entity's registered session, ticket not required.
    ///
    /// Read-only and unauthenticated: for a caller in the same process that
    /// needs to see what the base registered (the base's launch tests read the
    /// player params back through it). A connection task must still go through
    /// [`Self::authenticate_and_claim`].
    #[cfg(any(test, feature = "test-support"))]
    pub async fn peek_session(&self, entity_id: u32) -> Option<MinigameSession> {
        self.inner.lock().await.sessions.get(&entity_id).cloned()
    }

    /// How many registered sessions are still pending, that is, never had
    /// their SWF client connect. The listener does not read it: [`Self::try_admit`]
    /// counts pending sessions under its own lock and reports the number in
    /// [`AdmissionRefusal`]. No caller in the tree uses this one today.
    pub async fn pending_count(&self) -> usize {
        self.inner
            .lock()
            .await
            .sessions
            .values()
            .filter(|s| !s.connected)
            .count()
    }

    /// Validate a ticket without claiming the session.
    ///
    /// Read-only: useful for asserting registry state in tests. A connection
    /// task must use [`Self::authenticate_and_claim`] instead — see the race
    /// documented there.
    pub async fn authenticate(
        &self,
        entity_id: u32,
        password: &str,
        game_name: &str,
    ) -> Option<MinigameSession> {
        let inner = self.inner.lock().await;
        let session = inner.sessions.get(&entity_id)?;
        if session.ticket != password {
            tracing::warn!(
                entity_id,
                entity_name = session.player_name.as_deref(),
                "Minigame ticket mismatch"
            );
            return None;
        }
        if session.game_name != game_name {
            tracing::warn!(
                entity_id,
                entity_name = session.player_name.as_deref(),
                expected = %session.game_name,
                got = %game_name,
                "Minigame game name mismatch",
            );
            return None;
        }
        Some(session.clone())
    }

    /// Remove a session unconditionally.
    ///
    /// Prefer [`Self::remove_if_ticket`] from a connection task — see the
    /// race it closes. This stays as the blunt primitive for a caller that
    /// genuinely wants the entity's session gone whatever it is.
    pub async fn remove(&self, entity_id: u32) {
        let mut inner = self.inner.lock().await;
        if let Some(session) = inner.sessions.remove(&entity_id) {
            // The session is gone, so a connection bound to it holds no ticket.
            inner.release_bound_slot(entity_id, &session.ticket);
        }
        inner.boards.remove(&entity_id);
        inner.invalidate_entity_receipts(entity_id);
    }

    /// Cancel the entity's current session and release its registration.
    /// The connection task keeps its ticket and cancel receiver, so its
    /// teardown cannot erase a replacement session registered immediately.
    pub async fn cancel(&self, entity_id: u32) -> bool {
        self.cancel_reporting_connected(entity_id).await.is_some()
    }

    /// [`Self::cancel`], also saying whether a connection task exists.
    /// `None` when no session was registered. `Some(false)` means the SWF
    /// never connected, so no task will report a result and the caller must
    /// tell the cell itself (the TTL sweep cannot see a removed session).
    pub async fn cancel_reporting_connected(&self, entity_id: u32) -> Option<bool> {
        let mut inner = self.inner.lock().await;
        let session = inner.sessions.remove(&entity_id);
        if let Some(ref session) = session {
            session.cancel.send_replace(true);
            // Release on cancel (owner ruling): the old connection may still
            // be draining, but its ticket is gone and must not hold a slot.
            inner.release_bound_slot(entity_id, &session.ticket);
        }
        inner.boards.remove(&entity_id);
        inner.invalidate_entity_receipts(entity_id);
        session.map(|session| session.connected)
    }

    /// Cancel only the launch that minted this ticket. A stale failed send
    /// cannot erase a replacement board for the same entity.
    pub async fn cancel_if_ticket(&self, entity_id: u32, ticket: &str) -> bool {
        let mut inner = self.inner.lock().await;
        let matches_session = inner
            .sessions
            .get(&entity_id)
            .is_some_and(|session| session.ticket == ticket);
        let matches_receipt = inner
            .receipts
            .get(ticket)
            .is_some_and(|receipt| receipt.entity_id == entity_id);
        if !matches_session && !matches_receipt {
            return false;
        }
        if matches_session {
            if let Some(session) = inner.sessions.remove(&entity_id) {
                session.cancel.send_replace(true);
                inner.release_bound_slot(entity_id, &session.ticket);
            }
        }
        if inner
            .boards
            .get(&entity_id)
            .is_some_and(|board| board.ticket == ticket)
        {
            inner.boards.remove(&entity_id);
        }
        inner.invalidate_ticket_receipt(ticket);
        true
    }

    /// Whether the current registration belongs to this SmartFox ticket.
    /// An absent registration is not a replacement; a different ticket is.
    pub async fn has_replacement_ticket(&self, entity_id: u32, ticket: &str) -> bool {
        self.inner
            .lock()
            .await
            .sessions
            .get(&entity_id)
            .is_some_and(|session| session.ticket != ticket)
    }

    /// Remove a session only if it is still the one that minted `ticket`.
    /// Returns whether it was removed.
    ///
    /// The connection task's teardown must not delete a session it does not
    /// own. Sequence that makes it matter: a session outlives
    /// [`PENDING_SESSION_TTL`] without being marked connected (a bug, or a
    /// future code path that skips the claim), the sweep drops it,
    /// the player interacts again and `register` mints a *second* session
    /// for the same entity id — and then the first task finishes and its
    /// unconditional `remove` deletes the second one, leaving the live
    /// minigame with no registry entry. Keying the delete on the ticket,
    /// which is 64 hex chars of CSPRNG output per registration, makes a
    /// stale task's teardown a no-op instead.
    pub async fn remove_if_ticket(&self, entity_id: u32, ticket: &str) -> bool {
        let mut inner = self.inner.lock().await;
        match inner.sessions.get(&entity_id) {
            Some(session) if session.ticket == ticket => {
                inner.sessions.remove(&entity_id);
                inner.release_bound_slot(entity_id, ticket);
                if let Some(board) = inner.boards.get_mut(&entity_id) {
                    if board.ticket == ticket && board.snapshot.is_some() {
                        board.finish();
                    } else {
                        inner.boards.remove(&entity_id);
                    }
                }
                true
            }
            Some(newer) => {
                // Not an error the player sees, but it means two tasks
                // overlapped on one entity — worth a line if it ever fires.
                tracing::warn!(
                    entity_id,
                    entity_name = newer.player_name.as_deref(),
                    "Minigame: stale connection task tried to unregister a \
                     newer session; leaving it in place"
                );
                false
            }
            None => false,
        }
    }

    /// Allocate a unique room ID.
    pub async fn allocate_room_id(&self) -> u32 {
        let mut inner = self.inner.lock().await;
        let id = inner.next_room_id;
        inner.next_room_id += 1;
        id
    }
}

/// Generate a 64-character hex ticket (matching C++ implementation).
fn generate_ticket() -> String {
    const HEX: &[u8] = b"0123456789ABCDEF";
    let mut rng = rand::rng();
    (0..64)
        .map(|_| HEX[rng.random_range(0..16)] as char)
        .collect()
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "session_sweep_tests.rs"]
mod sweep_tests;

#[cfg(test)]
#[path = "session_result_tests.rs"]
mod result_tests;

#[cfg(test)]
#[path = "session_cancel_tests.rs"]
mod cancel_tests;

#[cfg(test)]
#[path = "session_board_tests.rs"]
mod board_tests;

#[cfg(test)]
#[path = "session_mask_tests.rs"]
mod mask_tests;
