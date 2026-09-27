//! "This character went offline", told once per session end: to the
//! contact-list watchers (CM 89) and to the online members of each Team and
//! Command the character belongs to ([37] with id 0, ORG-06).
//!
//! Every teardown that removes a session goes through
//! `helpers::destroy_client_entities` (client disconnect, inactivity
//! timeout, send error, duplicate login, account log-off), which calls
//! [`spawn_offline`] for a session whose character was in the world. Before
//! ORG-06 only `logOff` told anyone (audit A-35), so a crash or a timeout
//! left the character "online" in every friend list and roster.
//!
//! **Exactly once.** Only a session still listed online announces
//! (`ConnectedClientState::listed_online`). `logOff` clears the flag and
//! announces itself, so the disconnect that reaps a full exit afterwards
//! does not announce twice.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::organization::handlers::{announce_offline, OrgCtx, OrgPlayer};
use super::ConnectedClientState;

/// The character whose session ended, snapshotted before the session goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndedSession {
    pub account_id: u32,
    pub player_id: i32,
    pub entity_id: u32,
    pub player_name: Option<String>,
}

/// Tell the contact-list watchers and the organizations, now. The caller
/// has already removed or unlisted the session.
pub async fn announce_session_end(
    ended: &EndedSession,
    disconnect_reason: &'static str,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    if let Some(name) = &ended.player_name {
        super::contact_list::handlers::fanout_login_status(
            name,
            false,
            db_pool,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }
    let ctx = OrgCtx {
        db_pool,
        transport,
        connected,
        entity_to_addr,
        // Going offline ends no membership.
        cell_tx: &None,
    };
    let player = OrgPlayer {
        account_id: Some(ended.account_id),
        player_id: ended.player_id,
        entity_id: ended.entity_id,
    };
    announce_offline(&ctx, &player, disconnect_reason).await;
}

/// [`announce_session_end`] on its own task, so the teardown never waits
/// on the database. Without a database there is nobody to look up, and
/// without a runtime (a synchronous unit test) nothing can run; both log
/// DEBUG `session.presence_skipped` and send nothing.
pub fn spawn_offline(
    ended: EndedSession,
    disconnect_reason: &'static str,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let skipped = |reason: &'static str| {
        tracing::debug!(
            target: "org",
            event = "session.presence_skipped",
            account_id = ended.account_id,
            player_id = ended.player_id,
            entity_id = ended.entity_id,
            disconnect_reason,
            reason,
            "offline presence not announced"
        );
    };
    if db_pool.is_none() {
        skipped("no_db");
        return;
    }
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        skipped("no_runtime");
        return;
    };
    let (db_pool, transport, connected, entity_to_addr) = (
        db_pool.clone(),
        transport.clone(),
        connected.clone(),
        entity_to_addr.clone(),
    );
    runtime.spawn(async move {
        announce_session_end(
            &ended,
            disconnect_reason,
            &db_pool,
            &transport,
            &connected,
            &entity_to_addr,
        )
        .await;
    });
}
