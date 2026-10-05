//! BaseApp service -- Mercury UDP listener for persistent entity state
//! and client connections.
//!
//! See `docs/protocol/login-handshake.md` for the full wire-level spec.
//!
//! `BaseService`, the connect loop, login, the SGWPlayer base-method dispatch
//! and the character creator are the code this crate holds. The other names
//! here are the session-layer modules and types (`cimmeria-base-session`), the
//! resource cache and CharDef table (`cimmeria-resources`) and world entry and
//! the character list (`cimmeria-base-world-entry`) that code reaches through
//! `crate::base::…` and `super::…` paths, re-exported privately so those paths
//! compile unchanged.

// ── Submodules ───────────────────────────────────────────────────────────────

pub(crate) mod character_create;
pub(crate) mod connect_loop;
pub(crate) mod dispatch;
pub(crate) mod login;
mod service;

#[cfg(test)]
mod smoke_tests;

pub use service::BaseService;

pub(crate) use cimmeria_base_methods::base::world_entry::methods::black_market;
pub(crate) use cimmeria_base_session::base::{
    archetype_name, contact_list, cooked_data, cooked_sync, feedback, helpers, mutes, outbox,
    player_index, rate_limit, session_identity, tick_sync, BaseError, ConnectedClientState,
    OnlinePlayer,
};
pub(crate) use cimmeria_base_world_entry::base::{character, world_entry};
pub(crate) use cimmeria_resources::base::{chardef, resources};

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
            ..ServerConfig::loopback()
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
