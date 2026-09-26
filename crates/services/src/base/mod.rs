//! BaseApp service -- Mercury UDP listener for persistent entity state
//! and client connections.
//!
//! See `docs/protocol/login-handshake.md` for the full wire-level spec.
//!
//! The per-connection session state (`ConnectedClientState`, `OnlinePlayer`,
//! ...) and the session-layer modules are in `cimmeria-base-session` (wave B1
//! of docs/architecture/services-crate-split.md), and world entry and the
//! character list in `cimmeria-base-world-entry` (wave B3), re-exported below
//! at their old paths.

// ── Submodules ───────────────────────────────────────────────────────────────

pub(crate) mod character_create;
pub(crate) mod connect_loop;
pub(crate) mod dispatch;
pub(crate) mod login;
mod service;

#[cfg(test)]
mod smoke_tests;

// Split out to `cimmeria-resources` (docs/architecture/services-crate-split.md,
// wave W1b) and re-exported at their old paths.
pub use cimmeria_resources::base::{
    chardef, dialog_overrides, item_overrides, mission_overrides, resources, sequence_overrides,
};

// Split out to `cimmeria-base-session` (wave B1) and re-exported at their old
// paths, with their old visibility. `gm_feedback` is not: its only users here
// were the feature handlers, which moved to `cimmeria-base-methods` (B2). Nor
// are `cinematic_aoi_hold`, `console_authoring`, `crafting`, `deferred_aoi`,
// `deferred_aoi_lifecycle`, `session_identity` and `world_entry_chat`: world
// entry was their only user here, and it moved to `cimmeria-base-world-entry`
// (B3). `gm_spawn` and `PendingClientReadyInfo` are left only for tests.
pub(crate) use cimmeria_base_session::base::{
    archetype_name, contact_list, cooked_data, helpers, outbox, tick_sync, ConnectedClientState,
};
#[cfg(test)]
pub(crate) use cimmeria_base_session::base::{gm_spawn, PendingClientReadyInfo};
pub use cimmeria_base_session::base::{BaseError, OnlinePlayer};

// Split out to `cimmeria-base-world-entry` (wave B3) and re-exported at their
// old paths, with their old visibility, for the connect loop, the character
// creator and `BaseService`. `world_entry_appearance` is not: the connect loop
// reaches its two handlers through `world_entry`, and nothing else here used it.
pub(crate) use cimmeria_base_world_entry::base::{character, world_entry};

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
