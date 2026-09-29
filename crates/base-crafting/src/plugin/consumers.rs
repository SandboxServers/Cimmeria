//! The consumers of the crafting payloads in the `CellToBaseMsg::Plugin`
//! envelope. Each downcasts its payload and calls the handler the central
//! `CellToBaseMsg` arm (`cimmeria-base-world-entry`'s `cell_dispatch`)
//! called before #962 step 5, with the same arguments.

use std::any::Any;

use cimmeria_base_session::base::plugin::{BaseCtx, BoxFuture, PluginMsg};
use cimmeria_wire::crafting::{
    CraftRequest, CraftingStations, GmAllCraft, GmCraftGrant, GmGrantAppliedSciencePoints,
    GmGrantExpertise, RespecCraftOpen,
};

use crate::base::crafting::request::CraftCtx;
use crate::base::crafting::{allcraft, gm_grant, handlers, options, request, respec};

/// The envelope's payload as a `T`. The registry routes by payload type, so
/// a mismatch is a registration bug: it logs an ERROR and drops the message
/// rather than panicking the base's message loop.
fn payload<T: Any>(msg: PluginMsg) -> Option<T> {
    match msg.downcast::<T>() {
        Ok(payload) => Some(payload),
        Err(msg) => {
            tracing::error!(
                target: "crafting",
                event = "payload_mismatch",
                expected = std::any::type_name::<T>(),
                type_name = msg.type_name(),
                "crafting consumer got another payload type; dropped"
            );
            None
        }
    }
}

/// The crafting handlers' view of the base context.
fn craft_ctx(ctx: BaseCtx<'_>) -> CraftCtx<'_> {
    CraftCtx {
        db_pool: ctx.db_pool,
        cell_tx: ctx.cell_tx,
        transport: ctx.transport,
        connected: ctx.connected,
        entity_to_addr: ctx.entity_to_addr,
    }
}

/// A crafting verb (methods 95-100): `request` routes it.
pub(super) fn craft_request(msg: PluginMsg, ctx: BaseCtx<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        if let Some(request) = payload::<CraftRequest>(msg) {
            request::handle_craft_request(request, &craft_ctx(ctx)).await;
        }
    })
}

/// The stations in reach changed: rebuild `onUpdateCraftingOptions`.
pub(super) fn station_report(msg: PluginMsg, ctx: BaseCtx<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        if let Some(report) = payload::<CraftingStations>(msg) {
            options::handle_station_report(
                report,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await;
        }
    })
}

/// `.allcraft`.
pub(super) fn gm_all_craft(msg: PluginMsg, ctx: BaseCtx<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        if let Some(grant) = payload::<GmAllCraft>(msg) {
            allcraft::handle_gm_all_craft(grant, &craft_ctx(ctx)).await;
        }
    })
}

/// `.craftkit` and `.learnblueprint`.
pub(super) fn gm_craft_grant(msg: PluginMsg, ctx: BaseCtx<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        if let Some(grant) = payload::<GmCraftGrant>(msg) {
            gm_grant::handle_gm_craft_grant(grant, &craft_ctx(ctx)).await;
        }
    })
}

/// A player's `.respeccraft`.
pub(super) fn respec_open(msg: PluginMsg, ctx: BaseCtx<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        if let Some(open) = payload::<RespecCraftOpen>(msg) {
            respec::handle_respec_open(open, &craft_ctx(ctx)).await;
        }
    })
}

/// `gmGiveExpertise`.
pub(super) fn grant_expertise(msg: PluginMsg, ctx: BaseCtx<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        if let Some(grant) = payload::<GmGrantExpertise>(msg) {
            handlers::handle_grant_expertise(
                grant.entity_id,
                grant.player_id,
                grant.discipline_id,
                grant.amount,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await;
        }
    })
}

/// `gmGiveAppliedSciencePoints`.
pub(super) fn grant_applied_science(msg: PluginMsg, ctx: BaseCtx<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        if let Some(grant) = payload::<GmGrantAppliedSciencePoints>(msg) {
            handlers::handle_grant_applied_science(
                grant.entity_id,
                grant.player_id,
                grant.amount,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await;
        }
    })
}
