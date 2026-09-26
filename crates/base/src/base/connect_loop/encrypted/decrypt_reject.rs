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

use std::net::SocketAddr;

use super::super::super::login::parse_baseapp_login;

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

/// Log one dropped datagram. The session is left as it is.
pub(super) fn log_decrypt_reject(
    addr: SocketAddr,
    account_id: u32,
    raw: &[u8],
    error: &dyn std::fmt::Display,
) {
    if is_plaintext_login(raw) {
        tracing::warn!(
            %addr,
            account_id,
            raw_len = raw.len(),
            reason = REASON_LOGIN_RETRY_ON_CHANNEL,
            "Client is retrying baseAppLogin on an established channel: it did not complete the login reply; dropping the retry"
        );
    } else {
        tracing::warn!(
            %addr,
            account_id,
            raw_len = raw.len(),
            reason = REASON_DECRYPT_FAIL,
            error = %error,
            "Decryption failed; dropping the datagram (the session stays up)"
        );
    }
}
