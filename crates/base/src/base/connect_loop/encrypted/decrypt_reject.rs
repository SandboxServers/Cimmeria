//! What to log when a datagram from an established session fails to decrypt.
//!
//! A decrypt failure on an established channel is dropped, never a teardown.
//! The legacy C++ `EncryptionFilter` dropped a bad-length packet at TRACE
//! and kept the channel (`deprecated/cpp/src/mercury/encryption_filter.cpp`).
//! Tearing the session down would also let anyone who can spoof the client's
//! source address kill it with one garbage datagram.
//!
//! One shape gets its own row: the client's **plaintext `baseAppLogin`**
//! arriving after the server has already registered the channel. The
//! client's `ServerSelectSuccess` handler (`ghidra://SGW.exe@0x00ddfd00`)
//! arms a 300 ms repeating timer (`0x493e0` µs). Its tick
//! (`ghidra://SGW.exe@0x00de10b0`) sends another `baseAppLogin` attempt
//! from the same socket for as long as attempts remain and the login reply
//! handler has not finished. So a train of these at 300 ms means the
//! server accepted the login and replied, and the client never completed
//! its reply handler. That is a login stuck on the client side. It is not
//! a bad HMAC or a key mismatch, and the old "Decryption failed (bad
//! HMAC?)" row pointed triage the wrong way (colo, 2026-09-26: one tester,
//! 23 of these, two stuck logins).
//!
//! A v1 encrypted datagram is always `16k + 16` bytes long. A 20-character
//! ticket makes the plaintext login 41 bytes, so the two cannot collide,
//! and `parse_baseapp_login` checks the full shape anyway.
//!
//! The retry row carries `reply_outstanding`, which tells the two causes
//! apart (#842). `true`: the server has not seen the client's ACK of the
//! reply (seq 1), so the reply was probably lost, and the channel's
//! retransmit scan resends it on the next RTO. `false`: the client acked
//! the reply and is retrying anyway, the stuck-client case above, where a
//! resend would change nothing.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use super::super::super::login::{parse_baseapp_login, CONNECT_REPLY_SEQ};
use super::super::super::session_identity;
use super::super::super::ConnectedClientState;

/// Stable `reason` for a datagram that failed to decrypt and is not a
/// recognisable client message.
pub(super) const REASON_DECRYPT_FAIL: &str = "decrypt_fail";

/// Stable `reason` for a plaintext `baseAppLogin` retry that arrives on an
/// already-established channel.
pub(super) const REASON_LOGIN_RETRY_ON_CHANNEL: &str = "login_retry_on_channel";

/// `true` when `raw` is a well-formed plaintext `baseAppLogin` datagram.
pub(super) fn is_plaintext_login(raw: &[u8]) -> bool {
    parse_baseapp_login(raw).is_ok()
}

/// `true` while the session's login reply is still waiting for the
/// client's ACK: it sits in the channel's TX window (or deferred queue)
/// at [`CONNECT_REPLY_SEQ`]. `false` once acked, or when the session or
/// its channel cannot be read.
pub(super) fn reply_outstanding(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) -> bool {
    let Ok(clients) = connected.lock() else {
        return false;
    };
    let Some(state) = clients.get(&addr) else {
        return false;
    };
    let Ok(channel) = state.channel.lock() else {
        return false;
    };
    channel
        .tx_window
        .iter()
        .chain(channel.unsent_packets.iter())
        .any(|entry| entry.packet.sequence == CONNECT_REPLY_SEQ)
}

/// Log one dropped datagram. The session is left as it is.
pub(super) fn log_decrypt_reject(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    account_id: u32,
    raw: &[u8],
    error: &dyn std::fmt::Display,
) {
    if is_plaintext_login(raw) {
        tracing::warn!(
            %addr,
            account_id,
            account_name = session_identity::identity_for_addr(connected, addr).account_name,
            raw_len = raw.len(),
            reason = REASON_LOGIN_RETRY_ON_CHANNEL,
            reply_outstanding = reply_outstanding(connected, addr),
            "Client is retrying baseAppLogin on an established channel: it did not complete the login reply; dropping the retry"
        );
    } else {
        tracing::warn!(
            %addr,
            account_id,
            account_name = session_identity::identity_for_addr(connected, addr).account_name,
            raw_len = raw.len(),
            reason = REASON_DECRYPT_FAIL,
            error = %error,
            "Decryption failed; dropping the datagram (the session stays up)"
        );
    }
}
