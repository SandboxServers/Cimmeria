//! Persistence leg of the once-per-character loot gate
//! (`CellToBaseMsg::ContainerLooted`, the content `open_loot` action).
//!
//! The cell owns the live check (`CellEntity::looted_containers`, stamped by
//! `InitPlayerState` from `sgw_player.looted_containers`); this appends the
//! key so the next world entry hydrates it and a relog or respawn never
//! re-rolls the chest. Decision (@Cadacious, 2026-09-28).
//!
//! Shaped like `gate_travel::address_grant`: the account is resolved from the
//! session and used as the ownership predicate, and the append is a
//! set-difference so a duplicated message cannot double-append.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;

use cimmeria_entity::cell_entity::PlayerIdentity;

use super::super::session_identity::{identity_for_entity, session_identity};
use super::super::ConnectedClientState;

/// Append `container_key` to the player's looted containers, exactly once.
/// `Ok(None)` when the `(player_id, account_id)` pair matched no row.
pub(crate) async fn append_looted_container(
    pool: &PgPool,
    player_id: i32,
    account_id: i32,
    container_key: &str,
) -> Result<Option<Vec<String>>, sqlx::Error> {
    sqlx::query_scalar::<_, Vec<String>>(
        "UPDATE sgw_player \
            SET looted_containers = CASE \
                  WHEN $1::varchar = ANY(looted_containers) THEN looted_containers \
                  ELSE array_append(looted_containers, $1::varchar) \
                END \
          WHERE player_id = $2 AND account_id = $3 \
      RETURNING looted_containers::text[]",
    )
    .bind(container_key)
    .bind(player_id)
    .bind(account_id)
    .fetch_optional(pool)
    .await
}

/// Handle `CellToBaseMsg::ContainerLooted`.
pub(crate) async fn handle_container_looted(
    entity_id: u32,
    player_id: i32,
    container_key: String,
    db_pool: &Option<Arc<PgPool>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let Some(pool) = db_pool else {
        let who = identity_for_entity(connected, entity_to_addr, entity_id);
        tracing::warn!(
            target: "inventory",
            entity_id,
            entity_name = who.player_name,
            player_id,
            player_name = who.player_name,
            %container_key,
            reason = "looted_container_no_db_pool",
            "ContainerLooted: no DB pool -- the once-per-character flag holds for this \
             session only and is lost on relog"
        );
        return;
    };
    let addr = entity_to_addr.lock().unwrap().get(&entity_id).copied();
    // The session's identity: its account gates the UPDATE, and its names
    // pair the IDs on every line below (Rule 6).
    let who = addr
        .and_then(|addr| {
            let clients = match connected.lock() {
                Ok(c) => c,
                Err(poisoned) => poisoned.into_inner(),
            };
            clients.get(&addr).map(session_identity)
        })
        .unwrap_or(PlayerIdentity::UNKNOWN);
    let Some(account_id) = who.account_id else {
        tracing::warn!(
            target: "inventory",
            entity_id,
            entity_name = who.player_name,
            player_id,
            player_name = who.player_name,
            %container_key,
            reason = "looted_container_no_session",
            "ContainerLooted: no session for the entity -- cannot resolve the owning \
             account, so the flag is not persisted"
        );
        return;
    };

    match append_looted_container(pool.as_ref(), player_id, account_id as i32, &container_key).await
    {
        Ok(Some(keys)) => tracing::info!(
            target: "inventory",
            event = "container_looted_persisted",
            entity_id,
            entity_name = who.player_name,
            player_id,
            player_name = who.player_name,
            account_id,
            account_name = who.account_name,
            %container_key,
            looted_count = keys.len(),
            "ContainerLooted: once-per-character flag persisted"
        ),
        Ok(None) => tracing::warn!(
            target: "inventory",
            entity_id,
            entity_name = who.player_name,
            player_id,
            player_name = who.player_name,
            account_id,
            account_name = who.account_name,
            %container_key,
            rows_affected = 0,
            expected = 1,
            reason = "looted_container_rows_affected_zero",
            "ContainerLooted: UPDATE matched 0 rows -- the player/account pair names no \
             character; the flag holds for this session only"
        ),
        Err(e) => tracing::error!(
            target: "inventory",
            entity_id,
            entity_name = who.player_name,
            player_id,
            player_name = who.player_name,
            account_id,
            account_name = who.account_name,
            %container_key,
            reason = "looted_container_persist_failed",
            "ContainerLooted: failed to persist the flag ({e}) -- it holds for this \
             session only"
        ),
    }
}
