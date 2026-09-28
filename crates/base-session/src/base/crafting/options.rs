//! `onUpdateCraftingOptions` (140): what the client's crafting window shows
//! as its machine or tool, and the per-session inputs behind it.
//!
//! The base owns the message. Its three inputs are:
//!
//! - the stations in reach, reported by the cell on change
//!   (`CellToBaseMsg::CraftingStations`);
//! - the Field Crafting Tools in the crafting bag, re-read after every
//!   inventory commit (`send_full_inventory_update` is the shared post-commit
//!   seam) and at login;
//! - "craft anywhere", which `.allcraft` turns on for the session.
//!
//! The client keeps only the **last** id of each array, so each section
//! carries at most one machine and one tool. It checks neither distance nor
//! existence; the server's gate is the only enforcement.
//!
//! Sends: always at login, where [`login_options`] hands the options to the
//! login crafting bundle (`sync::push_crafting_on_login`, after
//! `onClientReady`), then only when the options change. Before the login
//! send nothing goes out, so a station report that lands while the client
//! is still loading the world never reaches an entity the client has not
//! created. Every send logs `event = "options_changed"` with its cause.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_cell_catalog::crafting::CraftType;
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::client_methods::player::ON_UPDATE_CRAFTING_OPTIONS;
use cimmeria_wire::crafting::{
    crafting_options_args, CraftingInfo, CraftingOptions, CraftingStations, StationChangeCause,
    StationSet,
};
use sqlx::PgPool;

use super::respec::PendingRespec;
use super::tools::{best_tool, load_held_tools, tool_table, tools_in_crafting_bag, HeldTool};
use crate::base::helpers::send_to_witness_reliable;
use crate::base::session_identity::identity_for_entity;
use crate::base::ConnectedClientState;
use crate::mercury::build_player_entity_method_packet;

/// A session's crafting-options inputs and the last options sent. One per
/// connection (`ConnectedClientState::crafting_options`), so it dies with
/// the connection. A world entry clears the stations and disarms the
/// change sends ([`Self::begin_world_entry`]); the tools and "craft
/// anywhere" carry over a world change.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CraftingSessionOptions {
    /// The nearest station per verb, as the cell last reported.
    pub stations: StationSet,
    /// The tools in the crafting bag, by instance id.
    pub tools: Vec<HeldTool>,
    /// `.allcraft`'s "craft anywhere": every verb allowed, with the player
    /// named as its own machine.
    pub craft_anywhere: bool,
    /// Whether the login send of this world entry has been attempted.
    /// Change sends wait for it, so none reaches an entity the client has
    /// not created yet.
    pub armed: bool,
    /// The options the client last received: recorded only after a send
    /// went out, so a failed send is retried by the next report.
    pub last_sent: Option<CraftingOptions>,
    /// A crafting respec the player opened with `.respeccraft` and has not
    /// confirmed yet. Not an input to 140: it lives here because this is
    /// the session's crafting state, and it dies with the connection.
    pub pending_respec: Option<PendingRespec>,
}

impl CraftingSessionOptions {
    /// A world entry is starting: the stations in reach belong to the old
    /// world, and the client is about to drop its player entity. Forget the
    /// stations and wait for the next login send. Called before the base
    /// asks the cell for the new entity, so every station report that
    /// follows is the new world's.
    pub fn begin_world_entry(&mut self) {
        self.stations = StationSet::default();
        self.armed = false;
        self.last_sent = None;
    }
}

/// Why the options were re-evaluated: the `cause` field of
/// `options_changed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsCause {
    Moved,
    StationDespawned,
    WorldChange,
    Bag15Changed,
    Login,
    GmAnywhere,
}

impl OptionsCause {
    pub fn as_str(self) -> &'static str {
        match self {
            OptionsCause::Moved => "moved",
            OptionsCause::StationDespawned => "station_despawned",
            OptionsCause::WorldChange => "world_change",
            OptionsCause::Bag15Changed => "bag15_changed",
            OptionsCause::Login => "login",
            OptionsCause::GmAnywhere => "gm_anywhere",
        }
    }
}

impl From<StationChangeCause> for OptionsCause {
    fn from(cause: StationChangeCause) -> Self {
        match cause {
            StationChangeCause::Moved => OptionsCause::Moved,
            StationChangeCause::StationDespawned => OptionsCause::StationDespawned,
            StationChangeCause::WorldChange => OptionsCause::WorldChange,
        }
    }
}

/// Build the 140 payload for `entity_id` from its session inputs.
///
/// Per section: the station as the machine; for crafting, research and
/// reverse engineering, the best tool as the tool (alloying takes no tool).
/// Under "craft anywhere" every section names the player's own entity as
/// its machine, as the legacy `.allcraft` did.
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

/// Per section (crafting, research, reverseEngineering, alloying), the id a
/// list names, 0 for none: the `stations` and `tools` fields of
/// `options_changed`.
fn per_section(options: &CraftingOptions, pick: impl Fn(&CraftingInfo) -> &[i32]) -> [i32; 4] {
    [
        &options.crafting,
        &options.research,
        &options.reverse_engineering,
        &options.alloying,
    ]
    .map(|s| pick(s).last().copied().unwrap_or(0))
}

/// Apply `update` to `entity_id`'s session inputs and decide whether 140 is
/// due: always when `force` (which also arms the change sends), otherwise
/// only after the login send and only when the options differ from what
/// the client last received. A due send is logged (`options_changed`); the
/// caller sends it and, once it went out, records it with [`record_sent`].
/// `None` when nothing is due or the entity has no session (a
/// `lookup_failed` WARN).
fn update_options(
    entity_id: u32,
    force: bool,
    cause: OptionsCause,
    update: impl FnOnce(&mut CraftingSessionOptions),
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Option<((u32, Option<i32>), CraftingOptions)> {
    let session = entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&entity_id).copied());
    let found = session.and_then(|addr| {
        let mut clients = connected.lock().ok()?;
        let client = clients.get_mut(&addr)?;
        let identity = (client.account_id, client.active_player_id);
        let inputs = &mut client.crafting_options;
        update(inputs);
        let options = build_options(entity_id, inputs);
        let due = force || (inputs.armed && inputs.last_sent.as_ref() != Some(&options));
        if force {
            inputs.armed = true;
        }
        Some((identity, due.then_some(options)))
    });
    let Some(((account_id, player_id), options)) = found else {
        let identity = identity_for_entity(connected, entity_to_addr, entity_id);
        tracing::warn!(
            target: "crafting",
            event = "lookup_failed",
            phase = "session",
            account_id = identity.account_id,
            player_id = identity.player_id,
            entity_id,
            cause = cause.as_str(),
            "crafting options update for an entity with no session; dropped"
        );
        return None;
    };
    let options = options?;
    tracing::info!(
        target: "crafting",
        event = "options_changed",
        account_id,
        player_id,
        entity_id,
        cause = cause.as_str(),
        stations = ?per_section(&options, |s| &s.entities),
        tools = ?per_section(&options, |s| &s.items),
        "crafting options changed"
    );
    Some(((account_id, player_id), options))
}

/// [`update_options`], then send 140 on its own when it is due. Returns
/// whether a send went out.
async fn update_and_send(
    entity_id: u32,
    force: bool,
    cause: OptionsCause,
    update: impl FnOnce(&mut CraftingSessionOptions),
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> bool {
    let Some((identity, options)) =
        update_options(entity_id, force, cause, update, connected, entity_to_addr)
    else {
        return false;
    };
    if send_options(
        entity_id,
        identity,
        &options,
        transport,
        connected,
        entity_to_addr,
    )
    .await
    {
        record_sent(entity_id, options, connected, entity_to_addr);
    }
    true
}

/// Record `options` as what `entity_id`'s client last received. Call only
/// after the send went out: an unrecorded failure leaves the next identical
/// report due, so the client is not stuck on stale options.
pub fn record_sent(
    entity_id: u32,
    options: CraftingOptions,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let Some(addr) = entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&entity_id).copied())
    else {
        return;
    };
    if let Some(client) = connected
        .lock()
        .ok()
        .as_mut()
        .and_then(|c| c.get_mut(&addr))
    {
        client.crafting_options.last_sent = Some(options);
    }
}

/// Send one `onUpdateCraftingOptions` to the player's own client; a failed
/// send is a `push_failed` WARN (`what = "crafting_options"`). Returns
/// whether it went out.
async fn send_options(
    entity_id: u32,
    (account_id, player_id): (u32, Option<i32>),
    options: &CraftingOptions,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> bool {
    let args = crafting_options_args(options);
    let outcome = send_to_witness_reliable(
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
    let Some(reason) = outcome.failure_reason() else {
        return true;
    };
    tracing::warn!(
        target: "crafting",
        event = "push_failed",
        what = "crafting_options",
        reason,
        account_id,
        player_id,
        entity_id,
        "onUpdateCraftingOptions did not reach the client"
    );
    false
}

/// The cell reported a new station set (`CellToBaseMsg::CraftingStations`).
pub async fn handle_station_report(
    report: CraftingStations,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let stations = report.stations;
    update_and_send(
        report.entity_id,
        false,
        report.cause.into(),
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
    player_id: i32,
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
                event = "lookup_failed",
                phase = "tool_table",
                account_id = identity_for_entity(connected, entity_to_addr, entity_id).account_id,
                player_id,
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
        OptionsCause::Bag15Changed,
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
        OptionsCause::GmAnywhere,
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

/// The login options: read the crafting bag, arm the change sends and
/// return the options for the login crafting bundle, which carries them
/// even when every section is empty, so the window starts from the
/// server's state. The caller records them with [`record_sent`] once the
/// bundle went out. A world change runs this again. A failed bag read
/// clears the tools, so the options go out without any. `None` when the
/// entity has no session.
pub async fn login_options(
    entity_id: u32,
    player_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Option<CraftingOptions> {
    let tools = match db_pool {
        Some(pool) => match load_held_tools(pool, player_id).await {
            Ok(tools) => Some(tools),
            Err(e) => {
                tracing::warn!(
                    target: "crafting",
                    event = "lookup_failed",
                    phase = "login_tools",
                    account_id = identity_for_entity(connected, entity_to_addr, entity_id).account_id,
                    player_id,
                    entity_id,
                    error = %e,
                    "crafting bag read failed at login; sending options without tools"
                );
                Some(Vec::new())
            }
        },
        // No database: nothing to read, the session's tools stand.
        None => None,
    };
    update_options(
        entity_id,
        true,
        OptionsCause::Login,
        |inputs| {
            if let Some(tools) = tools {
                inputs.tools = tools;
            }
        },
        connected,
        entity_to_addr,
    )
    .map(|(_, options)| options)
}

#[cfg(test)]
#[path = "options_tests.rs"]
mod tests;
