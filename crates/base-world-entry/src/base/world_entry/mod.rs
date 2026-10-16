//! World entry orchestration -- play_character (entity teardown), ENABLE_ENTITIES
//! (create player), mapLoaded (enter world), gate travel, and CellToBase message dispatch.
//!
//! Sub-concerns are split into sibling modules:
//! - `space_registry` -- world_name -> space_id table populated by CellService SpaceData.
//! - `play_character` -- `playCharacter` -> RESET_ENTITIES teardown.
//! - `enable_entities` -- ENABLE_ENTITIES dispatch (char list vs create-player).
//! - `map_loaded` -- mapLoaded -> VIEWPORT + CELL + entity-data enter-world.
//! - `gate_travel` -- gate transitions (replays world entry against a new space).
//! - `cell_dispatch` -- CellToBaseMsg fan-out to per-feature handlers.
//! - `methods` -- DB queries and feature handlers (player load, inventory, vendor,
//!   trade, mail, missions, progression), in `cimmeria-base-methods` since wave
//!   B2 of docs/architecture/services-crate-split.md.
//!
//! This module is in `cimmeria-base-world-entry` since wave B3;
//! `cimmeria-services` re-exports it at its old path.

// The feature handlers are in `cimmeria-base-methods` (wave B2), re-exported
// at their old path, so `super::methods::…` in the siblings is unchanged.
// `pub`: tests in `cimmeria-services` reach them through this path.
pub use cimmeria_base_methods::base::world_entry::methods;

pub(crate) mod cell_dispatch;
mod enable_entities;
mod gate_travel;
mod lab_fixture;
pub(crate) mod looted_containers;
mod map_loaded;
mod map_loaded_wire_rows;
mod play_character;
mod reanchor_player;
pub(crate) mod shown_tutorials;
mod teleport;

#[cfg(test)]
mod direct_send_origin_tests;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::packet::{Bytes, Packet, PacketFlags};
use cimmeria_mercury::transport::{SendOrigin, Transport};

use super::ConnectedClientState;

/// Send a direct datagram with its build-time ticket, or leave missing tickets unproven.
pub(super) async fn send_direct(
    transport: &Arc<dyn Transport>,
    bytes: &[u8],
    addr: SocketAddr,
    origin: Option<&SendOrigin>,
) -> std::io::Result<usize> {
    match origin {
        Some(origin) => transport.send_to_with_origin(bytes, addr, origin).await,
        None => transport.send_to(bytes, addr).await,
    }
}

/// Register only in the channel that reserved the packet before its send await.
fn register_direct_reliable(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    seq: u32,
    bytes: &[u8],
    next_seq: &Arc<AtomicU32>,
    origin: Option<&SendOrigin>,
) {
    let Ok(clients) = connected.lock() else {
        return;
    };
    let Some(state) = clients.get(&addr) else {
        return;
    };
    if !Arc::ptr_eq(&state.next_seq, next_seq)
        || !same_direct_origin(state.wire_send_origin().as_ref(), origin)
    {
        return;
    }
    let Ok(mut channel) = state.channel.lock() else {
        return;
    };
    let packet = Packet::new(PacketFlags::default(), seq, Bytes::new());
    if let Err(error) = channel.register_sent_packet(packet, Bytes::copy_from_slice(bytes)) {
        tracing::warn!(%addr, seq, %error, "direct reliable send registration failed");
    }
}

/// Compare the connection seed, allowing the player field to change within one session.
pub(super) fn same_direct_origin(
    current: Option<&SendOrigin>,
    expected: Option<&SendOrigin>,
) -> bool {
    match (current, expected) {
        (None, None) => true,
        (Some(current), Some(expected)) => {
            current.server_boot_id == expected.server_boot_id
                && current.session_generation == expected.session_generation
                && current.peer == expected.peer
                && current.connected_at == expected.connected_at
        }
        _ => false,
    }
}

#[cfg(test)]
mod crafting_options_world_entry_tests;

#[cfg(test)]
mod datagram_size_tests;
/// NT-24: the play-character path names the account, character and world.
#[cfg(test)]
mod named_world_entry_tests;
/// SS-00: the online name index across world entry and reanchor.
#[cfg(test)]
mod player_index_lifecycle_tests;

// The space registry is in `cimmeria-base-session` (wave B1): base-methods
// resolves world names through it from below world entry.
pub(crate) use cimmeria_base_session::base::world_entry::space_registry;

// Public surface: the connect loop and `BaseService`, in `cimmeria-base`,
// import these through `super::world_entry::handle_*`.
#[cfg(any(test, feature = "test-support"))]
pub use cell_dispatch::handle_cell_message;
pub use cell_dispatch::route_cell_message;
pub use enable_entities::handle_enable_entities;
pub use map_loaded::handle_map_loaded;
pub use play_character::handle_play_character;

// Legacy re-exports from world_entry_appearance (kept here so connect_loop.rs's
// existing `super::world_entry::{handle_on_client_ready, handle_cancel_movie}`
// imports stay unchanged after this refactor).
pub use super::world_entry_appearance::{handle_cancel_movie, handle_on_client_ready};

// Test hooks: the gate round trips in `cimmeria-services`
// (`gate_round_trip_tests`) drive the cell's dial handler before these, so they
// stay there and reach these through the `test-support` feature.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub use gate_travel::{handle_gate_travel, persist_arrival};
