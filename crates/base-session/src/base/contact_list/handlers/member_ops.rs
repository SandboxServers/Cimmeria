//! Contact-list member operations: add_members, remove_members.
//!
//! Each handler: validates ownership (DB), mutates the DB, then echoes the
//! appropriate S→C client method (CM 87–88) to the requesting player. A
//! change to the Ignore list (flags 301) also reloads the Ignore cache on the
//! base session and the cell entity, so an ignore added from the contact-list
//! UI takes effect at once, as one added by `chatIgnore` does.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::base::contact_list::ignore::{resync_ignore_cache, IgnoreSyncCtx, IGNORE_LIST_FLAGS};
use crate::base::contact_list::persistence::{add_members, load_list_header, remove_members};
use crate::base::contact_list::wire::{
    build_on_contact_list_add_members, build_on_contact_list_remove_members,
    MAX_MEMBERS_PER_REQUEST,
};
use crate::base::helpers::send_to_witness_reliable;
use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_player_entity_method_packet, method_idx};

/// Handle `ContactListAddMembers` — insert members and echo CM 87. Returns the
/// names actually inserted (empty on a duplicate, a refusal or an error).
pub async fn handle_add_members(
    entity_id: u32,
    player_id: i32,
    list_id: i32,
    mut names: Vec<String>,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) -> Vec<String> {
    let pool = match db_pool {
        Some(p) => p.as_ref(),
        None => {
            tracing::warn!(
                entity_id,
                player_id,
                list_id,
                "ContactListAddMembers: no DB pool"
            );
            return Vec::new();
        }
    };

    // Clamp to prevent abuse.
    if names.len() > MAX_MEMBERS_PER_REQUEST {
        tracing::warn!(
            entity_id,
            player_id,
            list_id,
            count = names.len(),
            "ContactListAddMembers: clamping names array from {} to {MAX_MEMBERS_PER_REQUEST}",
            names.len()
        );
        names.truncate(MAX_MEMBERS_PER_REQUEST);
    }

    match add_members(pool, player_id, list_id, &names).await {
        Ok(added) if !added.is_empty() => {
            tracing::info!(
                entity_id,
                player_id,
                list_id,
                count = added.len(),
                "ContactList: members added"
            );
            let args = build_on_contact_list_add_members(list_id, &added);
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
                        &args,
                        version,
                    )
                },
            )
            .await;

            // Light up any just-added contact who is ALREADY online, so the
            // adder sees them online immediately. Mirrors the login reverse
            // sync (`notify_online_contacts`); without it, a freshly-added
            // online friend stays dim until one side relogs.
            super::notify_online_contacts(entity_id, &added, transport, connected, entity_to_addr)
                .await;
            resync_if_ignore_list(
                entity_id,
                player_id,
                list_id,
                db_pool,
                connected,
                entity_to_addr,
                cell_tx,
            )
            .await;
            return added;
        }
        Ok(_) => {
            // All names were duplicates — nothing to echo.
            tracing::debug!(
                entity_id,
                player_id,
                list_id,
                "ContactListAddMembers: all duplicates, no echo"
            );
        }
        Err(sqlx::Error::RowNotFound) => {
            tracing::warn!(
                entity_id,
                player_id,
                list_id,
                "ContactListAddMembers: list not found or not owned"
            );
        }
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                list_id,
                "ContactListAddMembers: DB error: {e}"
            );
        }
    }
    Vec::new()
}

/// Handle `ContactListRemoveMembers` — delete members and echo CM 88. Returns
/// the names actually removed.
pub async fn handle_remove_members(
    entity_id: u32,
    player_id: i32,
    list_id: i32,
    mut names: Vec<String>,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) -> Vec<String> {
    let pool = match db_pool {
        Some(p) => p.as_ref(),
        None => {
            tracing::warn!(
                entity_id,
                player_id,
                list_id,
                "ContactListRemoveMembers: no DB pool"
            );
            return Vec::new();
        }
    };

    if names.len() > MAX_MEMBERS_PER_REQUEST {
        tracing::warn!(
            entity_id,
            player_id,
            list_id,
            count = names.len(),
            "ContactListRemoveMembers: clamping names array from {} to {MAX_MEMBERS_PER_REQUEST}",
            names.len()
        );
        names.truncate(MAX_MEMBERS_PER_REQUEST);
    }

    match remove_members(pool, player_id, list_id, &names).await {
        Ok(removed) if !removed.is_empty() => {
            tracing::info!(
                entity_id,
                player_id,
                list_id,
                count = removed.len(),
                "ContactList: members removed"
            );
            let args = build_on_contact_list_remove_members(list_id, &removed);
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
                        method_idx::ON_CONTACT_LIST_REMOVE_MEMBERS,
                        &args,
                        version,
                    )
                },
            )
            .await;
            resync_if_ignore_list(
                entity_id,
                player_id,
                list_id,
                db_pool,
                connected,
                entity_to_addr,
                cell_tx,
            )
            .await;
            return removed;
        }
        Ok(_) => {
            tracing::debug!(
                entity_id,
                player_id,
                list_id,
                "ContactListRemoveMembers: none of the named members were present"
            );
        }
        Err(sqlx::Error::RowNotFound) => {
            tracing::warn!(
                entity_id,
                player_id,
                list_id,
                "ContactListRemoveMembers: list not found or not owned"
            );
        }
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                list_id,
                "ContactListRemoveMembers: DB error: {e}"
            );
        }
    }
    Vec::new()
}

/// After a member change on `list_id`: when it is the Ignore list, reload the
/// Ignore cache on the session and the cell. Any other list is left alone.
async fn resync_if_ignore_list(
    entity_id: u32,
    player_id: i32,
    list_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) {
    let Some(pool) = db_pool else { return };
    let addr = entity_to_addr.lock().unwrap().get(&entity_id).copied();
    // For the failure logs (instrumentation rule 5).
    let account_id = addr.and_then(|a| connected.lock().unwrap().get(&a).map(|c| c.account_id));
    match load_list_header(pool, player_id, list_id).await {
        Ok(Some((_, flags))) if flags == IGNORE_LIST_FLAGS => {}
        Ok(_) => return,
        Err(e) => {
            tracing::error!(
                target: "chat",
                event = "chat.ignore_sync_failed",
                entity_id,
                player_id,
                account_id,
                list_id,
                reason = "db_error",
                error = %e,
                "could not tell whether the changed list is the Ignore list; cache not reloaded",
            );
            return;
        }
    }
    let Some(addr) = addr else {
        tracing::debug!(
            target: "chat",
            event = "chat.ignore_sync_failed",
            entity_id,
            player_id,
            account_id,
            list_id,
            reason = "entity_to_addr_miss",
            "Ignore list changed for an entity with no session; the next world entry reloads it",
        );
        return;
    };
    let ctx = IgnoreSyncCtx {
        db_pool,
        connected,
        cell_tx,
    };
    resync_ignore_cache(ctx, addr, player_id, entity_id, "contact_list").await;
}
