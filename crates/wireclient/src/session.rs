//! Real end-to-end game session: SOAP auth → Mercury phase-3 handshake →
//! a live Channel driven over a real UDP socket against the actual
//! `BaseService`.
//!
//! This is wireclient's Phase 1.5 (the "UDP send/recv loop" the ADR
//! flagged as pending) plus the slice of Phase 2/4 a two-client
//! shared-world visibility test needs: character select, world entry
//! (`ENABLE_ENTITIES` / `playCharacter` / `mapLoaded` / `onClientReady`),
//! and enough client→server builders to drive a player around after
//! entry. See `docs/architecture/wireclient.md` for how this fits the
//! phase table.
//!
//! # Why this reuses `cimmeria_mercury::test_harness::LoopbackPeer`
//!
//! Everything past the handshake is ordinary Mercury Channel traffic:
//! reliable sends need TX-window registration and retransmit; inbound
//! fragmented bundles need reassembly; inbound reliable packets owe a
//! piggyback ACK. `LoopbackPeer` already implements exactly that against
//! a real `tokio::net::UdpSocket` -- it was written for the Tier 2
//! peer-to-peer loopback harness, but nothing about it assumes the peer
//! on the other end is test code. A `GameSession` is a `LoopbackPeer`
//! bound to an ephemeral local port, sending to the real BaseApp
//! address, with the session's own `MercuryEncryption` context installed.
//! No production Channel code was duplicated to build this.
//!
//! # What this does *not* do
//!
//! No client-side invariant enforcement (ammo/range/cooldown/LOS) --
//! that is wireclient's Phase 5 scope, orthogonal to the socket loop.
//! No entity mirror -- callers decode what they need via
//! [`crate::bundle::decode_bundle`] directly.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::Bytes;
use tokio::net::UdpSocket;

use cimmeria_mercury::test_harness::{Direction, LoopbackPeer, NetworkPolicy, TestClock};

use crate::auth::{AuthClient, AuthSession, Credentials};
use crate::error::{Error, Result};
use crate::handshake;

/// A connected, authenticated, world-entry-capable client session.
pub struct GameSession {
    peer: LoopbackPeer,
    /// This player's own cell/base entity id -- learned from the
    /// `CREATE_BASE_PLAYER` reply during world entry, since the server
    /// allocates it (the client has no way to predict it).
    pub player_entity_id: Option<u32>,
    /// Account id from Phase 1 -- threaded through for logging/asserts.
    pub account_id: u32,
}

impl GameSession {
    /// Run SOAP auth Phase 1+2, then the Mercury phase-3 handshake, over a
    /// real UDP socket bound to an OS-assigned ephemeral port. Returns a
    /// session ready to drive `enable_entities()` / `play_character()` /
    /// world entry.
    pub async fn connect(
        auth_url: &str,
        creds: &Credentials,
        shard: &str,
        request_id: u32,
    ) -> Result<Self> {
        let auth = AuthClient::new(auth_url);
        let session: AuthSession = auth.login(creds, shard).await?;
        Self::from_auth_session(&session, request_id).await
    }

    /// As [`Self::connect`] but takes an already-completed [`AuthSession`]
    /// -- lets a caller reuse one Phase 1+2 round trip's timing
    /// independently of the socket connect (e.g. to control the exact
    /// interleaving of two clients' world-entry sequences).
    pub async fn from_auth_session(session: &AuthSession, request_id: u32) -> Result<Self> {
        let socket = UdpSocket::bind("127.0.0.1:0").await?;

        let login_pkt =
            handshake::build_baseapp_login(request_id, session.account_id, &session.ticket)?;
        socket.send_to(&login_pkt, session.base_addr).await?;

        let enc = session.encryption();

        let mut buf = vec![0u8; cimmeria_mercury::consts::PACKET_MAX_SIZE];
        let (n1, _) = tokio::time::timeout(Duration::from_secs(5), socket.recv_from(&mut buf))
            .await
            .map_err(|_| {
                Error::MercuryIo(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "no baseAppLogin reply",
                ))
            })??;
        let _reply = handshake::parse_baseapp_reply(&buf[..n1], &enc, request_id, &session.ticket)?;

        let (n2, _) = tokio::time::timeout(Duration::from_secs(5), socket.recv_from(&mut buf))
            .await
            .map_err(|_| {
                Error::MercuryIo(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "no time-sync reply",
                ))
            })??;
        let _time_sync = handshake::parse_time_sync(&buf[..n2], &enc)?;

        let clock = Arc::new(TestClock::new());
        let policy = Arc::new(Mutex::new(NetworkPolicy::default()));
        let peer = LoopbackPeer::from_socket(
            socket,
            session.base_addr,
            clock,
            Some(Arc::new(enc)),
            Direction::AToB,
            policy,
        )
        .await?;
        // The server's reliable stream opened with the baseAppLogin reply
        // (seq 1) and the time-sync bundle (seq 2), both consumed above
        // before the Channel existed. The real client's channel saw them
        // (its `inSeqAt` adopts seq 1), so the next reliable packet it
        // expects is 3. Pin that, so the receive gate orders everything
        // after the handshake the way the client does (NA38).
        peer.channel
            .lock()
            .expect("channel poisoned")
            .anchor_rx_seq(handshake::FIRST_CHANNEL_SEQ);

        Ok(Self {
            peer,
            player_entity_id: None,
            account_id: session.account_id,
        })
    }

    /// This session's local UDP address -- useful for correlating against
    /// server-side `ConnectedClientState` logs.
    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        Ok(self.peer.addr)
    }

    /// Send a raw bundle body (already-framed client messages,
    /// back-to-back). Thin pass-through to [`LoopbackPeer::send_bundle`].
    pub async fn send_bundle(&self, body: &[u8], reliable: bool) -> std::io::Result<()> {
        self.peer.send_bundle(body, reliable).await
    }

    /// Wait for `n` reassembled bundles from the server, or until
    /// `timeout` elapses (returning however many arrived).
    pub async fn recv_bundles(&self, n: usize, timeout: Duration) -> Vec<Bytes> {
        self.peer.recv_n_bundles(n, timeout).await
    }

    // ── Client → server message builders ────────────────────────────────
    // Each returns the raw message bytes (msg_id + framing + payload) to
    // hand to `send_bundle` -- callers can concatenate several into one
    // bundle, matching how the real client batches its outbound queue.

    /// `AUTHENTICATE` (0x01). The server ignores its content
    /// (`base/connect_loop/encrypted/mod.rs`: "AUTHENTICATE received --
    /// ignored"); included for wire fidelity with a real client's first
    /// post-handshake bundle.
    pub fn authenticate() -> Vec<u8> {
        word_len_msg(0x01, &[])
    }

    /// `ENABLE_ENTITIES` (0x08, CONSTANT_LENGTH = 8). Content is unread by
    /// the server (`messages.cpp` calls it "8 dummy bytes"); this sends
    /// eight zero bytes.
    pub fn enable_entities() -> Vec<u8> {
        let mut v = vec![0x08u8];
        v.extend_from_slice(&[0u8; 8]);
        v
    }

    /// `playCharacter` (account base method `0xC4`). `player_id` is the
    /// `sgw_player.player_id` row to enter the world as.
    pub fn play_character(player_id: i32) -> Vec<u8> {
        word_len_msg(0xC4, &player_id.to_le_bytes())
    }

    /// `mapLoaded` (SGWPlayer cell method index 25, wire msg_id `0x99`).
    /// Direct cell-method encoding: `[entity_id:4]`, no further args --
    /// the server-side handler (`cell_arms::dispatch_cell_method`) does
    /// not read past the entity_id.
    pub fn map_loaded(entity_id: u32) -> Vec<u8> {
        word_len_msg(0x99, &entity_id.to_le_bytes())
    }

    /// `onClientReady` (SGWPlayer base method, wire msg_id `0xD8`). Empty
    /// payload -- the server handler takes no message-derived argument.
    pub fn on_client_ready() -> Vec<u8> {
        word_len_msg(0xD8, &[])
    }

    /// `AVATAR_UPDATE_EXPLICIT` (0x03, CONSTANT_LENGTH = 40). Wire:
    /// `[spaceId:u32][vehicleId:u32=0][pos:3xf32][vel:3xf32][dir:3xi8][flags:u8][cells:3xu8][updateId:u8]`.
    /// See `base/connect_loop/encrypted/mod.rs` msg `0x03` decode for the
    /// authoritative field layout this mirrors.
    #[allow(clippy::too_many_arguments)]
    pub fn avatar_update_explicit(
        space_id: u32,
        pos: [f32; 3],
        vel: [f32; 3],
        dir: [i8; 3],
        update_id: u8,
    ) -> Vec<u8> {
        let mut v = vec![0x03u8];
        v.extend_from_slice(&space_id.to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes()); // vehicleId
        for c in pos {
            v.extend_from_slice(&c.to_le_bytes());
        }
        for c in vel {
            v.extend_from_slice(&c.to_le_bytes());
        }
        for d in dir {
            v.push(d as u8);
        }
        v.push(0); // flags
        v.extend_from_slice(&[0u8; 3]); // cells
        v.push(update_id);
        v
    }

    /// `DISCONNECT` (0x0C, CONSTANT_LENGTH = 1). Triggers the server's
    /// `destroy_client_entities` cleanup path -- the client-initiated
    /// "quit game" signal, as opposed to the socket simply going quiet.
    pub fn disconnect(reason: u8) -> Vec<u8> {
        vec![0x0Cu8, reason]
    }
}

/// Build `[msg_id][u16 LE payload.len()][payload]` -- the WORD_LENGTH
/// framing every account/cell/entity-method client message beyond the
/// CONSTANT_LENGTH system range uses.
fn word_len_msg(msg_id: u8, payload: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(3 + payload.len());
    v.push(msg_id);
    v.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    v.extend_from_slice(payload);
    v
}
