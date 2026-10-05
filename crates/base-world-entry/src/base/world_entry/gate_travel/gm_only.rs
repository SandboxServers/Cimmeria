//! The gate-travel half of the GM-only world rule (Debug Area D-DA4):
//! `cimmeria_base_session::base::world_entry::gm_only_worlds` holds the
//! policy, this asks it for one cross-world transfer.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_base_session::base::world_entry::gm_only_worlds::{
    gm_only_redirect, is_gm_only_world, note_gm_only_redirect, GmOnlyRedirect,
};
use sqlx::PgPool;

use super::super::super::ConnectedClientState;

/// `Some(redirect)` when the session at `addr` may not enter `target_world`:
/// a GM-only world and an account below GameMaster. The faction is the
/// session's cached alignment, else `sgw_player.alignment`, else Praxis.
/// The refusal is logged and queued for a chat line on arrival.
pub(super) async fn gm_only_gate_redirect(
    addr: SocketAddr,
    target_world: &str,
    player_id: i32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Option<GmOnlyRedirect> {
    if !is_gm_only_world(target_world) {
        return None;
    }
    let (access_level, account_id, account_name, player_name, cached_alignment) = {
        let clients = match connected.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        // No session: nothing to redirect, and the transfer fails on its
        // own a few lines later.
        let c = clients.get(&addr)?;
        (
            c.access_level,
            c.account_id,
            c.account_name.clone(),
            c.player_name.clone(),
            c.player_alignment,
        )
    };
    let alignment = match (cached_alignment, db_pool) {
        (Some(a), _) => a,
        (None, Some(pool)) => {
            sqlx::query_scalar::<_, i32>("SELECT alignment FROM sgw_player WHERE player_id = $1")
                .bind(player_id)
                .fetch_optional(pool.as_ref())
                .await
                .ok()
                .flatten()
                .unwrap_or(1)
        }
        (None, None) => 1,
    };
    let redirect = gm_only_redirect(target_world, access_level, alignment)?;
    note_gm_only_redirect(
        "gate_travel",
        player_id,
        player_name.as_deref(),
        Some(account_id),
        account_name.as_deref(),
        access_level,
        &redirect,
    );
    Some(redirect)
}
