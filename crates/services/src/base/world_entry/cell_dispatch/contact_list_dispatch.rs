//! Contact-list dispatch arms for `CellToBaseMsg`.
//!
//! Routes the six `ContactList*` variants to the base-side handlers in
//! `crate::base::contact_list::handlers`. Mirrors the shape of
//! `progression_dispatch` — thin arms that extract variant fields and call
//! the appropriate handler.

use crate::base::contact_list::handlers::{
    fanout_contact_event, handle_add_members, handle_create, handle_delete, handle_flags_update,
    handle_remove_members, handle_rename,
};
use crate::cell::messages::CellToBaseMsg;

use super::DispatchCtx;

/// Route a `ContactList*` `CellToBaseMsg` to its handler.
///
/// Called by [`super::handle_cell_message`] after the outer match narrows
/// `msg` to the contact-list family.
pub(super) async fn route(msg: CellToBaseMsg, ctx: &DispatchCtx<'_>) {
    match msg {
        CellToBaseMsg::ContactListCreate {
            entity_id,
            player_id,
            name,
            flags,
        } => {
            handle_create(
                entity_id,
                player_id,
                name,
                flags,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }

        CellToBaseMsg::ContactListDelete {
            entity_id,
            player_id,
            list_id,
        } => {
            handle_delete(
                entity_id,
                player_id,
                list_id,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }

        CellToBaseMsg::ContactListRename {
            entity_id,
            player_id,
            list_id,
            name,
        } => {
            handle_rename(
                entity_id,
                player_id,
                list_id,
                name,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }

        CellToBaseMsg::ContactListFlagsUpdate {
            entity_id,
            player_id,
            list_id,
            flags,
        } => {
            handle_flags_update(
                entity_id,
                player_id,
                list_id,
                flags,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }

        CellToBaseMsg::ContactListAddMembers {
            entity_id,
            player_id,
            list_id,
            names,
        } => {
            handle_add_members(
                entity_id,
                player_id,
                list_id,
                names,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await;
            // If this targeted the Ignore list, re-sync the ignore cache + cell
            // AoI filter (no-op for Friends/custom lists).
            crate::base::dispatch::ignore::resync_ignore_after_member_change(
                entity_id,
                player_id,
                list_id,
                ctx.db_pool,
                ctx.connected,
                ctx.entity_to_addr,
                ctx.cell_tx,
            )
            .await;
        }

        CellToBaseMsg::ContactListRemoveMembers {
            entity_id,
            player_id,
            list_id,
            names,
        } => {
            handle_remove_members(
                entity_id,
                player_id,
                list_id,
                names,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await;
            crate::base::dispatch::ignore::resync_ignore_after_member_change(
                entity_id,
                player_id,
                list_id,
                ctx.db_pool,
                ctx.connected,
                ctx.entity_to_addr,
                ctx.cell_tx,
            )
            .await;
        }

        CellToBaseMsg::ContactListPresenceEvent {
            player_name,
            event_id,
            data_value,
        } => {
            // Fire-and-forget: spawn so the cell channel drains immediately
            // rather than blocking on the DB lookup + network fan-out.
            let db_pool = ctx.db_pool.clone();
            let transport = std::sync::Arc::clone(ctx.transport);
            let connected = std::sync::Arc::clone(ctx.connected);
            let entity_to_addr = std::sync::Arc::clone(ctx.entity_to_addr);
            tokio::spawn(async move {
                fanout_contact_event(
                    &player_name,
                    event_id,
                    data_value,
                    &db_pool,
                    &transport,
                    &connected,
                    &entity_to_addr,
                )
                .await;
            });
        }

        // Unreachable by construction — the outer match only routes
        // ContactList* variants to this function.
        _ => unreachable!("contact_list_dispatch::route received non-ContactList variant"),
    }
}
