//! The gate-travel half of the GM-only world rule (Debug Area D-DA4):
//! `cimmeria_base_session::base::world_entry::gm_only_worlds` holds the
//! policy, this asks it for one cross-world transfer.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_base_session::base::world_entry::gm_only_worlds::{
    gm_only_redirect, is_gm_only_world, note_gm_only_redirect, GmOnlyDecision,
};
use sqlx::PgPool;

use super::super::super::ConnectedClientState;

/// The GM-only rule for the session at `addr` entering `target_world`: a
/// GM-only world and an account below GameMaster is refused and sent to the
/// character's start profile home. The alignment is the session's cached
/// one, else `sgw_player.alignment`, else Praxis; the archetype is
/// `sgw_player.archetype`. A redirect is logged and queued for a chat line
/// on arrival; [`GmOnlyDecision::NoHome`] is the caller's to refuse.
pub(super) async fn gm_only_gate_redirect(
    addr: SocketAddr,
    target_world: &str,
    player_id: i32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
) -> GmOnlyDecision {
    if !is_gm_only_world(target_world) {
        return GmOnlyDecision::Allowed;
    }
    let (access_level, account_id, account_name, player_name, cached_alignment) = {
        let clients = match connected.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        // No session: nothing to redirect, and the transfer fails on its
        // own a few lines later.
        let Some(c) = clients.get(&addr) else {
            return GmOnlyDecision::Allowed;
        };
        (
            c.access_level,
            c.account_id,
            c.account_name.clone(),
            c.player_name.clone(),
            c.player_alignment,
        )
    };
    // The archetype picks the start profile (a Free Jaffa's home is
    // Dakara_E1, not SGC_W1); the session caches only the alignment.
    let row: Option<(i32, i32)> = match db_pool {
        Some(pool) => {
            sqlx::query_as("SELECT alignment, archetype FROM sgw_player WHERE player_id = $1")
                .bind(player_id)
                .fetch_optional(pool.as_ref())
                .await
                .ok()
                .flatten()
        }
        None => None,
    };
    let alignment = cached_alignment.or(row.map(|r| r.0)).unwrap_or(1);
    let archetype = row.map(|r| r.1).unwrap_or(0);
    let decision = gm_only_redirect(target_world, access_level, alignment, archetype);
    if let GmOnlyDecision::Redirect(redirect) = &decision {
        note_gm_only_redirect(
            "gate_travel",
            player_id,
            player_name.as_deref(),
            Some(account_id),
            account_name.as_deref(),
            access_level,
            redirect,
        );
    }
    decision
}
