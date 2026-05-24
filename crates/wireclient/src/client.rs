//! Top-level wireclient driver.
//!
//! Today (Phase 1) this stitches together SOAP auth → Mercury phase-3
//! handshake → encryption-established state. Future phases bolt on:
//!
//! - An entity mirror (`onPropertyUpdate`, `onStateFieldUpdate` apply locally).
//! - Stats mirror (health / ammo / active bandolier slot — used for
//!   client-side invariant enforcement on `useAbility`).
//! - A dialog state machine (which screen is open, what choices the player
//!   can send).
//! - The Castle Cellblock script + behavior-trace comparator (see
//!   `docs/architecture/wireclient.md`).

use std::net::SocketAddr;

use cimmeria_mercury::encryption::MercuryEncryption;

use crate::auth::{AuthClient, AuthSession, Credentials};
use crate::error::Result;
use crate::handshake;

/// Top-level client state once the handshake is complete.
///
/// The struct is intentionally thin in Phase 1 — fields show the shape
/// follow-on phases will populate.
pub struct Client {
    /// Address of the BaseApp UDP endpoint (`SessionKey` issuer).
    pub base_addr: SocketAddr,
    /// Encryption context, ready for post-handshake reliable I/O.
    pub encryption: MercuryEncryption,
    /// Account ID — required to send `baseAppLogin`. Today this is supplied
    /// by the caller because the auth Phase 2 response does not yet
    /// surface it on `AuthSession`; surfacing it is a small follow-up.
    pub account_id: u32,
    /// Caller-side request ID picked at login; echoes through the reply
    /// for desync detection.
    pub request_id: u32,
}

impl Client {
    /// Run SOAP auth Phase 1+2 and return the [`AuthSession`] without
    /// dialing the BaseApp. Useful for tests that drive only the HTTP leg.
    pub async fn login_only(
        auth_url: impl Into<String>,
        creds: &Credentials,
        shard: &str,
    ) -> Result<AuthSession> {
        let client = AuthClient::new(auth_url);
        client.login(creds, shard).await
    }

    /// Construct the unencrypted `baseAppLogin` UDP datagram from a
    /// completed Phase 2 session.
    ///
    /// The caller is responsible for the actual UDP `send_to` — Phase 1 of
    /// the wireclient deliberately stops at "produce the bytes" so that
    /// tests can byte-diff against the recorded capture before any UDP
    /// socket is involved. Phase 1.5 wires the socket loop.
    pub fn build_login_packet(
        session: &AuthSession,
        account_id: u32,
        request_id: u32,
    ) -> Result<Vec<u8>> {
        handshake::build_baseapp_login(request_id, account_id, &session.ticket)
    }

    /// Construct a `Client` from a completed handshake. Intended for tests
    /// that drive the bytes themselves; the production path is
    /// `connect()` (not yet implemented — Phase 1.5).
    pub fn from_handshake(session: AuthSession, account_id: u32, request_id: u32) -> Self {
        let encryption = session.encryption();
        Self {
            base_addr: session.base_addr,
            encryption,
            account_id,
            request_id,
        }
    }
}
