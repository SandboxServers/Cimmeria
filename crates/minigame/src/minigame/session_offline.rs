//! An offline minigame session: one built without a base ticket or a client
//! connection, so a game can be driven in-process (tests and the REPL).

use tokio::sync::watch;
use tokio::time::Instant;

use super::session::MinigameSession;

impl MinigameSession {
    /// A session for `game_name` at `difficulty`, seeded with `seed`, with the
    /// player's tech competency and level fixed at 1.
    pub fn offline(game_name: &str, difficulty: u32, seed: u32) -> MinigameSession {
        MinigameSession {
            entity_id: 1,
            player_id: 100,
            game_name: game_name.to_string(),
            difficulty,
            tech_competency: 1,
            seed,
            abilities_mask: 0,
            intelligence: 0,
            player_level: 1,
            ticket: String::new(),
            on_victory_chains: vec![],
            on_failure_chains: vec![],
            player_name: None,
            client_ip: None,
            created_at: Instant::now(),
            connected: false,
            cancel: watch::channel(false).0,
        }
    }
}
