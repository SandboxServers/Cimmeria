//! Bank dispatch arm for `CellToBaseMsg::Bank`.
//!
//! Every `BankCellToBase` variant lands here. Later bank packets add their
//! handlers to this file rather than to `mod.rs`.

use crate::base::bank_dump::{handle_gm_dump, DumpCaller};
use crate::base::bank_expand::{handle_expand, handle_expansion_quote, ExpandCaller};
use crate::cell::messages::BankCellToBase;

use cimmeria_base_session::base::organization::handlers::OrgCtx;

use super::super::methods::{
    handle_org_vault_expand, handle_org_vault_open, OrgVaultExpandRequest, OrgVaultIo,
    OrgVaultOpenRequest,
};

use super::DispatchCtx;

/// Route one bank message from the cell.
pub(super) async fn route(msg: BankCellToBase, ctx: &DispatchCtx<'_>) {
    match msg {
        BankCellToBase::GmDump {
            entity_id,
            account_id,
            player_id,
            subject,
        } => {
            let caller = DumpCaller {
                entity_id,
                account_id,
                player_id,
            };
            handle_gm_dump(
                caller,
                subject,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        BankCellToBase::ExpansionQuote {
            entity_id,
            account_id,
            player_id,
            speaker_id,
        } => {
            let caller = ExpandCaller {
                entity_id,
                account_id,
                player_id,
            };
            handle_expansion_quote(
                caller,
                speaker_id,
                ctx.db_pool,
                ctx.cell_tx,
                ctx.transport,
                ctx.connected,
            )
            .await
        }
        BankCellToBase::Expand {
            entity_id,
            account_id,
            player_id,
            offer,
            vault,
            trigger,
        } => {
            let caller = ExpandCaller {
                entity_id,
                account_id,
                player_id,
            };
            handle_expand(
                caller,
                offer,
                vault,
                trigger,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
            )
            .await
        }
        BankCellToBase::OrgVaultOpen {
            entity_id,
            account_id,
            player_id,
            scope,
            banker_id,
            distance,
            space_id,
        } => {
            let req = OrgVaultOpenRequest {
                entity_id,
                account_id,
                player_id,
                scope,
                banker_id,
                distance,
                space_id,
            };
            let io = OrgVaultIo {
                db_pool: ctx.db_pool,
                cell_tx: ctx.cell_tx,
                transport: ctx.transport,
                connected: ctx.connected,
                entity_to_addr: ctx.entity_to_addr,
            };
            handle_org_vault_open(req, io).await
        }
        BankCellToBase::OrgVaultExpand {
            entity_id,
            account_id,
            player_id,
            scope,
            from_slots,
        } => {
            let req = OrgVaultExpandRequest {
                entity_id,
                account_id,
                player_id,
                scope,
                from_slots,
            };
            let octx = OrgCtx {
                db_pool: ctx.db_pool,
                transport: ctx.transport,
                connected: ctx.connected,
                entity_to_addr: ctx.entity_to_addr,
                cell_tx: ctx.cell_tx,
            };
            handle_org_vault_expand(req, &octx).await
        }
    }
}
