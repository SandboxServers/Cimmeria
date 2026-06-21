//! SGWPlayer base-method `chatIgnore` handler.
//!
//! `chatIgnore(WSTRING playerName, UINT8 flag)` — flag 1 adds the named
//! player to this player's 'Ignore' contact list (flags=301), flag 0 removes
//! them. The Ignore list is the single source of truth, so /ignore and the
//! contact-list UI always agree.
//!
//! Side effects beyond the DB write:
//! - echoes the matching CM (87 add / 88 remove) so the client's contact-list
//!   UI updates,
//! - updates the in-memory `ConnectedClientState::ignore_set`,
//! - pushes `BaseToCellMsg::UpdateIgnoreList` so the cell-side AoI filter
//!   re-evaluates witness sets (which gates spatial chat, combat fanout, and
//!   position updates for the ignored pair).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::base::contact_list::handlers::{handle_add_members, handle_remove_members};
use crate::base::contact_list::persistence::{
    ensure_system_lists, load_contact_lists, load_list_header,
};
use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;
use crate::mercury::read_wstring;

/// Flags value identifying the system 'Ignore' contact list.
const IGNORE_LIST_FLAGS: i32 = 301;

/// Handle `chatIgnore(WSTRING playerName, UINT8 flag)`.
///
/// `flag == 1` adds `playerName` to the Ignore list; any other value removes
/// it. The `.def` file carries both args even though the dispatch table lists
/// only the WSTRING — the UINT8 flag is authoritative.
#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_chat_ignore(
    payload: &[u8],
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<PgPool>>,
) {
    // Parse WSTRING playerName, then the trailing UINT8 flag.
    let (target_name, name_bytes) = match read_wstring(payload, 0) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                %addr,
                payload_len = payload.len(),
                error = %e,
                "chatIgnore: WSTRING decode failed -- ignore request dropped"
            );
            return;
        }
    };
    // flag follows the name WSTRING; default to remove (0) if truncated.
    let add = payload.get(name_bytes).copied().unwrap_or(0) == 1;

    if target_name.is_empty() {
        tracing::warn!(%addr, "chatIgnore: empty target name -- dropped");
        return;
    }

    // Resolve the session's player_id + entity_id.
    let (player_id, entity_id) = {
        let clients = match connected.lock() {
            Ok(c) => c,
            Err(_) => {
                tracing::error!(%addr, "chatIgnore: connected lock poisoned");
                return;
            }
        };
        match clients.get(&addr) {
            Some(c) => match (c.active_player_id, c.player_entity_id) {
                (Some(pid), Some(eid)) => (pid, eid),
                _ => {
                    tracing::warn!(
                        %addr,
                        "chatIgnore: session not in-world (no player_id/entity_id) -- dropped"
                    );
                    return;
                }
            },
            None => return,
        }
    };

    let pool = match db_pool {
        Some(p) => p.as_ref(),
        None => {
            tracing::warn!(entity_id, player_id, "chatIgnore: no DB pool -- dropped");
            return;
        }
    };

    // Ensure the Ignore list exists (idempotent) and grab its id.
    let ignore_list_id = match ensure_system_lists(pool, player_id).await {
        Ok((_friends, ignore)) => ignore,
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                "chatIgnore: ensure_system_lists failed: {e}"
            );
            return;
        }
    };

    // Apply the add/remove via the shared member-ops handlers, which also echo
    // CM 87/88 to the client so the contact-list UI reflects the change.
    let names = vec![target_name.clone()];
    if add {
        handle_add_members(
            entity_id,
            player_id,
            ignore_list_id,
            names,
            db_pool,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    } else {
        handle_remove_members(
            entity_id,
            player_id,
            ignore_list_id,
            names,
            db_pool,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }

    // Update the in-memory ignore_set and snapshot it for the cell sync.
    let ignore_snapshot = {
        let mut clients = match connected.lock() {
            Ok(c) => c,
            Err(_) => {
                tracing::error!(%addr, "chatIgnore: connected lock poisoned on update");
                return;
            }
        };
        match clients.get_mut(&addr) {
            Some(c) => {
                if add {
                    c.ignore_set.insert(target_name.clone());
                } else {
                    c.ignore_set.remove(&target_name);
                }
                c.ignore_set.clone()
            }
            None => return,
        }
    };

    tracing::info!(
        entity_id,
        player_id,
        add,
        ignore_count = ignore_snapshot.len(),
        "chatIgnore: ignore list updated"
    );

    // Push the updated set to the cell so the AoI filter re-evaluates.
    if let Some(ref tx) = cell_tx {
        let _ = tx
            .send(BaseToCellMsg::UpdateIgnoreList {
                entity_id,
                ignore_names: ignore_snapshot,
            })
            .await;
    }
}

/// Re-sync the ignore cache after a contact-list member add/remove, but only
/// if `list_id` is the player's 'Ignore' list (flags=301).
///
/// The contact-list UI lets a player edit the Ignore list directly (not just
/// via /ignore), so add/remove on that list must update the in-memory
/// `ignore_set` and re-push `UpdateIgnoreList` to the cell — otherwise an
/// ignore added through the UI would persist but never gate AoI/chat until
/// relog. Other lists (Friends, custom) are left untouched.
///
/// Reloads the Ignore list members from the DB rather than trusting the
/// just-changed name set, so a duplicate-add or no-op-remove (which the member
/// ops dedupe) doesn't desync the cache.
pub(crate) async fn resync_ignore_after_member_change(
    entity_id: u32,
    player_id: i32,
    list_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) {
    let pool = match db_pool {
        Some(p) => p.as_ref(),
        None => return,
    };

    // Cheap gate: only proceed if this is the Ignore list.
    match load_list_header(pool, player_id, list_id).await {
        Ok(Some((_name, flags))) if flags == IGNORE_LIST_FLAGS => {}
        Ok(_) => return, // not the Ignore list (or not owned) — nothing to do
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                list_id,
                "ignore resync: load_list_header failed: {e}"
            );
            return;
        }
    }

    // Reload the authoritative Ignore membership.
    let ignore_names: std::collections::HashSet<String> =
        match load_contact_lists(pool, player_id).await {
            Ok(lists) => lists
                .into_iter()
                .filter(|l| l.flags == IGNORE_LIST_FLAGS)
                .flat_map(|l| l.members.into_iter())
                .collect(),
            Err(e) => {
                tracing::error!(
                    entity_id,
                    player_id,
                    "ignore resync: load_contact_lists failed: {e}"
                );
                return;
            }
        };

    // Update the in-memory cache.
    let addr = entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&entity_id).copied());
    if let Some(addr) = addr {
        if let Ok(mut clients) = connected.lock() {
            if let Some(c) = clients.get_mut(&addr) {
                c.ignore_set = ignore_names.clone();
            }
        }
    }

    // Push to the cell so the AoI filter re-evaluates.
    if let Some(ref tx) = cell_tx {
        let _ = tx
            .send(BaseToCellMsg::UpdateIgnoreList {
                entity_id,
                ignore_names,
            })
            .await;
    }
}

#[cfg(test)]
mod tests;
