//! CellApp service, under the `cell::` path it had in `cimmeria-services`.
//!
//! This crate holds the service itself (`service`: `CellService`, the cell
//! loop, the base-message handlers and the ticks) and the cell-method router
//! (`dispatch`). Every system they drive is in a crate below this one, and is
//! imported here privately at the `cell::` path the moved code names it by.
//! `cimmeria-services`' `cell` module re-exports `dispatch` and `CellService`
//! at the same paths, beside the lower crates' modules.

pub mod dispatch;
mod service;

pub use service::CellService;

/// The content tests that drive the relog hydration,
/// `service::base_messages::player_init::mission_restore`. Test-only.
#[cfg(test)]
mod content_tests;

// Lower crates, at the `cell::` paths the moved code names them by.
pub(crate) use cimmeria_cell_catalog::cell::spawner;
pub(crate) use cimmeria_cell_combat::cell::{abilities, combat, effects};
pub(crate) use cimmeria_cell_console::cell::console::{self, chat};
pub(crate) use cimmeria_cell_content::cell::{content, missions, ring_transport};
pub(crate) use cimmeria_cell_interactions::cell::{gate_travel, interactions, respawn};
pub(crate) use cimmeria_cell_methods::cell::cell_methods;
pub(crate) use cimmeria_cell_world::cell::{cover, playtest_friction, space_manager, CellError};
pub(crate) use cimmeria_wire::cell::{client_methods, messages, player_journal};

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_common::ServerConfig;

    #[test]
    fn new_service_is_not_running() {
        let config = ServerConfig::default();
        let svc = CellService::new(&config);
        assert!(!svc.is_running);
        assert_eq!(svc.listener_addr.port(), 50000);
    }

    #[tokio::test]
    async fn start_sets_running() {
        let config = ServerConfig::default();
        let mut svc = CellService::new(&config);
        svc.start().await.unwrap();
        assert!(svc.is_running);
    }
}
