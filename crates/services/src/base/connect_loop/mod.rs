//! Main `BaseService` UDP receive loop and per-datagram dispatch.
//!
//! Layout:
//! - [`run_connect_loop`] — the recv-loop driver
//! - `handle_datagram` — channel-known vs. unauthenticated fork
//! - [`encrypted::handle_encrypted_datagram`] — bundle scanner + msg dispatch
//! - [`account_arms`] / [`cell_arms`] — per-msg-family handler groups

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::{FLAG_HAS_REQUESTS, FLAG_HAS_SEQUENCE};
use cimmeria_mercury::transport::{BidirectionalTransport, Transport};

use crate::cell::messages::BaseToCellMsg;

use super::helpers::to_hex;
use super::login::{handle_login, parse_baseapp_login};
use super::resources::ResourceCache;
use super::ConnectedClientState;

mod account_arms;
mod cell_arms;
mod encrypted;

pub(crate) use encrypted::handle_encrypted_datagram;

/// Main receive loop -- one per running `BaseService`.
///
/// Takes a [`BidirectionalTransport`] for the recv side and exposes its
/// `Transport` super-trait projection to handlers via `&Arc<dyn Transport>`.
/// Production wires in [`cimmeria_mercury::transport::UdpTransport`];
/// chaos integration tests wire in
/// [`cimmeria_mercury::lossy_transport::LossyTransport`] to exercise the
/// real recv loop under simulated transatlantic loss / latency / duplication.
/// See `docs/architecture/transport-trait.md` and
/// `docs/architecture/network-chaos-testing.md`.
pub(crate) async fn run_connect_loop(
    transport: Arc<dyn BidirectionalTransport>,
    pending_logins: Arc<Mutex<HashMap<String, crate::auth::PendingLogin>>>,
    db_pool: Option<Arc<PgPool>>,
    resource_cache: Option<Arc<ResourceCache>>,
    cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_manager: Arc<Mutex<EntityManager>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let mut buf = [0u8; 4096];

    // The `BidirectionalTransport` we own already implements the
    // send-only `Transport` trait via supertrait coercion. Handlers
    // take `&Arc<dyn Transport>` so the existing ADR is unchanged.
    let send_transport: Arc<dyn Transport> = transport.clone();

    loop {
        match transport.recv_from(&mut buf).await {
            Ok((len, addr)) => {
                tracing::trace!(%addr, len, hex = %to_hex(&buf[..len]), "UDP_IN");
                if let Err(e) = handle_datagram(
                    &send_transport,
                    addr,
                    &buf[..len],
                    &pending_logins,
                    &connected,
                    &db_pool,
                    &resource_cache,
                    &entity_manager,
                    &cell_tx,
                    &entity_to_addr,
                )
                .await
                {
                    tracing::warn!(%addr, "Datagram handler error: {e}");
                }
            }
            Err(e) => {
                // On Windows, WSAECONNRESET (10054) arrives on the UDP socket
                // when a previous send_to targeted a closed port (ICMP port
                // unreachable).  This is per-destination, NOT a socket failure.
                if e.raw_os_error() == Some(10054) {
                    tracing::debug!("UDP recv: WSAECONNRESET (10054) -- remote closed, continuing");
                    continue;
                }
                tracing::error!("Base service UDP recv error (fatal): {e}");
                break;
            }
        }
    }
}

/// Dispatch a single incoming UDP datagram.
async fn handle_datagram(
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
    raw: &[u8],
    pending_logins: &Arc<Mutex<HashMap<String, crate::auth::PendingLogin>>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
    resource_cache: &Option<Arc<ResourceCache>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if raw.is_empty() {
        return Ok(());
    }

    // Check for an established encrypted channel first.
    let channel_key: Option<(
        MercuryEncryption,
        [u8; 32],
        u32,
        Arc<Mutex<Vec<u32>>>,
        Arc<Mutex<Instant>>,
    )> = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        clients.get(&addr).map(|c| {
            (
                c.enc.clone(),
                c.key,
                c.account_id,
                Arc::clone(&c.pending_acks),
                Arc::clone(&c.last_recv),
            )
        })
    };

    if let Some((enc, key, account_id, pending_acks, last_recv)) = channel_key {
        // Update last-recv timestamp on every packet from this client.
        *last_recv.lock().unwrap() = Instant::now();
        return handle_encrypted_datagram(
            transport,
            addr,
            raw,
            enc,
            key,
            account_id,
            &pending_acks,
            connected,
            db_pool,
            resource_cache,
            entity_manager,
            cell_tx,
            entity_to_addr,
        )
        .await;
    }

    // Not yet connected -- only accept the unencrypted Phase 3 login (flags=0x41).
    let login_flags = FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE; // 0x41
    if raw[0] != login_flags {
        tracing::trace!(%addr, flags = raw[0], "Ignoring packet from unknown addr (flags={:#04x})", raw[0]);
        return Ok(());
    }

    match parse_baseapp_login(raw) {
        Ok((request_id, ticket_str)) => {
            tracing::info!(%addr, ticket = %ticket_str, "baseAppLogin received");
            handle_login(
                transport,
                addr,
                request_id,
                &ticket_str,
                pending_logins,
                connected,
                entity_manager,
                cell_tx,
                entity_to_addr,
            )
            .await
        }
        Err(e) => {
            tracing::warn!(%addr, "Failed to parse baseAppLogin: {e}");
            Ok(())
        }
    }
}

/// Read a CONSTANT_LENGTH payload (no length prefix, fixed size).
pub(super) fn read_constant_payload<'a>(
    body: &'a [u8],
    offset: &mut usize,
    size: usize,
) -> Option<&'a [u8]> {
    if *offset + size > body.len() {
        return None;
    }
    let payload = &body[*offset..*offset + size];
    *offset += size;
    Some(payload)
}

/// Read a WORD_LENGTH payload (u16 length prefix).
pub(super) fn read_word_length_payload<'a>(body: &'a [u8], offset: &mut usize) -> Option<&'a [u8]> {
    if *offset + 2 > body.len() {
        return None;
    }
    let word_len = u16::from_le_bytes([body[*offset], body[*offset + 1]]) as usize;
    *offset += 2;
    if *offset + word_len > body.len() {
        return None;
    }
    let payload = &body[*offset..*offset + word_len];
    *offset += word_len;
    Some(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_constant_length_0x02() {
        // AVATAR_UPD_IMPLICIT: CONSTANT_LENGTH = 36
        let buf = vec![0xAAu8; 36];
        let mut offset = 0usize;
        let payload = read_constant_payload(&buf, &mut offset, 36);
        assert!(payload.is_some());
        assert_eq!(offset, 36);
        assert_eq!(payload.unwrap().len(), 36);
    }

    #[test]
    fn scanner_constant_length_0x04() {
        // AVATAR_UPDW_IMPLICIT: CONSTANT_LENGTH = 36
        let buf = vec![0xBBu8; 36];
        let mut offset = 0usize;
        let payload = read_constant_payload(&buf, &mut offset, 36);
        assert!(payload.is_some());
        assert_eq!(offset, 36);
        assert_eq!(payload.unwrap().len(), 36);
    }

    #[test]
    fn scanner_constant_length_0x05() {
        // AVATAR_UPDW_EXPLICIT: CONSTANT_LENGTH = 40
        let buf = vec![0xCCu8; 40];
        let mut offset = 0usize;
        let payload = read_constant_payload(&buf, &mut offset, 40);
        assert!(payload.is_some());
        assert_eq!(offset, 40);
        assert_eq!(payload.unwrap().len(), 40);
    }
}
