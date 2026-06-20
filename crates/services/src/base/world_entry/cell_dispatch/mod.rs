//! `CellToBaseMsg` dispatch -- routes messages from CellService to client
//! packets, delegating gate-travel, vendor, inventory, mail, missions, and
//! minigame work to focused handlers.
//!
//! [`handle_cell_message`] is a thin two-level router: the outer match here
//! picks the message *family*, then hands the whole `CellToBaseMsg` to that
//! family's `route` fn, which destructures the variant and runs the
//! arm body. The per-family arm bodies live in sibling modules so each
//! family's wire-emitting / DB-touching logic can grow without bloating a
//! single 40-arm match:
//!
//! - [`aoi_dispatch`]          — AoI / space-lifecycle arms (deferred-buffer
//!   gate + emit), themselves routing into the [`aoi`] packet emitters
//! - [`inventory_dispatch`]    — inventory / bandolier / persisted-options arms
//! - [`vendor_dispatch`]       — vendor store + two-party trade arms
//! - [`progression_dispatch`]  — mission / grant / console / spawn / mail /
//!   minigame arms
//! - [`gate_teleport_dispatch`] — gate-travel / reanchor / teleport arms
//!
//! Pure helpers shared by the arm modules still live in their own siblings:
//!
//! - [`aoi`]       — AoI packet emitters (entered/left/moved/method/invisible)
//! - [`minigame`]  — minigame session start + result handling
//! - [`bandolier`] — active-slot + bandolier-ammo persistence

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};

use super::super::ConnectedClientState;

mod aoi;
mod aoi_dispatch;
mod bandolier;
mod contact_list_dispatch;
mod gate_teleport_dispatch;
mod inventory_dispatch;
mod minigame;
mod progression_dispatch;
mod state_field;
mod system_options;
mod vendor_dispatch;

pub(crate) use aoi::flush_deferred_aoi;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_dispatch_arms;

/// Shared per-call context threaded to every family `route` fn.
///
/// Bundles the transport + session maps + pools the arm bodies need so the
/// router signatures stay readable instead of carrying eight positional
/// parameters each. Borrowed for the duration of one `handle_cell_message`
/// call; nothing is owned here.
pub(super) struct DispatchCtx<'a> {
    pub transport: &'a Arc<dyn Transport>,
    pub connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
    pub cell_tx: &'a Option<mpsc::Sender<BaseToCellMsg>>,
    pub db_pool: &'a Option<Arc<PgPool>>,
    pub minigame_registry: &'a Option<crate::minigame::SessionRegistry>,
    pub minigame_external_host: &'a str,
    pub minigame_external_port: u16,
}

/// Handle a message from CellService -- dispatches AoI packets to witness clients.
pub(crate) async fn handle_cell_message(
    msg: CellToBaseMsg,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    db_pool: &Option<Arc<PgPool>>,
    minigame_registry: &Option<crate::minigame::SessionRegistry>,
    minigame_external_host: &str,
    minigame_external_port: u16,
) {
    let ctx = DispatchCtx {
        transport,
        connected,
        entity_to_addr,
        cell_tx,
        db_pool,
        minigame_registry,
        minigame_external_host,
        minigame_external_port,
    };

    // Two-level routing: the outer match only decides which family owns the
    // variant; the family `route` fn destructures the variant and runs the
    // arm body. Keeps this shell short while preserving exhaustiveness — a
    // new variant that isn't slotted into a family below fails to compile.
    match msg {
        CellToBaseMsg::SpaceData { .. }
        | CellToBaseMsg::EntityCreated { .. }
        | CellToBaseMsg::EnteredAoI { .. }
        | CellToBaseMsg::LeftAoI { .. }
        | CellToBaseMsg::EntityMoved { .. }
        | CellToBaseMsg::EntityMethodCall { .. }
        | CellToBaseMsg::EntityMethodCallBatch { .. }
        | CellToBaseMsg::WitnessEntityMethod { .. }
        | CellToBaseMsg::EntityInvisible { .. } => aoi_dispatch::route(msg, &ctx).await,

        CellToBaseMsg::GateTravel { .. }
        | CellToBaseMsg::ReanchorPlayer { .. }
        | CellToBaseMsg::TeleportPlayer { .. } => gate_teleport_dispatch::route(msg, &ctx).await,

        CellToBaseMsg::MailRequest { .. }
        | CellToBaseMsg::MissionUpdate { .. }
        | CellToBaseMsg::GrantXP { .. }
        | CellToBaseMsg::TrainAbility { .. }
        | CellToBaseMsg::GrantItem { .. }
        | CellToBaseMsg::GrantCash { .. }
        | CellToBaseMsg::GrantExpertise { .. }
        | CellToBaseMsg::GrantAppliedSciencePoints { .. }
        | CellToBaseMsg::ExecuteAuthoringSql { .. }
        | CellToBaseMsg::ConsoleSearch { .. }
        | CellToBaseMsg::GmSpawnNpc { .. }
        | CellToBaseMsg::StartMinigame { .. }
        | CellToBaseMsg::MinigameResult { .. } => progression_dispatch::route(msg, &ctx).await,

        CellToBaseMsg::ContactListCreate { .. }
        | CellToBaseMsg::ContactListDelete { .. }
        | CellToBaseMsg::ContactListRename { .. }
        | CellToBaseMsg::ContactListFlagsUpdate { .. }
        | CellToBaseMsg::ContactListAddMembers { .. }
        | CellToBaseMsg::ContactListRemoveMembers { .. } => {
            contact_list_dispatch::route(msg, &ctx).await
        }

        CellToBaseMsg::OpenVendorStore { .. }
        | CellToBaseMsg::PurchaseVendorItems { .. }
        | CellToBaseMsg::SellVendorItems { .. }
        | CellToBaseMsg::BuybackVendorItems { .. }
        | CellToBaseMsg::ExecuteTrade { .. } => vendor_dispatch::route(msg, &ctx).await,

        CellToBaseMsg::ListInventoryItems { .. }
        | CellToBaseMsg::MoveInventoryItem { .. }
        | CellToBaseMsg::RemoveInventoryItem { .. }
        | CellToBaseMsg::UseInventoryItem { .. }
        | CellToBaseMsg::RemoveInventoryItemByType { .. }
        | CellToBaseMsg::RepairInventoryItem { .. }
        | CellToBaseMsg::RepairInventoryItems { .. }
        | CellToBaseMsg::RechargeInventoryItems { .. }
        | CellToBaseMsg::ActiveSlotUpdate { .. }
        | CellToBaseMsg::SystemOptionsUpdate { .. }
        | CellToBaseMsg::StateFieldUpdate { .. }
        | CellToBaseMsg::RefreshAppearance { .. }
        | CellToBaseMsg::BandolierAmmoUpdate { .. } => inventory_dispatch::route(msg, &ctx).await,
    }
}
