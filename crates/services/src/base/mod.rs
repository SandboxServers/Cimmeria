//! BaseApp service -- Mercury UDP listener for persistent entity state
//! and client connections.
//!
//! See `docs/protocol/login-handshake.md` for the full wire-level spec.

use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cimmeria_common::EntityId;
use cimmeria_mercury::encryption::MercuryEncryption;

use serde::Serialize;

use crate::mercury::{PlayerLoadData, WorldEntryInfo};

// ── Submodules ───────────────────────────────────────────────────────────────

pub(crate) mod character;
pub(crate) mod character_create;
pub(crate) mod chardef;
pub(crate) mod connect_loop;
pub(crate) mod cooked_data;
pub(crate) mod dispatch;
pub(crate) mod helpers;
pub(crate) mod login;
pub(crate) mod outbox;
pub(crate) mod resources;
mod service;
pub(crate) mod tick_sync;
pub(crate) mod world_entry;
pub(crate) mod world_entry_appearance;

pub use service::BaseService;

// ── Error types ───────────────────────────────────────────────────────────────

/// Errors specific to the base service.
#[derive(Debug, thiserror::Error)]
pub enum BaseError {
    #[error("Entity {0} not found")]
    EntityNotFound(EntityId),

    #[error("Entity creation failed: {0}")]
    CreationFailed(String),

    #[error("Service not running")]
    NotRunning,

    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
}

// ── Public snapshot for admin API ─────────────────────────────────────────────

/// A snapshot of one connected player, safe to serialize for the admin API.
#[derive(Debug, Clone, Serialize)]
pub struct OnlinePlayer {
    pub id: u32,
    pub name: String,
    pub archetype: &'static str,
    pub level: i32,
    pub zone: String,
    pub ping: Option<u32>,
    pub status: &'static str,
    pub session: String,
}

fn archetype_name(id: i32) -> &'static str {
    match id {
        1 => "Soldier",
        2 => "Commando",
        3 => "Scientist",
        4 => "Archaeologist",
        5 => "Asgard",
        6 => "Goa'uld",
        7 => "Jaffa",
        _ => "Unknown",
    }
}

// ── Per-connection state ──────────────────────────────────────────────────────

/// Deferred world-entry finalization that must wait for `SGWPlayer.onClientReady`.
pub(crate) struct PendingClientReadyInfo {
    pub entity_id: u32,
    pub player_id: i32,
    pub world_name: String,
    pub appearance_args: Vec<u8>,
    pub tint_args: Vec<u8>,
}

/// State held for each client that has completed the Phase 3 handshake.
pub(crate) struct ConnectedClientState {
    pub enc: MercuryEncryption,
    pub key: [u8; 32],
    pub account_id: u32,
    #[allow(dead_code)]
    pub access_level: u32,
    pub char_list_sent: bool,
    pub world_entry_sent: bool,
    pub pending_player_entity_id: Option<u32>,
    pub player_entity_id: Option<u32>,
    pub next_seq: Arc<AtomicU32>,
    pub pending_acks: Arc<Mutex<Vec<u32>>>,
    pub last_recv: Arc<Mutex<Instant>>,
    pub account_entity_id: u32,
    pub next_data_id: u16,
    pub pending_world_entry: Option<WorldEntryInfo>,
    pub pending_player_load_data: Option<PlayerLoadData>,
    pub pending_map_loaded: Option<WorldEntryInfo>,
    pub pending_client_ready: Option<PendingClientReadyInfo>,
    pub cached_appearance_args: Option<Vec<u8>>,
    pub cached_tint_args: Option<Vec<u8>>,
    pub cancelled: Arc<AtomicBool>,
    pub player_name: Option<String>,
    pub player_level: Option<i32>,
    pub player_archetype: Option<i32>,
    pub world_name: Option<String>,
    pub player_xp: Option<u64>,
    pub player_training_points: Option<u32>,
    /// The DB `player_id` of the character the client selected via
    /// `playCharacter`. Set during the play-character flow and used by
    /// gate-travel / respawn so they target the active character instead
    /// of falling back to "lowest player_id for the account" — which is
    /// wrong on multi-character accounts.
    pub active_player_id: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mercury::SKIN_TINTS;
    use cimmeria_common::ServerConfig;

    #[test]
    fn new_service_is_not_running() {
        let config = ServerConfig::default();
        let svc = BaseService::new(&config);
        assert!(!svc.is_running);
        assert_eq!(svc.listener_addr.port(), 32832);
    }

    #[tokio::test]
    async fn start_sets_running() {
        let config = ServerConfig {
            base_port: 0,
            ..ServerConfig::default()
        };
        let mut svc = BaseService::new(&config);
        svc.start().await.unwrap();
        assert!(svc.is_running);
    }

    #[tokio::test]
    async fn create_entity_fails_when_not_running() {
        let config = ServerConfig::default();
        let svc = BaseService::new(&config);
        let result = svc.create_base_entity().await;
        assert!(result.is_err());
    }

    #[test]
    fn skin_tints_array_length() {
        assert_eq!(SKIN_TINTS.len(), 16);
    }

    #[test]
    fn skin_tints_all_nonzero() {
        for (i, &tint) in SKIN_TINTS.iter().enumerate() {
            assert_ne!(tint, 0, "SKIN_TINTS[{i}] should not be zero");
        }
    }

    #[test]
    fn skin_tints_index_0_matches_python() {
        assert_eq!(SKIN_TINTS[0], 0x2F1308FF);
    }
}
