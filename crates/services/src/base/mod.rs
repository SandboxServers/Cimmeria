//! BaseApp service -- Mercury UDP listener for persistent entity state
//! and client connections.
//!
//! See `docs/protocol/login-handshake.md` for the full wire-level spec.
//!
//! The per-connection session state (`ConnectedClientState`, `OnlinePlayer`,
//! ...) and the session-layer modules are in `cimmeria-base-session` (wave B1
//! of docs/architecture/services-crate-split.md), re-exported below at their
//! old paths.

// ── Submodules ───────────────────────────────────────────────────────────────

pub(crate) mod character;
pub(crate) mod character_create;
pub(crate) mod connect_loop;
pub(crate) mod dispatch;
pub(crate) mod login;
mod service;
pub(crate) mod world_entry;
pub(crate) mod world_entry_appearance;

#[cfg(test)]
mod smoke_tests;

// Split out to `cimmeria-resources` (docs/architecture/services-crate-split.md,
// wave W1b) and re-exported at their old paths.
pub use cimmeria_resources::base::{
    chardef, dialog_overrides, item_overrides, mission_overrides, resources, sequence_overrides,
};

// Split out to `cimmeria-base-session` (wave B1) and re-exported at their old
// paths, with their old visibility.
pub(crate) use cimmeria_base_session::base::{
    archetype_name, cinematic_aoi_hold, console_authoring, contact_list, cooked_data, crafting,
    deferred_aoi, deferred_aoi_lifecycle, gm_feedback, gm_spawn, helpers, outbox, session_identity,
    tick_sync, world_entry_chat, ConnectedClientState, PendingClientReadyInfo,
};
pub use cimmeria_base_session::base::{BaseError, OnlinePlayer};

pub use service::BaseService;

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
