//! Minigame session registry and room management.
//!
//! Tracks pending minigame sessions (ticket → game params) and active rooms
//! for spectator/helper broadcasting.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use rand::Rng;
use tokio::sync::Mutex;

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
    pub created_at: Instant,
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
        };

        let mut inner = self.inner.lock().await;
        if inner.sessions.contains_key(&entity_id) {
            tracing::warn!(entity_id, "Entity already has an active minigame session");
            return None;
        }
        inner.sessions.insert(entity_id, session);
        Some(ticket)
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
}
