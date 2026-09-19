//! Minigame session registry and room management.
//!
//! Tracks pending minigame sessions (ticket → game params) and active rooms
//! for spectator/helper broadcasting.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use rand::RngExt;
use tokio::sync::Mutex;
use tokio::time::Instant;

/// How long a session may sit in the registry after `register` before the
/// sweep drops it, *if its SWF client never connected*.
///
/// **This TTL is a backstop, not a port.** The original had no timeout of
/// any kind: `MinigameRequestManager::QueueEntry`
/// (`deprecated/cpp/src/baseapp/minigame.hpp`) carries no timestamp, and
/// entries left the queue only through an explicit remove or cancel. Its
/// recovery path was two client-driven RPCs instead —
/// `endMinigameForPlayer` (base) and `minigameStartCancel` (cell method
/// 30, `MINIGAME_RESULT_NotStarted`). Cimmeria implements neither; cell
/// method 30 still logs `UNIMPLEMENTED`. Wiring that RPC through to
/// [`SessionRegistry::remove`] is the faithful fix, and this TTL remains
/// useful afterwards for the case the original had no answer to either: a
/// client that crashes without sending anything.
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

/// A registered minigame session awaiting client connection.
#[derive(Debug, Clone)]
pub struct MinigameSession {
    pub entity_id: u32,
    pub player_id: i32,
    pub game_name: String,
    pub difficulty: u32,
    pub tech_competency: u32,
    pub seed: u32,
    pub abilities_mask: u32,
    pub intelligence: u32,
    pub player_level: u32,
    pub ticket: String,
    pub on_victory_chains: Vec<i64>,
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
}

/// Thread-safe session registry shared between BaseApp and the minigame TCP server.
#[derive(Clone)]
pub struct SessionRegistry {
    inner: Arc<Mutex<SessionRegistryInner>>,
}

struct SessionRegistryInner {
    /// entity_id → session
    sessions: HashMap<u32, MinigameSession>,
    /// Next room ID (auto-incrementing from 1001)
    next_room_id: u32,
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SessionRegistryInner {
                sessions: HashMap::new(),
                next_room_id: 1001,
            })),
        }
    }

    /// Register a new minigame session. Returns the generated ticket.
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
    ) -> Option<String> {
        let ticket = generate_ticket();
        let session = MinigameSession {
            entity_id,
            player_id,
            game_name,
            difficulty,
            tech_competency,
            seed,
            abilities_mask,
            intelligence,
            player_level,
            ticket: ticket.clone(),
            on_victory_chains,
            created_at: Instant::now(),
            connected: false,
        };

        let mut inner = self.inner.lock().await;
        // Sweep before the duplicate check. An abandoned launch (SWF never
        // connected) otherwise pins this entity id forever and every later
        // interaction with the same object hits the reject below until the
        // player relogs. Doing it here as well as on the periodic sweep
        // means the *next* interaction is the one that recovers, rather
        // than the player having to wait out a sweep tick.
        expire_pending_locked(&mut inner, PENDING_SESSION_TTL);
        if inner.sessions.contains_key(&entity_id) {
            tracing::warn!(entity_id, "Entity already has an active minigame session");
            return None;
        }
        inner.sessions.insert(entity_id, session);
        Some(ticket)
    }

    /// Drop every registered-but-never-connected session older than `ttl`.
    /// Returns the evicted entity ids.
    pub async fn expire_pending(&self, ttl: Duration) -> Vec<u32> {
        let mut inner = self.inner.lock().await;
        expire_pending_locked(&mut inner, ttl)
    }

    /// Spawn the background expiry sweep. Called once from
    /// [`crate::minigame::server::run`]; the task lives as long as the
    /// process.
    pub fn spawn_sweep(&self, ttl: Duration, interval: Duration) {
        let registry = self.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            // The first `tick()` completes immediately; burn it so the
            // first real sweep happens one interval in, not at startup
            // (where there is nothing to sweep anyway).
            ticker.tick().await;
            loop {
                ticker.tick().await;
                registry.expire_pending(ttl).await;
            }
        });
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
    /// the sweep wins and this returns `None`.
    pub async fn authenticate_and_claim(
        &self,
        entity_id: u32,
        password: &str,
        game_name: &str,
    ) -> Option<MinigameSession> {
        let mut inner = self.inner.lock().await;
        let session = inner.sessions.get_mut(&entity_id)?;
        if session.ticket != password {
            tracing::warn!(entity_id, "Minigame ticket mismatch");
            return None;
        }
        if session.game_name != game_name {
            tracing::warn!(
                entity_id,
                expected = %session.game_name,
                got = %game_name,
                "Minigame game name mismatch"
            );
            return None;
        }
        session.connected = true;
        Some(session.clone())
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
            tracing::warn!(entity_id, "Minigame ticket mismatch");
            return None;
        }
        if session.game_name != game_name {
            tracing::warn!(
                entity_id,
                expected = %session.game_name,
                got = %game_name,
                "Minigame game name mismatch"
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
        inner.sessions.remove(&entity_id);
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
                true
            }
            Some(_) => {
                // Not an error the player sees, but it means two tasks
                // overlapped on one entity — worth a line if it ever fires.
                tracing::warn!(
                    entity_id,
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

/// Expiry pass over an already-locked registry.
///
/// Split out so [`SessionRegistry::register`] can sweep inside the lock it
/// already holds without re-entering `Mutex::lock` (which would deadlock on
/// the non-reentrant `tokio::sync::Mutex`).
fn expire_pending_locked(inner: &mut SessionRegistryInner, ttl: Duration) -> Vec<u32> {
    let now = Instant::now();
    let expired: Vec<u32> = inner
        .sessions
        .iter()
        .filter(|(_, s)| !s.connected && now.saturating_duration_since(s.created_at) >= ttl)
        .map(|(id, _)| *id)
        .collect();

    for entity_id in &expired {
        if let Some(session) = inner.sessions.remove(entity_id) {
            // info!, not warn!: an abandoned launch is a player closing a
            // window, not a server fault. It is logged at all because a
            // "minigame won't start" report correlates directly with
            // whether this line fired for that entity.
            tracing::info!(
                entity_id = *entity_id,
                game = %session.game_name,
                ttl_secs = ttl.as_secs(),
                "Minigame: expiring session whose client never connected"
            );
        }
    }
    expired
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
mod tests;
