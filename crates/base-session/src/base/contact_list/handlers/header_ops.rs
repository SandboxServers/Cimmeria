//! Contact-list header operations: create, delete, rename, flags_update.
//!
//! Each handler: validates ownership (DB), mutates the DB, then echoes the
//! appropriate S→C client method (CM 85–86) to the requesting player.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use crate::base::contact_list::persistence::{
    create_list, delete_list, load_list_header, rename_list, update_flags,
};
use crate::base::contact_list::wire::{build_on_contact_list_delete, build_on_contact_list_update};
use crate::base::helpers::send_to_witness_reliable;
use crate::base::ConnectedClientState;
use crate::mercury::{build_player_entity_method_packet, method_idx};

/// Handle `ContactListCreate` — insert a new list and echo CM 85.
pub async fn handle_create(
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
            let who = crate::base::session_identity::identity_for_entity(
                connected,
                entity_to_addr,
                entity_id,
            );
            tracing::warn!(
                entity_id,
                entity_name = who.player_name,
                player_id,
                player_name = who.player_name,
                "ContactListCreate: no DB pool, dropping"
            );
            return;
        }
    };

    match create_list(pool, player_id, &name, flags).await {
        Ok(list_id) => {
            let who = crate::base::session_identity::identity_for_entity(
                connected,
                entity_to_addr,
                entity_id,
            );
            tracing::info!(
                entity_id,
                entity_name = who.player_name,
                player_id,
                player_name = who.player_name,
                list_id, // nt:id-only list row is player-owned, no name loaded at this site
                name,
                "ContactList: created"
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
        Err(e) => {
            let who = crate::base::session_identity::identity_for_entity(
                connected,
                entity_to_addr,
                entity_id,
            );
            tracing::warn!(
                entity_id,
                entity_name = who.player_name,
                player_id,
                player_name = who.player_name,
                name,
                "ContactListCreate: DB error (duplicate name?): {e}"
            );
        }
    }
}

/// Handle `ContactListDelete` — delete a list and echo CM 86.
pub async fn handle_delete(
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
            let who = crate::base::session_identity::identity_for_entity(
                connected,
                entity_to_addr,
                entity_id,
            );
            tracing::warn!(
                entity_id,
                entity_name = who.player_name,
                player_id,
                player_name = who.player_name,
                list_id, // nt:id-only list row is player-owned, no name loaded at this site
                "ContactListDelete: no DB pool"
            );
            return;
        }
    };

    match delete_list(pool, player_id, list_id).await {
        Ok(true) => {
            let who = crate::base::session_identity::identity_for_entity(
                connected,
                entity_to_addr,
                entity_id,
            );
            tracing::info!(
                entity_id,
                entity_name = who.player_name,
                player_id,
                player_name = who.player_name,
                list_id, // nt:id-only list row is player-owned, no name loaded at this site
                "ContactList: deleted"
            );
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
            let who = crate::base::session_identity::identity_for_entity(
                connected,
                entity_to_addr,
                entity_id,
            );
            tracing::warn!(
                entity_id,
                entity_name = who.player_name,
                player_id,
                player_name = who.player_name,
                list_id, // nt:id-only list row is player-owned, no name loaded at this site
                "ContactListDelete: list not found or not owned by player"
            );
        }
        Err(e) => {
            let who = crate::base::session_identity::identity_for_entity(
                connected,
                entity_to_addr,
                entity_id,
            );
            tracing::error!(
                entity_id,
                entity_name = who.player_name,
                player_id,
                player_name = who.player_name,
                list_id, // nt:id-only list row is player-owned, no name loaded at this site
                "ContactListDelete: DB error: {e}"
            );
        }
    }
}

/// Handle `ContactListRename` — update name and echo CM 85.
pub async fn handle_rename(
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
            let who = crate::base::session_identity::identity_for_entity(
                connected,
                entity_to_addr,
                entity_id,
            );
            tracing::warn!(
                entity_id,
                entity_name = who.player_name,
                player_id,
                player_name = who.player_name,
                list_id, // nt:id-only list row is player-owned, no name loaded at this site
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
                    let who = crate::base::session_identity::identity_for_entity(
                        connected,
                        entity_to_addr,
                        entity_id,
                    );
                    tracing::info!(
                        entity_id,
                        entity_name = who.player_name,
                        player_id,
                        player_name = who.player_name,
                        list_id, // nt:id-only list row is player-owned, no name loaded at this site
                        name,
                        "ContactList: renamed"
                    );
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
                    let who = crate::base::session_identity::identity_for_entity(
                        connected,
                        entity_to_addr,
                        entity_id,
                    );
                    tracing::warn!(
                        entity_id,
                        entity_name = who.player_name,
                        player_id,
                        player_name = who.player_name,
                        list_id, // nt:id-only list row is player-owned, no name loaded at this site
                        "ContactListRename: list disappeared between rename and reload"
                    );
                }
                Err(e) => {
                    let who = crate::base::session_identity::identity_for_entity(
                        connected,
                        entity_to_addr,
                        entity_id,
                    );
                    tracing::error!(
                        entity_id,
                        entity_name = who.player_name,
                        player_id,
                        player_name = who.player_name,
                        list_id, // nt:id-only list row is player-owned, no name loaded at this site
                        "ContactListRename: reload after rename failed: {e}"
                    );
                }
            }
        }
        Ok(false) => {
            let who = crate::base::session_identity::identity_for_entity(
                connected,
                entity_to_addr,
                entity_id,
            );
            tracing::warn!(
                entity_id,
                entity_name = who.player_name,
                player_id,
                player_name = who.player_name,
                list_id, // nt:id-only list row is player-owned, no name loaded at this site
                "ContactListRename: list not found or not owned"
            );
        }
        Err(e) => {
            let who = crate::base::session_identity::identity_for_entity(
                connected,
                entity_to_addr,
                entity_id,
            );
            tracing::error!(
                entity_id,
                entity_name = who.player_name,
                player_id,
                player_name = who.player_name,
                list_id, // nt:id-only list row is player-owned, no name loaded at this site
                "ContactListRename: DB error: {e}"
            );
        }
    }
}

/// Handle `ContactListFlagsUpdate` — update flags and echo CM 85.
pub async fn handle_flags_update(
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
            let who = crate::base::session_identity::identity_for_entity(
                connected,
                entity_to_addr,
                entity_id,
            );
            tracing::warn!(
                entity_id,
                entity_name = who.player_name,
                player_id,
                player_name = who.player_name,
                list_id, // nt:id-only list row is player-owned, no name loaded at this site
                "ContactListFlagsUpdate: no DB pool"
            );
            return;
        }
    };

    match update_flags(pool, player_id, list_id, flags).await {
        Ok(true) => match load_list_header(pool, player_id, list_id).await {
            Ok(Some((name, _))) => {
                let who = crate::base::session_identity::identity_for_entity(
                    connected,
                    entity_to_addr,
                    entity_id,
                );
                tracing::info!(
                    entity_id,
                    entity_name = who.player_name,
                    player_id,
                    player_name = who.player_name,
                    list_id, // nt:id-only list row is player-owned, no name loaded at this site
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
                let who = crate::base::session_identity::identity_for_entity(
                    connected,
                    entity_to_addr,
                    entity_id,
                );
                tracing::warn!(
                    entity_id,
                    entity_name = who.player_name,
                    player_id,
                    player_name = who.player_name,
                    list_id, // nt:id-only list row is player-owned, no name loaded at this site
                    "ContactListFlagsUpdate: list disappeared between update and reload"
                );
            }
            Err(e) => {
                let who = crate::base::session_identity::identity_for_entity(
                    connected,
                    entity_to_addr,
                    entity_id,
                );
                tracing::error!(
                    entity_id,
                    entity_name = who.player_name,
                    player_id,
                    player_name = who.player_name,
                    list_id, // nt:id-only list row is player-owned, no name loaded at this site
                    "ContactListFlagsUpdate: reload failed: {e}"
                );
            }
        },
        Ok(false) => {
            let who = crate::base::session_identity::identity_for_entity(
                connected,
                entity_to_addr,
                entity_id,
            );
            tracing::warn!(
                entity_id,
                entity_name = who.player_name,
                player_id,
                player_name = who.player_name,
                list_id, // nt:id-only list row is player-owned, no name loaded at this site
                "ContactListFlagsUpdate: list not found or not owned"
            );
        }
        Err(e) => {
            let who = crate::base::session_identity::identity_for_entity(
                connected,
                entity_to_addr,
                entity_id,
            );
            tracing::error!(
                entity_id,
                entity_name = who.player_name,
                player_id,
                player_name = who.player_name,
                list_id, // nt:id-only list row is player-owned, no name loaded at this site
                "ContactListFlagsUpdate: DB error: {e}"
            );
        }
    }
}
