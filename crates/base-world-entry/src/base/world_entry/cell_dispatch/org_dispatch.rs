//! Organization dispatch arm for `CellToBaseMsg::Org`.
//!
//! Every `OrgCellToBase` variant lands here:
//!
//! - ORG-05's creation variants (`RegistrarOpen`, `Create`, `GmCreate`)
//!   call `base::organization::creation::handler`.
//! - `ForwardCellCall` carries a Team or Command cell method (8-17). The
//!   actor is re-checked against the session first (`resolve_actor`), then
//!   CM 8 goes to the invite response (ORG-07) and CM 9 to leave (ORG-06).
//!   CM 10 and 13-17 get ORG-01's "not available yet" pair until ORG-08
//!   serves them; CM 11 and 12 never reach the base (the cell refuses them
//!   as unsolicited), so a forward of either is a forged message.
//! - `TransferCash` (CM 19) is the Bank campaign's route: the reject stub
//!   answers with the "not available yet" pair and logs
//!   `org.transfer_cash_unimplemented` until BV-08 replaces this arm.
//! - The GM commands `GmDisband` (ORG-06), `GmJoin` and `GmRank` (ORG-07).
//!
//! Later packets add their handlers to this file rather than to `mod.rs`.

use std::ops::RangeInclusive;
use std::time::Instant;

use cimmeria_base_session::base::organization::creation::handler::{
    handle_create, handle_gm_create, handle_registrar_open, CreationCtx,
};
use cimmeria_base_session::base::organization::handlers::answer::not_available;
use cimmeria_base_session::base::organization::handlers::{
    gm_disband, gm_join, gm_rank, handle_invite_response, handle_leave, resolve_actor, GmCaller,
    OrgCtx,
};
use cimmeria_wire::cell::cell_methods::organization::{decode_org_cell_method, OrgCellCall};

use crate::cell::messages::OrgCellToBase;

use super::DispatchCtx;

fn org_ctx<'a>(ctx: &DispatchCtx<'a>) -> OrgCtx<'a> {
    OrgCtx {
        db_pool: ctx.db_pool,
        transport: ctx.transport,
        connected: ctx.connected,
        entity_to_addr: ctx.entity_to_addr,
        cell_tx: ctx.cell_tx,
    }
}

/// The OrganizationMember cell methods the cell may forward to the base:
/// invite response (8) to rank name (17). CM 18 (squad loot mode) never
/// leaves the cell, and CM 19 has its own `TransferCash` variant, so any
/// other index in a forward is a cell-side bug or a forged message.
const FORWARDABLE: RangeInclusive<u16> = 8..=17;

/// The creation handlers' view of the dispatch context.
fn creation_ctx<'a>(ctx: &DispatchCtx<'a>) -> CreationCtx<'a> {
    CreationCtx {
        db_pool: ctx.db_pool,
        cell_tx: ctx.cell_tx,
        transport: ctx.transport,
        connected: ctx.connected,
        entity_to_addr: ctx.entity_to_addr,
    }
}

/// Route one organization message from the cell.
pub(super) async fn route(msg: OrgCellToBase, ctx: &DispatchCtx<'_>) {
    let (player_id, entity_id) = msg.actor();
    let kind = msg.kind();
    let gm = GmCaller {
        entity_id,
        player_id,
    };
    match msg {
        // ORG-05: creation.
        OrgCellToBase::RegistrarOpen {
            npc_entity_id,
            org_type,
            ..
        } => {
            handle_registrar_open(
                &creation_ctx(ctx),
                player_id,
                entity_id,
                npc_entity_id,
                org_type,
            )
            .await
        }
        OrgCellToBase::Create { org_type, name, .. } => {
            handle_create(&creation_ctx(ctx), player_id, entity_id, org_type, &name).await
        }
        OrgCellToBase::GmCreate { org_type, name, .. } => {
            handle_gm_create(&creation_ctx(ctx), player_id, entity_id, org_type, &name).await
        }
        OrgCellToBase::TransferCash { org_id, dir, .. } => {
            // The Bank campaign's BV-08 replaces this arm. Until then the
            // press is answered, never silent.
            let octx = org_ctx(ctx);
            let live = resolve_actor(&octx, player_id, entity_id).is_some();
            tracing::debug!(
                target: "org",
                event = "org.transfer_cash_unimplemented",
                player_id,
                entity_id,
                kind,
                org_id,
                dir = ?dir,
                answered = live,
                "organizationTransferCash has no handler yet; answered with feedback"
            );
            if live {
                not_available(&octx, entity_id, org_id).await;
            }
        }
        OrgCellToBase::ForwardCellCall {
            method_index, args, ..
        } => forward(ctx, player_id, entity_id, method_index, &args).await,
        OrgCellToBase::GmDisband { org_id, .. } => {
            let _ = gm_disband(&org_ctx(ctx), gm, org_id).await;
        }
        OrgCellToBase::GmJoin {
            org_id,
            target_name,
            ..
        } => {
            let _ = gm_join(&org_ctx(ctx), gm, org_id, target_name.as_deref()).await;
        }
        OrgCellToBase::GmRank {
            target_name,
            rank,
            org_id,
            ..
        } => {
            let _ = gm_rank(&org_ctx(ctx), gm, &target_name, rank, org_id).await;
        }
    }
}

/// One forwarded OrganizationMember cell method (8-17) for a Team or
/// Command id or a base-issued request id.
async fn forward(
    ctx: &DispatchCtx<'_>,
    player_id: i32,
    entity_id: u32,
    method_index: u16,
    args: &[u8],
) {
    // Range first, before a single byte is decoded.
    if !FORWARDABLE.contains(&method_index) {
        tracing::warn!(
            target: "org",
            event = "org.forward_rejected",
            player_id,
            entity_id,
            method_index,
            reason = "method_out_of_range",
            "forwarded organization cell call outside 8..=17"
        );
        return;
    }
    let call = match decode_org_cell_method(method_index, args) {
        Ok(call) => call,
        Err(e) => {
            tracing::warn!(
                target: "org",
                event = "org.forward_rejected",
                player_id,
                entity_id,
                method_index,
                reason = e.reason(),
                error = %e,
                "forwarded organization cell call did not decode"
            );
            return;
        }
    };
    let octx = org_ctx(ctx);
    // The cell named the actor from its own entity; confirm it is still
    // this session's character in the world.
    let Some(player) = resolve_actor(&octx, player_id, entity_id) else {
        tracing::warn!(
            target: "org",
            event = "org.actor_mismatch",
            player_id,
            entity_id,
            method_index,
            org_id = call.org_id(),
            reason = "actor_mismatch",
            "forwarded organization call no longer matches a session in the world"
        );
        return;
    };
    match call {
        OrgCellCall::InviteResponse {
            request_id,
            response,
        } => {
            let _ =
                handle_invite_response(&octx, &player, request_id, response != 0, Instant::now())
                    .await;
        }
        OrgCellCall::Leave { org_id } => {
            let _ = handle_leave(&octx, &player, org_id).await;
        }
        OrgCellCall::StrikeTeamResponse { .. } | OrgCellCall::PvpLeaveResponse { .. } => {
            // The cell refuses both as unsolicited and never forwards them.
            tracing::warn!(
                target: "org",
                event = "org.forward_rejected",
                account_id = player.account_id,
                player_id,
                entity_id,
                method_index,
                reason = "unsolicited",
                "forwarded strike-team / PvP-leave response: nothing ever asked"
            );
        }
        call => {
            tracing::debug!(
                target: "org",
                event = "org.forward_unimplemented",
                account_id = player.account_id,
                player_id,
                entity_id,
                method_index,
                method = call.method_name(),
                org_id = call.org_id(),
                "organization cell method has no base handler yet; answered with feedback"
            );
            not_available(&octx, entity_id, call.org_id().unwrap_or(0)).await;
        }
    }
}
