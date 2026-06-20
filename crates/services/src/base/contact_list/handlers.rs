//! BaseApp-side handlers for contact-list CellToBaseMsg variants.
//!
//! Each handler: validates ownership (DB), mutates the DB, then echoes
//! the appropriate S→C client method (CM 85–89) to the requesting player.
//!
//! Max members per request is clamped to MAX_MEMBERS_PER_REQUEST to prevent
//! abuse from malformed or malicious add/remove arrays.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use crate::base::contact_list::persistence::{
    add_members, create_list, delete_list, find_watchers, load_contact_lists, load_list_header,
    remove_members, rename_list, update_flags,
};
use crate::base::contact_list::wire::{
    build_on_contact_list_add_members, build_on_contact_list_delete, build_on_contact_list_event,
    build_on_contact_list_remove_members, build_on_contact_list_update, DATA_OFFLINE, DATA_ONLINE,
    EVENT_LOGGED_IN_STATUS,
};
use crate::base::helpers::send_to_witness_reliable;
use crate::base::ConnectedClientState;
use crate::mercury::{build_player_entity_method_packet, method_idx};

/// Hard limit on names per ADD/REMOVE request. Prevents abuse.
const MAX_MEMBERS_PER_REQUEST: usize = 100;

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
    if let Err(e) =
        crate::base::contact_list::persistence::ensure_system_lists(pool, player_id).await
    {
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
            |key, seq, acks| {
                build_player_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_CONTACT_LIST_UPDATE,
                    &update_args,
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
            |key, seq, acks| {
                build_player_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_CONTACT_LIST_ADD_MEMBERS,
                    &members_args,
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
                |key, seq, acks| {
                    build_player_entity_method_packet(
                        key,
                        seq,
                        acks,
                        entity_id,
                        method_idx::ON_CONTACT_LIST_UPDATE,
                        &args,
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
                |key, seq, acks| {
                    build_player_entity_method_packet(
                        key,
                        seq,
                        acks,
                        entity_id,
                        method_idx::ON_CONTACT_LIST_DELETE,
                        &args,
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
                        |key, seq, acks| {
                            build_player_entity_method_packet(
                                key,
                                seq,
                                acks,
                                entity_id,
                                method_idx::ON_CONTACT_LIST_UPDATE,
                                &args,
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
                    |key, seq, acks| {
                        build_player_entity_method_packet(
                            key,
                            seq,
                            acks,
                            entity_id,
                            method_idx::ON_CONTACT_LIST_UPDATE,
                            &args,
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
                |key, seq, acks| {
                    build_player_entity_method_packet(
                        key,
                        seq,
                        acks,
                        entity_id,
                        method_idx::ON_CONTACT_LIST_ADD_MEMBERS,
                        &args,
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
                |key, seq, acks| {
                    build_player_entity_method_packet(
                        key,
                        seq,
                        acks,
                        entity_id,
                        method_idx::ON_CONTACT_LIST_REMOVE_MEMBERS,
                        &args,
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

/// Fan out `onContactListEvent` (CM 89, eventId=LoggedInStatus) to all online
/// players who have `player_name` in any of their contact lists.
///
/// Called on player login (`data_value = DATA_ONLINE`) and logout
/// (`data_value = DATA_OFFLINE`).
///
/// # Invariant #5 (ignore-list leak)
/// This function is for the *caller's* presence fanout only — it does NOT
/// check the recipient's ignore list because the spec does not filter
/// LoggedInStatus events on the notifier's side. If that becomes a
/// requirement (e.g., muted players), add a join against the recipient's
/// Ignore list in `find_watchers` or filter here before the send.
///
/// # TODO(#572 Phase 5)
/// GainLevel / Death / GateTravel events (eventId 1/2/3) are deferred —
/// their data values need x64dbg confirmation (D.3).
pub(crate) async fn fanout_login_status(
    player_name: &str,
    is_online: bool,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(p) => p.as_ref(),
        None => {
            tracing::warn!(
                player_name,
                "ContactList LoginStatus fanout: no DB pool, skipping"
            );
            return;
        }
    };

    let watcher_player_ids = match find_watchers(pool, player_name).await {
        Ok(ids) => ids,
        Err(e) => {
            tracing::error!(
                player_name,
                "ContactList LoginStatus fanout: find_watchers failed: {e}"
            );
            return;
        }
    };

    if watcher_player_ids.is_empty() {
        return;
    }

    let data_value = if is_online { DATA_ONLINE } else { DATA_OFFLINE };
    let args = build_on_contact_list_event(player_name, EVENT_LOGGED_IN_STATUS, data_value);

    // Map player_id → entity_id for online players only.
    // We take the entity_to_addr lock to get the reverse mapping from
    // active_player_id → entity_id stored on ConnectedClientState.
    let watcher_entity_ids: Vec<u32> = {
        let clients = match connected.lock() {
            Ok(g) => g,
            Err(_) => {
                tracing::error!(
                    player_name,
                    "ContactList LoginStatus fanout: connected lock poisoned"
                );
                return;
            }
        };
        clients
            .values()
            .filter_map(|state| {
                state
                    .active_player_id
                    .filter(|pid| watcher_player_ids.contains(pid))
                    .and(state.player_entity_id)
            })
            .collect()
    };

    for entity_id in watcher_entity_ids {
        let args_clone = args.clone();
        send_to_witness_reliable(
            transport,
            connected,
            entity_to_addr,
            entity_id,
            move |key, seq, acks| {
                build_player_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_CONTACT_LIST_EVENT,
                    &args_clone,
                )
            },
        )
        .await;
    }

    tracing::debug!(
        player_name,
        is_online,
        watcher_count = watcher_player_ids.len(),
        "ContactList: LoginStatus fanout complete"
    );
}
