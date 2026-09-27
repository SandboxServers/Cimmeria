//! The three-copy resync: reload the Ignore list from the database, write
//! the base session's copy, push the cell's (`UpdateIgnoreList`), newest
//! read wins. See the parent module for when it runs.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::sync::mpsc;

use super::{load_ignore_names, load_ignored_player_ids};
use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;

/// What [`resync_ignore_cache`] needs from the base.
#[derive(Clone, Copy)]
pub struct IgnoreSyncCtx<'a> {
    pub db_pool: &'a Option<Arc<PgPool>>,
    pub connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub cell_tx: &'a Option<mpsc::Sender<BaseToCellMsg>>,
}

/// Reload `player_id`'s Ignore list from the database, store it on the
/// session at `addr`, and push it to the cell entity `entity_id`. `path`
/// names the caller in the log (`world_entry`, `chat_ignore`,
/// `contact_list`). Returns the new set, or `None` when nothing could be
/// loaded (no pool, DB error), in which case the old copies are kept.
pub async fn resync_ignore_cache(
    ctx: IgnoreSyncCtx<'_>,
    addr: SocketAddr,
    player_id: i32,
    entity_id: u32,
    path: &'static str,
) -> Option<HashSet<String>> {
    // For the failure logs before the session is re-read below.
    let session_account_id = ctx
        .connected
        .lock()
        .unwrap()
        .get(&addr)
        .map(|c| c.account_id);
    let Some(pool) = ctx.db_pool else {
        tracing::warn!(
            target: "chat",
            event = "chat.ignore_sync_failed",
            %addr,
            player_id,
            account_id = session_account_id,
            entity_id,
            path,
            reason = "no_db_pool",
            "Ignore list not loaded: no DB pool; tells and spatial chat ignore nobody",
        );
        return None;
    };
    // Take a version before reading, so the newest read wins even when
    // resyncs from different tasks (the client loop, the cell-message loop)
    // overlap and finish out of order.
    let version = {
        let mut clients = ctx.connected.lock().unwrap();
        match clients.get_mut(&addr) {
            Some(c) if c.active_player_id == Some(player_id) => Some(c.ignore.begin_sync()),
            _ => None,
        }
    };
    let Some(version) = version else {
        tracing::debug!(
            target: "chat",
            event = "chat.ignore_sync_failed",
            %addr,
            player_id,
            account_id = session_account_id,
            entity_id,
            path,
            reason = "session_changed",
            "Ignore resync for a session that no longer plays this character; skipped",
        );
        return None;
    };
    let loaded = match load_ignore_names(pool, player_id).await {
        Ok(n) => load_ignored_player_ids(pool, player_id)
            .await
            .map(|ids| (n, ids)),
        Err(e) => Err(e),
    };
    let (names, ignored_ids) = match loaded {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                target: "chat",
                event = "chat.ignore_sync_failed",
                %addr,
                player_id,
                account_id = session_account_id,
                entity_id,
                path,
                reason = "db_error",
                error = %e,
                "Ignore list reload failed; the previous copy is kept",
            );
            return None;
        }
    };

    let (account_id, synced) = {
        let mut clients = ctx.connected.lock().unwrap();
        match clients.get_mut(&addr) {
            // Only the session still playing this character takes the set: a
            // logOff to character select between the load and here must not
            // hand char A's list to char B.
            Some(c) if c.active_player_id == Some(player_id) => {
                let before = c.ignore.len();
                if c.ignore.apply_sync(version, &names, ignored_ids) {
                    (Some(c.account_id), Ok((c.account_id, before)))
                } else {
                    (Some(c.account_id), Err("stale_version"))
                }
            }
            Some(c) => (Some(c.account_id), Err("session_changed")),
            None => (None, Err("session_changed")),
        }
    };
    let (session_account_id, before) = match synced {
        Ok(v) => v,
        Err(reason) => {
            tracing::debug!(
                target: "chat",
                event = "chat.ignore_sync_failed",
                %addr,
                player_id,
                account_id,
                entity_id,
                path,
                version,
                reason,
                "Ignore list reload dropped: the session plays another character, \
                 or a newer resync already applied",
            );
            return None;
        }
    };

    tracing::debug!(
        target: "chat",
        event = "chat.ignore_synced",
        %addr,
        player_id,
        account_id,
        entity_id,
        path,
        version,
        before,
        after = names.len(),
        "Ignore list cached on the base session and pushed to the cell",
    );

    if let Some(tx) = ctx.cell_tx {
        if let Err(e) = tx
            .send(BaseToCellMsg::UpdateIgnoreList {
                entity_id,
                player_id,
                account_id: session_account_id,
                version,
                ignore_names: names.clone(),
            })
            .await
        {
            tracing::warn!(
                target: "chat",
                event = "chat.ignore_sync_failed",
                %addr,
                player_id,
                account_id,
                entity_id,
                path,
                reason = "cell_send_failed",
                error = %e,
                "UpdateIgnoreList base->cell send failed; spatial chat keeps the old set",
            );
        }
    }
    Some(names)
}
