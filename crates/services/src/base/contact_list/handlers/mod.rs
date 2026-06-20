//! BaseApp-side handlers for contact-list CellToBaseMsg variants.
//!
//! Split by lifecycle phase:
//! - `header_ops`     — list-header operations (create/delete/rename/flags_update)
//! - `member_ops`     — member operations (add_members/remove_members)
//! - `presence_fanout` — login/logout broadcast to watchers
//!
//! `push_contact_lists_on_login` lives here because it is the single entry
//! point that login-path callers (`world_entry_appearance::handle_on_client_ready`)
//! import — keeping it in `mod.rs` avoids a deeper import path for a hot path.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use crate::base::contact_list::persistence::{ensure_system_lists, load_contact_lists};
use crate::base::contact_list::wire::{
    build_on_contact_list_add_members, build_on_contact_list_update,
};
use crate::base::helpers::send_to_witness_reliable;
use crate::base::ConnectedClientState;
use crate::mercury::{build_player_entity_method_packet, method_idx};

pub(crate) mod header_ops;
pub(crate) mod member_ops;
pub(crate) mod presence_fanout;

// Re-export the public surface so callers (cell_dispatch, login path, etc.)
// can import from `handlers::*` without knowing the split.
pub(crate) use header_ops::{handle_create, handle_delete, handle_flags_update, handle_rename};
pub(crate) use member_ops::{handle_add_members, handle_remove_members};
pub(crate) use presence_fanout::{fanout_contact_event, fanout_login_status};

/// Push all contact lists + members to the player's client on world entry.
///
/// Called from `world_entry_appearance::handle_on_client_ready` after the
/// burst bundle. Calls `ensure_system_lists` first so every character
/// always has Friends / Ignore even on first login.
///
/// Does nothing (logs warn) when `db_pool` is `None`.
pub(crate) async fn push_contact_lists_on_login(
    entity_id: u32,
    player_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(p) => p.as_ref(),
        None => {
            tracing::warn!(
                entity_id,
                player_id,
                "ContactList login push: no DB pool — contact lists not sent"
            );
            return;
        }
    };

    // Ensure system lists exist (idempotent).
    if let Err(e) = ensure_system_lists(pool, player_id).await {
        tracing::error!(
            entity_id,
            player_id,
            "ContactList: ensure_system_lists failed: {e}"
        );
        return;
    }

    let lists = match load_contact_lists(pool, player_id).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                "ContactList login push: load_contact_lists failed: {e}"
            );
            return;
        }
    };

    for list in &lists {
        // CM 85: header.
        let update_args = build_on_contact_list_update(list.list_id, &list.name, list.flags as u32);
        send_to_witness_reliable(
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
                    method_idx::ON_CONTACT_LIST_UPDATE,
                    &update_args,
                    version,
                )
            },
        )
        .await;

        // CM 87: members (sent even when empty — client expects the packet).
        let members_args = build_on_contact_list_add_members(list.list_id, &list.members);
        send_to_witness_reliable(
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
                    method_idx::ON_CONTACT_LIST_ADD_MEMBERS,
                    &members_args,
                    version,
                )
            },
        )
        .await;
    }

    tracing::debug!(
        entity_id,
        player_id,
        list_count = lists.len(),
        "ContactList: pushed {} lists to client on login",
        lists.len()
    );
}
