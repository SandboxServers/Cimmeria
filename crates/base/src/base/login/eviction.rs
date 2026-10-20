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
//! - **Address reclaim**: another account's session on the same address
//!   that registered with a ticket issued to a different IP, displaced by
//!   a login whose ticket was issued to this address's IP (a squatter on a
//!   spoofed address; `relaunch.rs` rule 4). No `LOGGED_OFF`, for the same
//!   reason; `disconnect_reason = "address_reclaimed"`.
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
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::encryption::EncryptionVersion;
use cimmeria_mercury::transport::{SendOrigin, Transport};
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

/// Send a teardown packet with its retained session ticket when one exists.
pub(super) async fn send_direct(
    transport: &Arc<dyn Transport>,
    packet: &[u8],
    addr: SocketAddr,
    origin: Option<&SendOrigin>,
) -> std::io::Result<usize> {
    match origin {
        Some(origin) => transport.send_to_with_origin(packet, addr, origin).await,
        None => transport.send_to(packet, addr).await,
    }
}

/// Idle time from which a displaced session counts as dead: a client that
/// crashed and relaunched goes silent well before this, a live one never does.
const STALE_IDLE_MS: u64 = 15_000;

/// One session to evict, snapshotted under the `connected` lock.
struct Displaced {
    addr: SocketAddr,
    key: [u8; 32],
    enc_version: EncryptionVersion,
    next_seq: Arc<AtomicU32>,
    origin: Option<SendOrigin>,
    identity: cimmeria_entity::cell_entity::PlayerIdentity,
    session_secs: u64,
    /// Milliseconds since the old session last received a packet.
    idle_ms: u64,
    /// Whether the old session had reached world entry.
    in_world: bool,
}

/// Evict every session of `login`'s account, wherever it is, ahead of the
/// new session at `addr`. See the module doc for the two cases. With
/// `evict_squatter`, the session on `addr` goes too whatever its account:
/// `relaunch::address_claim` found it registered with a ticket issued to
/// another IP while this login's ticket was issued to this address's IP.
#[allow(clippy::too_many_arguments)]
pub(super) async fn evict_prior_sessions(
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
    login: &PendingLogin,
    evict_squatter: bool,
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
            .filter(|(a, c)| c.account_id == login.account_id || (evict_squatter && **a == addr))
            .map(|(old_addr, c)| Displaced {
                addr: *old_addr,
                key: c.key,
                enc_version: c.enc_version,
                next_seq: Arc::clone(&c.next_seq),
                origin: c.wire_send_origin(),
                identity: session_identity::session_identity(c),
                session_secs: c.connected_at.elapsed().as_secs(),
                idle_ms: c.last_recv.lock().unwrap().elapsed().as_millis() as u64,
                in_world: c.player_entity_id.is_some(),
            })
            .collect()
    };

    for old in displaced {
        let reason = if old.addr == addr && old.identity.account_id != Some(login.account_id) {
            // The squatter: `relaunch::address_claim` already wrote the WARN.
            // No LOGGED_OFF: it would go to this address, which is the new
            // client's.
            super::relaunch::REASON_ADDRESS_RECLAIMED
        } else if old.addr == addr {
            // INFO for a stale session (a crash-and-relaunch), WARN for one
            // still receiving packets (a real concurrent login).
            if old.idle_ms >= STALE_IDLE_MS {
                tracing::info!(
                    %addr,
                    account_id = login.account_id,
                    account_name = %login.account_name,
                    player_id = old.identity.player_id,
                    player_name = old.identity.player_name,
                    old_session_secs = old.session_secs,
                    old_idle_ms = old.idle_ms,
                    old_in_world = old.in_world,
                    reason = REASON_RELAUNCH_TAKEOVER,
                    "Client relaunched on its live session's address -- evicting the old session \
                     and accepting the new login"
                );
            } else {
                tracing::warn!(
                    %addr,
                    account_id = login.account_id,
                    account_name = %login.account_name,
                    player_id = old.identity.player_id,
                    player_name = old.identity.player_name,
                    old_session_secs = old.session_secs,
                    old_idle_ms = old.idle_ms,
                    old_in_world = old.in_world,
                    reason = REASON_RELAUNCH_TAKEOVER,
                    "Client relaunched on its live session's address -- evicting the old session \
                     and accepting the new login"
                );
            }
            REASON_RELAUNCH_TAKEOVER
        } else {
            if old.idle_ms >= STALE_IDLE_MS {
                tracing::info!(
                    account_id = login.account_id,
                    account_name = %login.account_name,
                    player_id = old.identity.player_id,
                    player_name = old.identity.player_name,
                    old_addr = %old.addr,
                    %addr,
                    old_session_secs = old.session_secs,
                    old_idle_ms = old.idle_ms,
                    old_in_world = old.in_world,
                    reason = "duplicate_login",
                    "Duplicate login -- evicting old session"
                );
            } else {
                tracing::warn!(
                    account_id = login.account_id,
                    account_name = %login.account_name,
                    player_id = old.identity.player_id,
                    player_name = old.identity.player_name,
                    old_addr = %old.addr,
                    %addr,
                    old_session_secs = old.session_secs,
                    old_idle_ms = old.idle_ms,
                    old_in_world = old.in_world,
                    reason = "duplicate_login",
                    "Duplicate login -- evicting old session"
                );
            }
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
    let (acks, seq, origin) = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        match clients.get_mut(&old.addr) {
            Some(c) if Arc::ptr_eq(&c.next_seq, &old.next_seq) && c.key == old.key => {
                let acks: Vec<u32> = cimmeria_mercury::packet::take_piggyback_acks(
                    &mut c.pending_acks.lock().unwrap(),
                    c.enc_version,
                );
                let _seq_gate = c
                    .reliable_seq_gate
                    .lock()
                    .map_err(|_| "sequence gate poisoned")?;
                let seq = c
                    .next_seq
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    & cimmeria_mercury::packet::SEQUENCE_MASK;
                (acks, seq, old.origin.clone())
            }
            _ => {
                tracing::debug!(
                    old_addr = %old.addr,
                    reason = "displaced_owner_replaced",
                    "Skipping LOGGED_OFF for a replaced displaced session"
                );
                return Ok(());
            }
        }
    };
    let pkt = build_logged_off(&old.key, seq, &acks, old.enc_version);
    let result = send_direct(transport, &pkt, old.addr, origin.as_ref()).await;
    if let Err(e) = result {
        tracing::debug!(
            old_addr = %old.addr,
            reason = "send_failed",
            error = %e,
            "LOGGED_OFF to the displaced session failed"
        );
    }
    Ok(())
}

/// Handle `logOff` (0xC2) -- the client is leaving the session.
///
/// Both "Change Server" and "Back" buttons in the character select UI call this.
/// The C++ reference (`Account.py:logOff`) was a no-op (`pass`), which meant the
/// server never cleaned up -- leaking sessions indefinitely.
///
/// Client flow (from Ghidra analysis of FUN_00def9f0):
///   1. Both buttons emit `Event_NetOut_LogOff` -> sends logOff (0xC2)
///   2. `relogin()` also sets a relogin flag before step 1
///   3. Client fires `gotoLogin` state transition
///   4. Client waits for Mercury channel to die (server closes or timeout)
///   5. `Event_Net_Disconnected` fires -> disconnect callback (FUN_00de9bb0):
///      - If relogin flag set -> `EntityManager::disconnected()` + SOAP re-login
///      - If not set -> shows "disconnected from network" UI
///
/// We send `LOGGED_OFF` (0x37) to trigger an immediate channel teardown on the
/// client side (via `ServerConnection::loggedOff -> Mercury::disconnect`).
/// Without it, the client waits 30s for the Mercury channel timeout.
pub(crate) async fn handle_log_off(
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
    key: [u8; 32],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // The account pair is read before the session is removed below.
    let identity = session_identity::identity_for_addr(connected, addr);
    tracing::info!(
        %addr,
        account_id = identity.account_id,
        account_name = identity.account_name,
        "Client requests logOff -- sending LOGGED_OFF and cleaning up"
    );

    // Signal the tick-sync loop to stop and grab seq/acks for the final packet.
    // Snapshot the identity + session duration here too, before
    // `destroy_client_entities` removes the session from the map — the
    // Discord logout emit below needs them.
    let (acks, seq, enc_version, account, character, session_secs) = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let client = clients.get_mut(&addr).ok_or("no session for addr")?;
        client
            .cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let acks: Vec<u32> = cimmeria_mercury::packet::take_piggyback_acks(
            &mut client.pending_acks.lock().unwrap(),
            client.enc_version,
        );
        let _seq_gate = client
            .reliable_seq_gate
            .lock()
            .map_err(|_| "sequence gate poisoned")?;
        let seq = client
            .next_seq
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            & cimmeria_mercury::packet::SEQUENCE_MASK;
        let enc_version = client.enc_version;
        let account = client.discord_account();
        let character = client.discord_character();
        let session_secs = client.connected_at.elapsed().as_secs();
        (acks, seq, enc_version, account, character, session_secs)
    };

    // Send LOGGED_OFF (0x37) with reason=0 to trigger immediate client-side
    // channel teardown.  This fires ServerConnection::loggedOff -> Event_Net_Disconnected
    // which triggers the relogin flow (Change Server) or disconnect UI (Back).
    let pkt = build_logged_off(&key, seq, &acks, enc_version);
    send_direct(transport, &pkt, addr, None).await?;
    tracing::debug!(%addr, seq, "Sent LOGGED_OFF (0x37)");

    // Destroy entities and remove from connected map.
    destroy_client_entities(
        connected,
        entity_manager,
        addr,
        cell_tx,
        entity_to_addr,
        transport,
        // Account-side logOff is from character select: no character is in
        // the world, so there is no offline presence to look up.
        &None,
        "logoff",
    );

    // Discord auth-channel: clean logout carries the session length. The
    // lower-level teardown in `destroy_client_entities` also fires a
    // `PlayerDisconnect { reason: Clean }` — by design (see the
    // `DisconnectReason::Clean` doc): logout is the gameplay-level event,
    // disconnect the connection-level one.
    cimmeria_discord::emit_player_logout(account, character, session_secs);

    Ok(())
}

#[cfg(test)]
mod direct_send_origin_tests {
    use super::*;
    use async_trait::async_trait;
    use cimmeria_base_session::base::SessionOriginSeed;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Notify;

    struct Wire {
        entered: Notify,
        release: Notify,
        fail: bool,
        sends: AtomicUsize,
        rows: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl Transport for Wire {
        async fn send_to(&self, bytes: &[u8], _addr: SocketAddr) -> std::io::Result<usize> {
            self.sends.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                Err(std::io::Error::other("synthetic failure"))
            } else {
                Ok(bytes.len())
            }
        }

        async fn send_to_with_origin(
            &self,
            bytes: &[u8],
            _addr: SocketAddr,
            origin: &SendOrigin,
        ) -> std::io::Result<usize> {
            self.sends.fetch_add(1, Ordering::SeqCst);
            self.entered.notify_one();
            self.release.notified().await;
            if self.fail {
                return Err(std::io::Error::other("synthetic failure"));
            }
            self.rows
                .lock()
                .unwrap()
                .push(origin.session_generation.to_string());
            Ok(bytes.len())
        }

        fn local_addr(&self) -> std::io::Result<SocketAddr> {
            Ok("127.0.0.1:0".parse().unwrap())
        }
    }

    fn session(addr: SocketAddr, generation: &str) -> ConnectedClientState {
        let mut state = crate::test_support::test_default_connected_client_state();
        state.extensions.insert(SessionOriginSeed {
            server_boot_id: Arc::from("boot"),
            session_generation: Arc::from(generation),
            peer: addr,
            connected_at: state.connected_at,
        });
        state
    }

    async fn run_replacement(fail: bool) -> Vec<String> {
        let addr: SocketAddr = "127.0.0.1:55632".parse().unwrap();
        let a = session(addr, "A");
        let old = Displaced {
            addr,
            key: a.key,
            enc_version: a.enc_version,
            next_seq: Arc::clone(&a.next_seq),
            origin: a.wire_send_origin(),
            identity: session_identity::session_identity(&a),
            session_secs: 0,
            idle_ms: 0,
            in_world: false,
        };
        let connected = Arc::new(Mutex::new(HashMap::from([(addr, a)])));
        let wire = Arc::new(Wire {
            entered: Notify::new(),
            release: Notify::new(),
            fail,
            sends: AtomicUsize::new(0),
            rows: Mutex::new(Vec::new()),
        });
        let transport: Arc<dyn Transport> = wire.clone();
        let clients = Arc::clone(&connected);
        let task = tokio::spawn(async move { send_logged_off(&transport, &clients, &old).await });
        wire.entered.notified().await;
        connected.lock().unwrap().insert(addr, session(addr, "B"));
        wire.release.notify_one();
        task.await.unwrap().unwrap();
        assert!(connected.lock().unwrap()[&addr]
            .channel
            .lock()
            .unwrap()
            .tx_window
            .is_empty());
        let rows = wire.rows.lock().unwrap().clone();
        rows
    }

    #[tokio::test]
    async fn direct_send_origin_displaced_logoff_keeps_a_after_same_peer_relog() {
        assert_eq!(run_replacement(false).await, ["A"]);
    }

    #[tokio::test]
    async fn direct_send_origin_failed_displaced_logoff_has_no_proof_row() {
        assert!(run_replacement(true).await.is_empty());
    }

    #[tokio::test]
    async fn direct_send_origin_replaced_before_reservation_sends_no_logout() {
        let addr: SocketAddr = "127.0.0.1:55632".parse().unwrap();
        let a = session(addr, "A");
        let old = Displaced {
            addr,
            key: a.key,
            enc_version: a.enc_version,
            next_seq: Arc::clone(&a.next_seq),
            origin: a.wire_send_origin(),
            identity: session_identity::session_identity(&a),
            session_secs: 0,
            idle_ms: 0,
            in_world: false,
        };
        let b = session(addr, "B");
        assert_eq!(a.key, b.key, "fixture must exercise same-key replacement");
        b.pending_acks.lock().unwrap().extend([10, 11]);
        let connected = Arc::new(Mutex::new(HashMap::from([(addr, b)])));
        let wire = Arc::new(Wire {
            entered: Notify::new(),
            release: Notify::new(),
            fail: false,
            sends: AtomicUsize::new(0),
            rows: Mutex::new(Vec::new()),
        });
        let transport: Arc<dyn Transport> = wire.clone();
        send_logged_off(&transport, &connected, &old).await.unwrap();
        assert_eq!(wire.sends.load(Ordering::SeqCst), 0);
        assert!(wire.rows.lock().unwrap().is_empty());
        let clients = connected.lock().unwrap();
        let b = &clients[&addr];
        assert_eq!(b.next_seq.load(Ordering::Relaxed), 0);
        assert_eq!(&*b.pending_acks.lock().unwrap(), &[10, 11]);
    }
}
