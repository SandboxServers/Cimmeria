//! `onUpdateCraftingOptions` (140): what the client's crafting window shows
//! as its machine or tool, and the per-session inputs behind it (CR-05).
//!
//! The base owns the message. Its three inputs are:
//!
//! - the stations in reach, reported by the cell on change
//!   (`CellToBaseMsg::CraftingStations`);
//! - the Field Crafting Tools in the crafting bag, re-read after every
//!   inventory commit (`send_full_inventory_update` is the shared post-commit
//!   seam) and at login;
//! - "craft anywhere", which `.allcraft` turns on for the session (D-CR17).
//!
//! The client keeps only the **last** id of each array (CR-E1 Q2), so each
//! section carries at most one machine and one tool. It checks neither
//! distance nor existence; the server's gate in `request.rs` is the only
//! enforcement.
//!
//! Sends: always at login ([`send_login_options`], after `onClientReady`),
//! then only when the options change. Before the login send nothing goes
//! out, so a station report that lands while the client is still loading
//! the world never reaches an entity the client has not created.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_cell_catalog::crafting::CraftType;
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::client_methods::player::ON_UPDATE_CRAFTING_OPTIONS;
use cimmeria_wire::crafting::{crafting_options_args, CraftingInfo, CraftingOptions, StationSet};
use sqlx::PgPool;

use super::tools::{best_tool, load_held_tools, tool_table, tools_in_crafting_bag, HeldTool};
use crate::base::helpers::send_to_witness_reliable;
use crate::base::ConnectedClientState;
use crate::mercury::build_player_entity_method_packet;

/// A session's crafting-options inputs and the last options sent. One per
/// connection (`ConnectedClientState::crafting_options`), so it dies with
/// the connection and survives a world change.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CraftingSessionOptions {
    /// The nearest station per verb, as the cell last reported.
    pub stations: StationSet,
    /// The tools in the crafting bag, by instance id.
    pub tools: Vec<HeldTool>,
    /// `.allcraft`'s "craft anywhere": every verb allowed, with the player
    /// named as its own machine.
    pub craft_anywhere: bool,
    /// The options last sent; `None` until the login send.
    pub last_sent: Option<CraftingOptions>,
}

/// Build the 140 payload for `entity_id` from its session inputs.
///
/// Per section: the station as the machine; for crafting, research and
/// reverse engineering, the best tool as the tool (alloying takes no tool,
/// D-CR21). Under "craft anywhere" every section names the player's own
/// entity as its machine, as the legacy `.allcraft` did
/// (`Crafting.py:85-95`).
pub fn build_options(entity_id: u32, inputs: &CraftingSessionOptions) -> CraftingOptions {
    let tool = best_tool(&inputs.tools).map(|t| t.instance_id);
    let section = |verb: CraftType, station: Option<u32>| {
        let machine = if inputs.craft_anywhere {
            Some(entity_id)
        } else {
            station
        };
        CraftingInfo {
            items: match verb {
                CraftType::Alloying => vec![],
                _ => tool.into_iter().collect(),
            },
            entities: machine.map(|id| id as i32).into_iter().collect(),
        }
    };
    let [craft, research, reverse, alloy] = inputs.stations;
    CraftingOptions {
        crafting: section(CraftType::Craft, craft),
        research: section(CraftType::Research, research),
        reverse_engineering: section(CraftType::ReverseEngineering, reverse),
        alloying: section(CraftType::Alloying, alloy),
    }
}

/// Apply `update` to `entity_id`'s session inputs and send 140 when due:
/// always when `force`, otherwise only after the login send and only when
/// the options changed. Returns whether a send went out.
async fn update_and_send(
    entity_id: u32,
    force: bool,
    update: impl FnOnce(&mut CraftingSessionOptions),
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> bool {
    let Some(addr) = entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&entity_id).copied())
    else {
        tracing::debug!(
            target: "crafting",
            event = "options_no_session",
            entity_id,
            "crafting options update for an entity with no session; dropped"
        );
        return false;
    };
    let options = {
        let Ok(mut clients) = connected.lock() else {
            return false;
        };
        let Some(client) = clients.get_mut(&addr) else {
            return false;
        };
        let inputs = &mut client.crafting_options;
        update(inputs);
        let options = build_options(entity_id, inputs);
        let due = force
            || inputs
                .last_sent
                .as_ref()
                .is_some_and(|sent| *sent != options);
        if !due {
            return false;
        }
        inputs.last_sent = Some(options.clone());
        options
    };
    send_options(entity_id, &options, transport, connected, entity_to_addr).await;
    true
}

/// Send one `onUpdateCraftingOptions` to the player's own client.
async fn send_options(
    entity_id: u32,
    options: &CraftingOptions,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    tracing::debug!(
        target: "crafting",
        event = "options_sent",
        entity_id,
        options = ?options,
        "onUpdateCraftingOptions"
    );
    let args = crafting_options_args(options);
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                ON_UPDATE_CRAFTING_OPTIONS,
                &args,
                version,
            )
        },
    )
    .await;
}

/// The cell reported a new station set (`CellToBaseMsg::CraftingStations`).
pub async fn handle_station_report(
    entity_id: u32,
    stations: StationSet,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    update_and_send(
        entity_id,
        false,
        |inputs| inputs.stations = stations,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// Re-derive the held tools from a fresh read of the player's inventory
/// (instance id, type id, container id), after an inventory commit.
pub async fn refresh_tools_from_rows(
    entity_id: u32,
    pool: &PgPool,
    rows: impl IntoIterator<Item = (i32, i32, i32)>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let table = match tool_table(pool).await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(
                target: "crafting",
                event = "tool_table_failed",
                entity_id,
                error = %e,
                "Field Crafting Tool table could not be loaded; tools unchanged"
            );
            return;
        }
    };
    let tools = tools_in_crafting_bag(&table, rows);
    update_and_send(
        entity_id,
        false,
        |inputs| inputs.tools = tools,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// Turn on "craft anywhere" for `entity_id`'s session and send the options
/// that name the player as its own machine.
pub async fn enable_craft_anywhere(
    entity_id: u32,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> bool {
    update_and_send(
        entity_id,
        true,
        |inputs| inputs.craft_anywhere = true,
        transport,
        connected,
        entity_to_addr,
    )
    .await
}

/// Whether `entity_id`'s session has "craft anywhere".
pub fn craft_anywhere(
    entity_id: u32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> bool {
    let Some(addr) = entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&entity_id).copied())
    else {
        return false;
    };
    connected
        .lock()
        .ok()
        .and_then(|c| c.get(&addr).map(|c| c.crafting_options.craft_anywhere))
        .unwrap_or(false)
}

/// The login send, after `onClientReady`: read the crafting bag and send
/// 140 unconditionally, even when every section is empty, so the window
/// starts from the server's state. A world change runs this again.
// TODO(CR-03): fold into the login crafting bundle once it lands.
pub async fn send_login_options(
    entity_id: u32,
    player_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let tools = match db_pool {
        Some(pool) => match load_held_tools(pool, player_id).await {
            Ok(tools) => Some(tools),
            Err(e) => {
                tracing::warn!(
                    target: "crafting",
                    event = "login_tools_failed",
                    entity_id,
                    player_id,
                    error = %e,
                    "crafting bag read failed at login; sending options without tools"
                );
                None
            }
        },
        None => None,
    };
    update_and_send(
        entity_id,
        true,
        |inputs| {
            if let Some(tools) = tools {
                inputs.tools = tools;
            }
        },
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

#[cfg(test)]
#[path = "options_tests.rs"]
mod tests;
