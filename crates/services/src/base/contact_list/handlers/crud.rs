//! Per-operation contact-list handlers: create, delete, rename, flags_update,
//! add_members, remove_members.
//!
//! Each handler: validates ownership (DB), mutates the DB, then echoes the
//! appropriate S→C client method (CM 85–89) to the requesting player.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use crate::base::contact_list::persistence::{
    add_members, create_list, delete_list, load_list_header, remove_members, rename_list,
    update_flags,
};
use crate::base::contact_list::wire::{
    build_on_contact_list_add_members, build_on_contact_list_delete,
    build_on_contact_list_remove_members, build_on_contact_list_update,
};
use crate::base::helpers::send_to_witness_reliable;
use crate::base::ConnectedClientState;
use crate::mercury::{build_player_entity_method_packet, method_idx};

/// Hard limit on names per ADD/REMOVE request. Prevents abuse.
pub(crate) const MAX_MEMBERS_PER_REQUEST: usize = 100;

/// Handle `ContactListCreate` — insert a new list and echo CM 85.
pub(crate) async fn handle_create(
    entity_id: u32,
    player_id: i32,
    name: String,
    flags: u32,
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
                "ContactListCreate: no DB pool, dropping"
            );
            return;
        }
    };

    match create_list(pool, player_id, &name, flags).await {
        Ok(list_id) => {
            tracing::info!(entity_id, player_id, list_id, name, "ContactList: created");
            let args = build_on_contact_list_update(list_id, &name, flags);
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
                        &args,
                        version,
                    )
                },
            )
            .await;
        }
        Err(e) => {
            tracing::warn!(
                entity_id,
                player_id,
                name,
                "ContactListCreate: DB error (duplicate name?): {e}"
            );
        }
    }
}

/// Handle `ContactListDelete` — delete a list and echo CM 86.
pub(crate) async fn handle_delete(
    entity_id: u32,
    player_id: i32,
    list_id: i32,
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
                list_id,
                "ContactListDelete: no DB pool"
            );
            return;
        }
    };

    match delete_list(pool, player_id, list_id).await {
        Ok(true) => {
            tracing::info!(entity_id, player_id, list_id, "ContactList: deleted");
            let args = build_on_contact_list_delete(list_id);
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
                        method_idx::ON_CONTACT_LIST_DELETE,
                        &args,
                        version,
                    )
                },
            )
            .await;
        }
        Ok(false) => {
            tracing::warn!(
                entity_id,
                player_id,
                list_id,
                "ContactListDelete: list not found or not owned by player"
            );
        }
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                list_id,
                "ContactListDelete: DB error: {e}"
            );
        }
    }
}

/// Handle `ContactListRename` — update name and echo CM 85.
pub(crate) async fn handle_rename(
    entity_id: u32,
    player_id: i32,
    list_id: i32,
    name: String,
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
                list_id,
                "ContactListRename: no DB pool"
            );
            return;
        }
    };

    match rename_list(pool, player_id, list_id, &name).await {
        Ok(true) => {
            // Re-read flags to compose the full CM 85 response.
            match load_list_header(pool, player_id, list_id).await {
                Ok(Some((_, flags))) => {
                    tracing::info!(entity_id, player_id, list_id, name, "ContactList: renamed");
                    let args = build_on_contact_list_update(list_id, &name, flags as u32);
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
                                &args,
                                version,
                            )
                        },
                    )
                    .await;
                }
                Ok(None) => {
                    tracing::warn!(
                        entity_id,
                        player_id,
                        list_id,
                        "ContactListRename: list disappeared between rename and reload"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        entity_id,
                        player_id,
                        list_id,
                        "ContactListRename: reload after rename failed: {e}"
                    );
                }
            }
        }
        Ok(false) => {
            tracing::warn!(
                entity_id,
                player_id,
                list_id,
                "ContactListRename: list not found or not owned"
            );
        }
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                list_id,
                "ContactListRename: DB error: {e}"
            );
        }
    }
}

/// Handle `ContactListFlagsUpdate` — update flags and echo CM 85.
pub(crate) async fn handle_flags_update(
    entity_id: u32,
    player_id: i32,
    list_id: i32,
    flags: u32,
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
                list_id,
                "ContactListFlagsUpdate: no DB pool"
            );
            return;
        }
    };

    match update_flags(pool, player_id, list_id, flags).await {
        Ok(true) => match load_list_header(pool, player_id, list_id).await {
            Ok(Some((name, _))) => {
                tracing::info!(
                    entity_id,
                    player_id,
                    list_id,
                    flags,
                    "ContactList: flags updated"
                );
                let args = build_on_contact_list_update(list_id, &name, flags);
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
                            &args,
                            version,
                        )
                    },
                )
                .await;
            }
            Ok(None) => {
                tracing::warn!(
                    entity_id,
                    player_id,
                    list_id,
                    "ContactListFlagsUpdate: list disappeared between update and reload"
                );
            }
            Err(e) => {
                tracing::error!(
                    entity_id,
                    player_id,
                    list_id,
                    "ContactListFlagsUpdate: reload failed: {e}"
                );
            }
        },
        Ok(false) => {
            tracing::warn!(
                entity_id,
                player_id,
                list_id,
                "ContactListFlagsUpdate: list not found or not owned"
            );
        }
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                list_id,
                "ContactListFlagsUpdate: DB error: {e}"
            );
        }
    }
}

/// Handle `ContactListAddMembers` — insert members and echo CM 87.
pub(crate) async fn handle_add_members(
    entity_id: u32,
    player_id: i32,
    list_id: i32,
    mut names: Vec<String>,
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
                list_id,
                "ContactListAddMembers: no DB pool"
            );
            return;
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
}

/// Handle `ContactListRemoveMembers` — delete members and echo CM 88.
pub(crate) async fn handle_remove_members(
    entity_id: u32,
    player_id: i32,
    list_id: i32,
    mut names: Vec<String>,
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
                list_id,
                "ContactListRemoveMembers: no DB pool"
            );
            return;
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
}
