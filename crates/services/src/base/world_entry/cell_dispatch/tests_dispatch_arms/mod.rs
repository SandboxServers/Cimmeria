//! Per-arm dispatch tests for `handle_cell_message`.
//!
//! Sibling to `tests.rs` (which covers `flush_deferred_aoi`, bandolier
//! validation, system_options, and the helper-glue regression guards).
//! This module pins the routing decisions and the pre-`onClientReady`
//! defer gates that the dispatcher applies *before* delegating to the
//! per-family handler — those gates are the load-bearing piece that
//! turns the deferred-AoI buffer from "nice to have" into "correct".
//!
//! Bug shapes pinned:
//!
//! 1. Message routed to wrong handler — assert the downstream effect
//!    of the correct arm (transport packets, cell_tx receive, log line).
//! 2. Defer gate skipped on an AoI-bearing arm — pre-ready traffic must
//!    buffer (or drop, for `EntityMoved`), not hit the wire. Both
//!    branches are tested.
//! 3. Witness-cardinality on broadcast arms — `WitnessEntityMethod`
//!    and `EntityInvisible` must send exactly one packet to exactly
//!    one address.
//! 4. Silent error paths — `GateTravel` and `ReanchorPlayer` wrap
//!    fallible handlers in `if let Err(e) = ...` and log via
//!    `tracing::error!`. `LogCapture` pins the level + message
//!    substring so the negative log can't silently downgrade.
//!
//! Submodule layout — one file per natural seam:
//!
//! - [`aoi_defer_gate`]    — `EnteredAoI` / `LeftAoI` / `EntityMoved` /
//!   `EntityMethodCall` / `EntityMethodCallBatch` pre/post-ready behavior.
//! - [`fallible_handlers`] — `GateTravel` / `ReanchorPlayer` error-log
//!   seams.
//! - [`passthrough`]       — `SpaceData` / `MissionUpdate` / `MailRequest`
//!   / `GrantXP` / `TeleportPlayer` routing (handler short-circuits on
//!   no-pool / no-addr — pinned via `LogCapture`).
//! - [`witness_broadcast`] — `WitnessEntityMethod` / `EntityInvisible`
//!   single-witness fan-out byte tests.

use super::*;
use crate::base::PendingClientReadyInfo;
use crate::test_support::test_default_connected_client_state;

mod aoi_defer_gate;
mod fallible_handlers;
mod passthrough;
mod witness_broadcast;

/// Empty maps shared by routing tests that don't need a session installed.
pub(super) fn empty_maps() -> (
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    (
        Arc::new(Mutex::new(HashMap::new())),
        Arc::new(Mutex::new(HashMap::new())),
    )
}

/// Build a `connected` map + `entity_to_addr` map with one session for
/// `entity_id` at a deterministic addr. The session's pre-ready state is
/// controlled by `pre_ready`: when `true`, sets `pending_client_ready`
/// so `should_defer` returns true.
pub(super) fn one_session(
    entity_id: u32,
    pre_ready: bool,
) -> (
    SocketAddr,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let addr: SocketAddr = "127.0.0.1:55550".parse().unwrap();
    let mut state = test_default_connected_client_state();
    if pre_ready {
        state.pending_client_ready = Some(PendingClientReadyInfo {
            entity_id,
            player_id: 1,
            world_name: "Test".into(),
            appearance_args: Vec::new(),
            tint_args: Vec::new(),
            first_login: 0,
        });
    }
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    (addr, connected, entity_to_addr)
}
