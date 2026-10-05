//! Evicting the sessions a fresh Phase 3 login displaces.
//!
//! One account has at most one session. A login that authenticates for an
//! account evicts every other session of that account before its own
//! channel is registered:
//!
//! - **Duplicate login** (KI-7): a session of the same account on another
//!   address. That client is still running, so it gets a `LOGGED_OFF`
//!   for an immediate teardown on its side, then the session is torn down
//!   with `disconnect_reason = "duplicate_login"`.
//! - **Relaunch takeover**: a session of the same account on the **same**
//!   address:port. The client that owned it is dead (it was killed and
//!   relaunched, and the SGW client binds a fixed UDP port), and the
//!   address now belongs to the new client. No `LOGGED_OFF`: it would be
//!   encrypted with the old key and reach the new client as one more
//!   corrupted packet. The session is torn down with
//!   `disconnect_reason = "relaunch_takeover"`. The gate that lets a login
//!   reach this point on an occupied address is in `relaunch.rs`.
//!
//! Both go through `destroy_client_entities`: the old character is
//! unlisted and announced offline, its cell entity is told to disconnect
//! (the cell persists its position before confirming), the old tick-sync
//! loop is cancelled, and the session leaves the `connected` map. The
//! caller then inserts the new session, with the new key and a fresh
//! channel, in one map write, so nothing encrypts toward the address with
//! the old key after that write. The old tick-sync loop can send at most
//! one more tick on its old key before it sees its cancel flag (one
//! "Dropped corrupted incoming packet" on the client, harmless), and its
//! own teardown is owner-checked (`destroy_owned_client_entities`) so it
//! cannot remove the new session.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::encryption::EncryptionVersion;
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_entity::manager::EntityManager;

use crate::auth::PendingLogin;
use crate::cell::messages::BaseToCellMsg;
use crate::mercury::build_logged_off;

use super::super::helpers::destroy_client_entities;
use super::super::session_identity;
use super::super::ConnectedClientState;

/// Stable `disconnect_reason` (and log `reason`) for a session replaced by
/// a relaunched client on the same address:port.
pub(crate) const REASON_RELAUNCH_TAKEOVER: &str = "relaunch_takeover";

/// One session to evict, snapshotted under the `connected` lock.
struct Displaced {
    addr: SocketAddr,
    key: [u8; 32],
    enc_version: EncryptionVersion,
    identity: cimmeria_entity::cell_entity::PlayerIdentity,
    session_secs: u64,
}

/// Evict every session of `login`'s account, wherever it is, ahead of the
/// new session at `addr`. See the module doc for the two cases.
#[allow(clippy::too_many_arguments)]
pub(super) async fn evict_prior_sessions(
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
    login: &PendingLogin,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let displaced: Vec<Displaced> = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        clients
            .iter()
            .filter(|(_, c)| c.account_id == login.account_id)
            .map(|(old_addr, c)| Displaced {
                addr: *old_addr,
                key: c.key,
                enc_version: c.enc_version,
                identity: session_identity::session_identity(c),
                session_secs: c.connected_at.elapsed().as_secs(),
            })
            .collect()
    };

    for old in displaced {
        let reason = if old.addr == addr {
            tracing::warn!(
                %addr,
                account_id = login.account_id,
                account_name = %login.account_name,
                player_id = old.identity.player_id,
                player_name = old.identity.player_name,
                old_session_secs = old.session_secs,
                reason = REASON_RELAUNCH_TAKEOVER,
                "Client relaunched on its live session's address -- evicting the old session \
                 and accepting the new login"
            );
            REASON_RELAUNCH_TAKEOVER
        } else {
            tracing::warn!(
                account_id = login.account_id,
                account_name = %login.account_name,
                player_id = old.identity.player_id,
                player_name = old.identity.player_name,
                old_addr = %old.addr,
                %addr,
                "Duplicate login -- evicting old session"
            );
            send_logged_off(transport, connected, &old).await?;
            "duplicate_login"
        };
        destroy_client_entities(
            connected,
            entity_manager,
            old.addr,
            cell_tx,
            entity_to_addr,
            transport,
            db_pool,
            reason,
        );
    }
    Ok(())
}

/// Send `LOGGED_OFF` to a still-running client on another address so it
/// tears its side down at once instead of waiting for its own timeout.
async fn send_logged_off(
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    old: &Displaced,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (acks, seq) = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        match clients.get_mut(&old.addr) {
            Some(c) => {
                let acks: Vec<u32> = cimmeria_mercury::packet::take_piggyback_acks(
                    &mut c.pending_acks.lock().unwrap(),
                    c.enc_version,
                );
                let seq = c
                    .next_seq
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    & cimmeria_mercury::packet::SEQUENCE_MASK;
                (acks, seq)
            }
            None => (vec![], 0),
        }
    };
    let pkt = build_logged_off(&old.key, seq, &acks, old.enc_version);
    let _ = transport.send_to(&pkt, old.addr).await;
    Ok(())
}
