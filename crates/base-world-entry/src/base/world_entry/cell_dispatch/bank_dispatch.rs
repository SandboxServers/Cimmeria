//! Bank dispatch arm for `CellToBaseMsg::Bank`.
//!
//! Every `BankCellToBase` variant lands here. Later bank packets add their
//! handlers to this file rather than to `mod.rs`.

use crate::base::bank_dump::{handle_gm_dump, DumpCaller};
use crate::cell::messages::BankCellToBase;

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
    }
}
