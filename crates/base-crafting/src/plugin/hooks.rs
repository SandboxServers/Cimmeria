//! Crafting's lifecycle and seam hooks. Each body is the inline call core
//! made at that point before #962 step 5.

use cimmeria_base_session::base::plugin::{
    AppliedScienceCall, BoxFuture, InventoryCall, ItemConsumed, ItemUseCall, ItemUseOutcome,
    SessionEvent, WorldEntryCall,
};

use crate::base::crafting::item_use::{handle_crafting_item_use, is_crafting_miss};
use crate::base::crafting::options::{refresh_tools_from_rows, CraftingSessionOptions};
use crate::base::crafting::request::CraftCtx;
use crate::base::crafting::session::{drop_player_inductions, DropReason};
use crate::base::crafting::sync::{push_asp, push_crafting_on_login, CraftClient};
use crate::base::crafting::telemetry::account_id_of;
use crate::base::ConnectedClientState;

/// `logOff` and the disconnect teardown: the player's queued inductions
/// die with the session, and nothing they would have consumed is touched.
pub(super) fn drop_on_logout(event: SessionEvent) {
    drop_player_inductions(event.entity_id, DropReason::Logout, event.cause);
}

/// Gate travel: a world change drops the queue without consuming anything;
/// the running bar goes with the old world.
pub(super) fn drop_on_world_change(event: SessionEvent) {
    drop_player_inductions(event.entity_id, DropReason::WorldChange, event.cause);
}

/// `playCharacter`, when a new character enters the world: no crafting
/// station, tool or "craft anywhere" carries over from whatever the
/// connection played before, and no option send goes out until this entry's
/// login send. Removing the options is resetting them: a session without
/// them reads the defaults.
pub(super) fn reset_on_play_character(state: &mut ConnectedClientState) {
    state.extensions.remove::<CraftingSessionOptions>();
}

/// Gate travel, before the cell creates the destination entity: forget the
/// origin world's stations and hold option sends until the destination's
/// login send. A session crafting never touched has the defaults, which
/// this would leave unchanged, so it stores nothing.
pub(super) fn begin_world_entry(state: &mut ConnectedClientState) {
    if let Some(options) = state.extensions.get_mut::<CraftingSessionOptions>() {
        options.begin_world_entry();
    }
}

/// `onClientReady`: the crafting login sync (disciplines, paradigm levels,
/// blueprints, ASP, crafting options), owner-only, in one bundle.
pub(super) fn login_sync(call: WorldEntryCall<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        push_crafting_on_login(
            call.entity_id,
            call.player_id,
            call.ctx.db_pool,
            CraftClient {
                transport: call.ctx.transport,
                connected: call.ctx.connected,
                entity_to_addr: call.ctx.entity_to_addr,
            },
        )
        .await;
    })
}

/// Use a crafting item (a Blueprint item or a Racial Paradigm Guide): the
/// crafting use decides and commits; the core brings the inventory up to
/// date on a consumed item.
async fn use_item(call: ItemUseCall<'_>) -> ItemUseOutcome {
    let ctx = CraftCtx {
        db_pool: call.ctx.db_pool,
        cell_tx: call.ctx.cell_tx,
        transport: call.ctx.transport,
        connected: call.ctx.connected,
        entity_to_addr: call.ctx.entity_to_addr,
    };
    match handle_crafting_item_use(call.entity_id, call.player_id, call.item_id, &ctx).await {
        Some(consumed) => ItemUseOutcome::Consumed(ItemConsumed {
            removed_all: consumed.removed_all,
            outbox: consumed.outbox,
        }),
        None => ItemUseOutcome::Refused,
    }
}

/// `useItem` on this character's crafting item.
pub(super) fn use_crafting_item(call: ItemUseCall<'_>) -> BoxFuture<'_, ItemUseOutcome> {
    Box::pin(use_item(call))
}

/// `useItem` on an instance that is not this character's: a crafting item
/// the player already used up, or another character's, gets the crafting
/// refusal line ("no longer in your inventory"); anything else is not
/// crafting's.
pub(super) fn use_missing_item(call: ItemUseCall<'_>) -> BoxFuture<'_, ItemUseOutcome> {
    Box::pin(async move {
        let account_id = account_id_of(call.entity_id, call.ctx.connected, call.ctx.entity_to_addr);
        if !is_crafting_miss(
            call.pool.as_ref(),
            account_id,
            call.entity_id,
            call.player_id,
            call.item_id,
        )
        .await
        {
            return ItemUseOutcome::NotHandled;
        }
        use_item(call).await
    })
}

/// The inventory resync: a Field Crafting Tool may have entered or left the
/// crafting bag. No second query: the rows carry the container.
pub(super) fn refresh_tools(call: InventoryCall<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        refresh_tools_from_rows(
            call.entity_id,
            call.player_id,
            call.pool,
            call.rows.iter().copied(),
            call.transport,
            call.connected,
            call.entity_to_addr,
        )
        .await;
    })
}

/// An XP grant earned applied-science points: the discipline trainer shows
/// the ASP property live, so push the new total. `push_asp` logs its own
/// failed send.
pub(super) fn push_applied_science(call: AppliedScienceCall<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        push_asp(
            call.entity_id,
            call.player_id,
            call.total,
            CraftClient {
                transport: call.transport,
                connected: call.connected,
                entity_to_addr: call.entity_to_addr,
            },
        )
        .await;
    })
}
