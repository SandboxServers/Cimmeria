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
//! - [`black_market_dispatch`] — Black Market search / create / bid / cancel
//!   arms, routing into `base::black_market`
//! - [`progression_dispatch`]  — mission / grant / console / spawn / mail /
//!   minigame arms
//! - [`item_grant_dispatch`]   — the `GrantItem` body `progression_dispatch`
//!   calls: loot pickups to the loot-aware grant, the rest to the plain one
//! - [`gate_teleport_dispatch`] — gate-travel / reanchor / teleport arms
//! - [`org_dispatch`]          — organization (`CellToBaseMsg::Org`) arms
//! - [`chat_dispatch`]         — chat (`CellToBaseMsg::Chat`) arms: the GM
//!   broadcast fan-out
//! - [`bank_dispatch`]         — bank (`CellToBaseMsg::Bank`) arms: the GM
//!   `.bankdump`
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

use cimmeria_base_session::base::plugin::{BaseCtx, BasePlugins};

use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};

use super::super::ConnectedClientState;

mod aoi;
mod aoi_dispatch;
mod bandolier;
mod bank_dispatch;
mod black_market_dispatch;
mod chat_dispatch;
mod contact_list_dispatch;
mod deferred_flush;
mod gate_teleport_dispatch;
mod inventory_dispatch;
mod item_grant_dispatch;
mod minigame;
mod org_dispatch;
mod player_ghost;
mod position;
mod progression_dispatch;
mod state_field;
mod system_options;
mod vendor_dispatch;

pub(crate) use deferred_flush::{flush_deferred_aoi, flush_deferred_self_methods};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_dispatch_arms;
#[cfg(test)]
mod tests_flush_fragmentation;

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
    /// The base plugin table (#962 step 5): the consumers of
    /// `CellToBaseMsg::Plugin`.
    pub plugins: &'a BasePlugins,
}

/// The crafting handlers' view of the dispatch context.
fn craft_ctx<'a>(ctx: &DispatchCtx<'a>) -> crate::base::crafting::request::CraftCtx<'a> {
    crate::base::crafting::request::CraftCtx {
        db_pool: ctx.db_pool,
        cell_tx: ctx.cell_tx,
        transport: ctx.transport,
        connected: ctx.connected,
        entity_to_addr: ctx.entity_to_addr,
    }
}

/// Handle a message from CellService with no base plugin installed: every
/// `CellToBaseMsg::Plugin` envelope is dropped with the no-consumer WARN.
///
/// For the dispatch tests (`test-support`), which predate the plugin table
/// and exercise the static arms; production goes through
/// [`route_cell_message`] with the service's table, and so does a test that
/// needs a plugin.
#[cfg(any(test, feature = "test-support"))]
pub async fn handle_cell_message(
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
    route_cell_message(
        msg,
        transport,
        connected,
        entity_to_addr,
        cell_tx,
        db_pool,
        minigame_registry,
        minigame_external_host,
        minigame_external_port,
        &BasePlugins::empty(),
    )
    .await
}

/// Handle a message from CellService -- dispatches AoI packets to witness
/// clients. `plugins` is the base plugin table, which consumes the
/// `CellToBaseMsg::Plugin` envelopes (#962 step 5).
pub async fn route_cell_message(
    msg: CellToBaseMsg,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    db_pool: &Option<Arc<PgPool>>,
    minigame_registry: &Option<crate::minigame::SessionRegistry>,
    minigame_external_host: &str,
    minigame_external_port: u16,
    plugins: &BasePlugins,
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
        plugins,
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
        | CellToBaseMsg::GrantStargateAddress { .. }
        | CellToBaseMsg::ReanchorPlayer { .. }
        | CellToBaseMsg::TeleportPlayer { .. } => gate_teleport_dispatch::route(msg, &ctx).await,

        CellToBaseMsg::MailRequest { .. }
        | CellToBaseMsg::MissionUpdate { .. }
        | CellToBaseMsg::GrantXP { .. }
        | CellToBaseMsg::TrainAbility { .. }
        | CellToBaseMsg::ResetAbilities { .. }
        | CellToBaseMsg::GrantItem { .. }
        | CellToBaseMsg::GrantCash { .. }
        | CellToBaseMsg::ContainerLooted { .. }
        | CellToBaseMsg::GrantTrainingPoints { .. }
        | CellToBaseMsg::GmGrantAbility { .. }
        | CellToBaseMsg::GrantExpertise { .. }
        | CellToBaseMsg::GrantAppliedSciencePoints { .. }
        | CellToBaseMsg::ExecuteAuthoringSql { .. }
        | CellToBaseMsg::ConsoleSearch { .. }
        | CellToBaseMsg::GmSpawnNpc { .. }
        | CellToBaseMsg::StartMinigame { .. }
        | CellToBaseMsg::MinigameResult { .. } => progression_dispatch::route(msg, &ctx).await,

        // Crafting (95-100): `base::crafting::request` routes the verbs, so
        // later verbs never touch these arms.
        CellToBaseMsg::Crafting(request) => {
            crate::base::crafting::request::handle_craft_request(request, &craft_ctx(&ctx)).await
        }
        CellToBaseMsg::CraftingStations(report) => {
            crate::base::crafting::options::handle_station_report(
                report,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::GmAllCraft(grant) => {
            crate::base::crafting::allcraft::handle_gm_all_craft(grant, &craft_ctx(&ctx)).await
        }
        CellToBaseMsg::GmCraftGrant(grant) => {
            crate::base::crafting::gm_grant::handle_gm_craft_grant(grant, &craft_ctx(&ctx)).await
        }
        CellToBaseMsg::GmGiveAmmo(give) => {
            super::methods::inventory::ammo_gm_give::handle_gm_give_ammo(
                give,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::RespecCraftOpen(open) => {
            crate::base::crafting::respec::handle_respec_open(open, &craft_ctx(&ctx)).await
        }

        CellToBaseMsg::ContactListCreate { .. }
        | CellToBaseMsg::ContactListDelete { .. }
        | CellToBaseMsg::ContactListRename { .. }
        | CellToBaseMsg::ContactListFlagsUpdate { .. }
        | CellToBaseMsg::ContactListAddMembers { .. }
        | CellToBaseMsg::ContactListRemoveMembers { .. }
        | CellToBaseMsg::ContactListPresenceEvent { .. } => {
            contact_list_dispatch::route(msg, &ctx).await
        }

        CellToBaseMsg::OpenVendorStore { .. }
        | CellToBaseMsg::PurchaseVendorItems { .. }
        | CellToBaseMsg::SellVendorItems { .. }
        | CellToBaseMsg::BuybackVendorItems { .. }
        | CellToBaseMsg::ExecuteTrade { .. } => vendor_dispatch::route(msg, &ctx).await,

        CellToBaseMsg::BlackMarket(bm) => black_market_dispatch::route(bm, &ctx).await,

        CellToBaseMsg::ListInventoryItems { .. }
        | CellToBaseMsg::MoveInventoryItem { .. }
        | CellToBaseMsg::RemoveInventoryItem { .. }
        | CellToBaseMsg::ConsumeItemForUse(_)
        | CellToBaseMsg::UseInventoryItem { .. }
        | CellToBaseMsg::RemoveInventoryItemByType { .. }
        | CellToBaseMsg::RepairInventoryItem { .. }
        | CellToBaseMsg::RepairInventoryItems { .. }
        | CellToBaseMsg::RechargeInventoryItems { .. }
        | CellToBaseMsg::ActiveSlotUpdate { .. }
        | CellToBaseMsg::SystemOptionsUpdate { .. }
        | CellToBaseMsg::StateFieldUpdate { .. }
        | CellToBaseMsg::PersistPosition { .. }
        | CellToBaseMsg::RefreshAppearance { .. }
        | CellToBaseMsg::BandolierAmmoUpdate { .. } => inventory_dispatch::route(msg, &ctx).await,

        CellToBaseMsg::Org(org) => org_dispatch::route(org, &ctx).await,

        CellToBaseMsg::Chat(chat) => chat_dispatch::route(chat, &ctx).await,

        // GM mail tools (SS-U1): the base-methods mail module owns them.
        CellToBaseMsg::MailGm(msg) => {
            super::methods::mail::handle_mail_gm(
                msg,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
                ctx.db_pool,
            )
            .await
        }

        CellToBaseMsg::Bank(bank) => bank_dispatch::route(bank, &ctx).await,

        // A migrated feature's message (#962, plugin ADR §3.4): the base
        // plugin that consumes its payload type handles it; one nobody
        // consumes is dropped with a WARN (`reason = "no_consumer"`).
        CellToBaseMsg::Plugin(msg) => {
            ctx.plugins
                .dispatch_cell_message(
                    msg,
                    BaseCtx {
                        db_pool: ctx.db_pool,
                        cell_tx: ctx.cell_tx,
                        transport: ctx.transport,
                        connected: ctx.connected,
                        entity_to_addr: ctx.entity_to_addr,
                    },
                )
                .await;
        }

        // The special-ammo reserve round trip (ammo campaign AM-02).
        CellToBaseMsg::AmmoReserve(req) => {
            super::methods::inventory::ammo_reserve::handle_ammo_reserve_request(
                req,
                super::methods::inventory::ammo_reserve::ReserveIo {
                    db_pool: ctx.db_pool,
                    cell_tx: ctx.cell_tx,
                    transport: ctx.transport,
                    connected: ctx.connected,
                    entity_to_addr: ctx.entity_to_addr,
                },
            )
            .await
        }

        // The content engine's `send_system_mail` action (SS-U3).
        CellToBaseMsg::ContentSystemMail(msg) => {
            super::methods::mail::handle_content_system_mail(
                msg,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
                ctx.db_pool,
            )
            .await
        }
    }
}
