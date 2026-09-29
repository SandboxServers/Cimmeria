//! The organization half the lower crates call (ORG-03, ORG-04, ORG-05).
//!
//! The organization cell methods (8-19 and 94), the squad disconnect and
//! the world-entry replay are the org plugin's (`cimmeria-cell-org`, #962
//! step 3). What stays here is what code below the plugin calls directly:
//!
//! - [`squad`]: the base-forwarded squad invite and kick
//!   (`OrgBaseToCell::SquadInvite` / `SquadKick`, which `cimmeria-cell`'s
//!   base-message handler calls), the GM `.squad_invite` / `.squad_join`
//!   backends (`cimmeria-cell-console` calls them), and the fanout, feedback
//!   lines and telemetry the plugin's handlers build on.
//! - [`creation`]: the registrar reply and the create result
//!   (`OrgBaseToCell::RegistrarEligible` / `CreateResult`, the base-message
//!   handler again), with the replies and telemetry cell method 94 shares.
//!
//! Moving these needs a base-message seam (the `CellToBaseMsg` /
//! `BaseToCellMsg` envelope of ADR §3.4) and the console as a plugin (ADR
//! §3.8 step 6). Until then they sit here, below the console, so the
//! console needs no edge to `cimmeria-cell-methods`.

pub mod creation;
pub mod squad;

use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;

use crate::cell::messages::CellToBaseMsg;

/// `text` from `SYSTEM` on the feedback channel, as an entity-method call
/// on `entity_id`'s own player.
pub fn feedback_line(entity_id: u32, text: &str) -> CellToBaseMsg {
    CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: ON_PLAYER_COMMUNICATION,
        args: serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text),
    }
}
