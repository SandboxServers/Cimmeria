//! Targeted inventory reconciliation after the atomic Frost letter commit.

use cimmeria_base_session::base::helpers::{
    send_letter_turn_in_method, ExpectedLetterTurnInEntry, LetterTurnInSendOutcome,
};
use cimmeria_base_session::base::plugin::BaseCtx;
use sqlx::PgPool;

use super::{update_item_args, InventoryRow, INVENTORY_ONE_ITEM_SELECT};
use crate::mercury::method_idx;

pub(crate) async fn sync_committed_letter(
    pool: &PgPool,
    expected: &ExpectedLetterTurnInEntry,
    item_id: i32,
    stack_after: i32,
    ctx: BaseCtx<'_>,
) -> Result<LetterTurnInSendOutcome, sqlx::Error> {
    let item = if stack_after > 0 {
        sqlx::query_as::<_, InventoryRow>(INVENTORY_ONE_ITEM_SELECT)
            .bind(expected.player_id)
            .bind(item_id)
            .fetch_optional(pool)
            .await?
    } else {
        None
    };
    // A concurrent mutation can delete the partial stack after A1 commits.
    // The client's cache then needs an explicit removal, not an empty upsert.
    let (method, args) = if let Some(row) = item {
        (method_idx::ON_UPDATE_ITEM, update_item_args(&[row]))
    } else {
        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&1u32.to_le_bytes());
        args.extend_from_slice(&item_id.to_le_bytes());
        (method_idx::ON_REMOVE_ITEM, args)
    };
    Ok(send_letter_turn_in_method(
        expected,
        method,
        &args,
        ctx.transport,
        ctx.connected,
        ctx.entity_to_addr,
    )
    .await)
}
