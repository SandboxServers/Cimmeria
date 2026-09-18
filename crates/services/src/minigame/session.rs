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
    /// Set by [`SessionRegistry::mark_connected`] once the SWF has
    /// authenticated and its connection task has taken ownership of the
    /// session. A connected session is exempt from age-based expiry: the
    /// connection task removes it when the socket closes, and a long
    /// Livewire round can easily outlive [`PENDING_SESSION_TTL`].
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

    /// Mark a session as owned by a live connection task, exempting it from
    /// age-based expiry. Idempotent; a no-op if the session is already gone.
    pub async fn mark_connected(&self, entity_id: u32) {
        let mut inner = self.inner.lock().await;
        if let Some(session) = inner.sessions.get_mut(&entity_id) {
            session.connected = true;
        }
    }

    /// Authenticate a login attempt. Returns the session if valid.
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

    /// Remove a session (called on game completion or disconnect).
    pub async fn remove(&self, entity_id: u32) {
        let mut inner = self.inner.lock().await;
        inner.sessions.remove(&entity_id);
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
mod tests {
    use super::*;

    #[tokio::test]
    async fn register_and_authenticate() {
        let reg = SessionRegistry::new();
        let ticket = reg
            .register(42, 1, "Livewire".into(), 1, 50, 12345, 0, 10, 5, vec![1017])
            .await
            .unwrap();

        let session = reg.authenticate(42, &ticket, "Livewire").await.unwrap();
        assert_eq!(session.entity_id, 42);
        assert_eq!(session.difficulty, 1);
    }

    #[tokio::test]
    async fn wrong_ticket_fails() {
        let reg = SessionRegistry::new();
        reg.register(42, 1, "Livewire".into(), 1, 50, 0, 0, 0, 1, vec![])
            .await;

        assert!(reg.authenticate(42, "WRONG", "Livewire").await.is_none());
    }

    #[tokio::test]
    async fn wrong_game_name_fails() {
        let reg = SessionRegistry::new();
        let ticket = reg
            .register(42, 1, "Livewire".into(), 1, 50, 0, 0, 0, 1, vec![])
            .await
            .unwrap();

        assert!(reg.authenticate(42, &ticket, "Alignment").await.is_none());
    }

    #[tokio::test]
    async fn duplicate_session_rejected() {
        let reg = SessionRegistry::new();
        reg.register(42, 1, "Livewire".into(), 1, 50, 0, 0, 0, 1, vec![])
            .await
            .unwrap();

        assert!(reg
            .register(42, 1, "Livewire".into(), 1, 50, 0, 0, 0, 1, vec![])
            .await
            .is_none());
    }

    #[tokio::test]
    async fn remove_allows_re_register() {
        let reg = SessionRegistry::new();
        reg.register(42, 1, "Livewire".into(), 1, 50, 0, 0, 0, 1, vec![])
            .await
            .unwrap();

        reg.remove(42).await;

        assert!(reg
            .register(42, 1, "Livewire".into(), 1, 50, 0, 0, 0, 1, vec![])
            .await
            .is_some());
    }

    #[test]
    fn ticket_is_64_hex_chars() {
        let ticket = generate_ticket();
        assert_eq!(ticket.len(), 64);
        assert!(ticket.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // ── Session expiry (defect B4) ───────────────────────────────────────
    //
    // `start_paused = true` freezes `tokio::time::Instant`, which is what
    // `created_at` is measured against, so `advance()` moves the TTL
    // without a wall-clock sleep. A `std::time::Instant` would be immune
    // to `advance()` and these tests would have to sleep for real.

    /// Register a Livewire session for `entity_id` with no victory chains.
    async fn register_livewire(reg: &SessionRegistry, entity_id: u32) -> Option<String> {
        reg.register(entity_id, 1, "Livewire".into(), 1, 50, 0, 0, 0, 1, vec![])
            .await
    }

    /// A session whose SWF never connected must be evicted once it is older
    /// than the TTL.
    #[tokio::test(start_paused = true)]
    async fn pending_session_expires_after_ttl() {
        let reg = SessionRegistry::new();
        register_livewire(&reg, 42).await.unwrap();

        tokio::time::advance(PENDING_SESSION_TTL + Duration::from_secs(1)).await;

        assert_eq!(
            reg.expire_pending(PENDING_SESSION_TTL).await,
            vec![42],
            "a never-connected session older than the TTL must be evicted",
        );
    }

    /// A session that is still inside the TTL must survive the sweep — the
    /// player may simply be waiting on a slow SWF load.
    #[tokio::test(start_paused = true)]
    async fn fresh_pending_session_survives_the_sweep() {
        let reg = SessionRegistry::new();
        let ticket = register_livewire(&reg, 42).await.unwrap();

        tokio::time::advance(PENDING_SESSION_TTL / 2).await;
        assert!(
            reg.expire_pending(PENDING_SESSION_TTL).await.is_empty(),
            "a session half-way to the TTL must not be swept",
        );
        assert!(
            reg.authenticate(42, &ticket, "Livewire").await.is_some(),
            "the surviving session must still authenticate",
        );
    }

    /// A session owned by a live connection task must never be expired by
    /// age. A Livewire round can run longer than the TTL, and sweeping it
    /// would let a second interaction register a *concurrent* session for
    /// the same entity.
    #[tokio::test(start_paused = true)]
    async fn connected_session_is_never_expired_by_age() {
        let reg = SessionRegistry::new();
        register_livewire(&reg, 42).await.unwrap();
        reg.mark_connected(42).await;

        tokio::time::advance(PENDING_SESSION_TTL * 10).await;

        assert!(
            reg.expire_pending(PENDING_SESSION_TTL).await.is_empty(),
            "a connected session must be exempt from age-based expiry; it is \
             removed by its connection task when the socket closes",
        );
    }

    /// **Defect B4 regression guard.** The reported symptom: a player
    /// launches a minigame, the SWF never connects (window closed, Flash
    /// blocked, whatever), and every later interaction with the same object
    /// is rejected as a duplicate until relog.
    ///
    /// Fails with the `expire_pending_locked` call removed from
    /// `register`: the stale session is still in the map, the
    /// `contains_key` reject fires, and the second register returns `None`.
    #[tokio::test(start_paused = true)]
    async fn second_interaction_succeeds_after_an_abandoned_launch() {
        let reg = SessionRegistry::new();
        let first = register_livewire(&reg, 42).await;
        assert!(first.is_some(), "the first launch must register");

        // The SWF never connects. The player walks away, comes back.
        tokio::time::advance(PENDING_SESSION_TTL + Duration::from_secs(1)).await;

        let second = register_livewire(&reg, 42).await;
        assert!(
            second.is_some(),
            "re-interacting after an abandoned launch must mint a fresh \
             ticket, not hit the duplicate-session reject",
        );
        assert_ne!(
            first.unwrap(),
            second.clone().unwrap(),
            "the second launch must mint a NEW ticket — reusing the \
             abandoned one would let a late SWF from the first launch \
             authenticate against the second session",
        );
        assert!(
            reg.authenticate(42, &second.unwrap(), "Livewire")
                .await
                .is_some(),
            "the replacement session must be the one in the registry",
        );
    }

    /// The duplicate reject must still hold *inside* the TTL — that is what
    /// stops a click-spamming player opening two concurrent sessions on one
    /// entity. The B4 fix must not widen into "duplicates are fine".
    #[tokio::test(start_paused = true)]
    async fn duplicate_register_still_rejected_inside_the_ttl() {
        let reg = SessionRegistry::new();
        register_livewire(&reg, 42).await.unwrap();

        tokio::time::advance(PENDING_SESSION_TTL / 2).await;

        assert!(
            register_livewire(&reg, 42).await.is_none(),
            "a second launch while the first is still within its TTL must \
             still be rejected",
        );
    }

    /// The sweep must be entity-scoped: expiring one player's abandoned
    /// session must not disturb another player's pending one. Staggering
    /// the two registrations by half the TTL puts them on opposite sides
    /// of the cutoff at sweep time.
    #[tokio::test(start_paused = true)]
    async fn sweep_only_evicts_the_stale_entity() {
        let half = PENDING_SESSION_TTL / 2;
        let reg = SessionRegistry::new();
        register_livewire(&reg, 42).await.unwrap();

        tokio::time::advance(half + Duration::from_secs(1)).await;
        let fresh = register_livewire(&reg, 99).await.unwrap();

        // 42 is now TTL+2s old, 99 is half+1s old.
        tokio::time::advance(half + Duration::from_secs(1)).await;

        assert_eq!(
            reg.expire_pending(PENDING_SESSION_TTL).await,
            vec![42],
            "only the entity past the TTL may be evicted",
        );
        assert!(
            reg.authenticate(99, &fresh, "Livewire").await.is_some(),
            "the younger session must be untouched by the sweep",
        );
    }
}
