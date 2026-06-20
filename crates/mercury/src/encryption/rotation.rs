//! Server-initiated Mercury session-key rotation (v2 sessions only).
//!
//! # Why rotation exists
//!
//! A v2 session pins one 32-byte session key for its whole lifetime. The
//! longer a key is in use, the more ciphertext an attacker can collect under
//! it. Periodic rotation bounds that exposure: the server mints a fresh key
//! every cadence interval (default ~1 h), hands it to the peer **encrypted
//! under the current key**, and both sides switch to it.
//!
//! # v1 is never rotated
//!
//! The unmodified game client speaks v1 and has no code path to receive a
//! [`RotateSessionKey`](crate::messages::MsgId::RotateSessionKey) message —
//! sending one would feed it a bundle it cannot parse and break the session.
//! Rotation is therefore **hard-gated to v2** by [`rotation_enabled`]: a v1
//! session, regardless of configured cadence, never arms a rotation and never
//! emits a rotation message. This keeps today's clients byte-identical to the
//! pre-rotation behavior.
//!
//! # Wire message
//!
//! The rotation control message body is simply the new 32-byte key encrypted
//! under the *current* session context:
//!
//! ```text
//! [ encrypt_current( new_key[0..32] ) ]
//! ```
//!
//! Because the key travels inside the existing v2 AEAD-ish envelope (random
//! IV, HKDF-split keys, encrypt-then-MAC), it is confidential and integrity-
//! protected on the wire — the 32 raw key bytes never appear in the clear.
//! The peer recovers the key by decrypting under the still-current context,
//! then rebuilds its own [`MercuryEncryption`] from it.
//!
//! # Switch semantics (server → peer rotation)
//!
//! The hard problem is "no packet is encrypted with one key and decrypted
//! with the other". The ordering this module enforces:
//!
//! 1. **Server arms.** It generates a fresh key, encrypts it under the current
//!    context, and stages the new context as *pending* — but keeps encrypting
//!    its own outbound traffic with the **old** key until the rotation message
//!    itself has gone out.
//! 2. **Server switches outbound.** After the `RotateSessionKey` message is on
//!    the wire, [`RotationState::commit_outbound`] promotes the pending context
//!    to current. Every subsequent outbound packet uses the new key.
//! 3. **Peer applies immediately.** On receiving and decrypting (under the old
//!    key) the rotation message, the peer rebuilds *both* its inbound and
//!    outbound contexts from the new key via [`apply_received_key`]. Its very
//!    next outbound packet is already under the new key.
//! 4. **Server inbound accepts both keys during the window.** Between arming
//!    (step 1) and observing the first inbound packet that decrypts under the
//!    new key, the server cannot know whether an arriving packet was sent
//!    before or after the peer applied the rotation. [`RotationState::decrypt`]
//!    therefore tries the **new** context first and falls back to the **old**
//!    context for the duration of the window, then drops the old key once a new
//!    packet has been seen. This is what makes the switch race-free: an in-
//!    flight packet sent under the old key right before the peer switched still
//!    decrypts.
//!
//! The narrowed scope (server→peer only, with the peer applying on receipt)
//! avoids a full multi-round handshake while still being loss-tolerant for the
//! data plane: the rotation message is sent reliably by the transport layer, so
//! a dropped rotation message is retransmitted (byte-identically, under the old
//! key) until acked.

use crate::encryption::{EncryptionVersion, MercuryEncryption};
use cimmeria_common::{CimmeriaError, Result};

/// The 32-byte session key carried by a rotation message.
pub const SESSION_KEY_LEN: usize = 32;

/// Decide whether session-key rotation is active for a session.
///
/// Rotation runs **only** when the session speaks v2 *and* the configured
/// cadence is non-zero. A v1 session never rotates (the stock client cannot
/// process the control message); a zero cadence disables rotation regardless
/// of version. This is the single gate the scheduler and the send path consult
/// before ever minting or sending a [`RotateSessionKey`].
pub fn rotation_enabled(version: EncryptionVersion, cadence_secs: u64) -> bool {
    matches!(version, EncryptionVersion::V2) && cadence_secs > 0
}

/// Generate a fresh 32-byte session key from the OS CSPRNG.
///
/// Used by the server when arming a rotation. `rand::random` pulls from the
/// same OS-seeded generator the v2 per-packet IV uses.
pub fn generate_session_key() -> [u8; SESSION_KEY_LEN] {
    rand::random()
}

/// Build the `RotateSessionKey` message body: the `new_key` encrypted under
/// `current` (the still-active session context).
///
/// The returned bytes are the message payload — the caller frames them with the
/// [`MsgId::RotateSessionKey`](crate::messages::MsgId::RotateSessionKey) opcode
/// and ships them on the channel like any other control message. The raw key
/// never appears in the output; it is inside the v2 ciphertext envelope.
pub fn build_rotation_payload(
    current: &MercuryEncryption,
    new_key: &[u8; SESSION_KEY_LEN],
) -> Result<Vec<u8>> {
    // Defense-in-depth: rotation is a v2-only protocol. Refuse to build a
    // payload under a v1 context so a future caller can never accidentally
    // emit a RotateSessionKey on a v1 session (which the stock client cannot
    // parse) — even if the `rotation_enabled` gate upstream is bypassed.
    if !current.is_v2() {
        return Err(CimmeriaError::Encryption(
            "session-key rotation requires a v2 context".to_string(),
        ));
    }
    current.encrypt(new_key)
}

/// Recover the new key from a received `RotateSessionKey` payload by decrypting
/// it under `current`.
///
/// Returns the 32-byte key on success. Fails if the payload does not decrypt
/// (HMAC mismatch / wrong key / truncation) or does not decrypt to exactly 32
/// bytes — both are treated as a malformed/forged rotation and the caller must
/// NOT switch keys.
pub fn recover_rotation_key(
    current: &MercuryEncryption,
    payload: &[u8],
) -> Result<[u8; SESSION_KEY_LEN]> {
    // Mirror the v2 gate in `build_rotation_payload`: only a v2 context ever
    // legitimately carries a rotation payload.
    if !current.is_v2() {
        return Err(CimmeriaError::Encryption(
            "session-key rotation requires a v2 context".to_string(),
        ));
    }
    let plaintext = current.decrypt(payload)?;
    if plaintext.len() != SESSION_KEY_LEN {
        return Err(CimmeriaError::Encryption(format!(
            "rotation payload decrypted to {} bytes, expected {SESSION_KEY_LEN}",
            plaintext.len()
        )));
    }
    let mut key = [0u8; SESSION_KEY_LEN];
    key.copy_from_slice(&plaintext);
    Ok(key)
}

/// Per-session rotation state machine.
///
/// Wraps the active [`MercuryEncryption`] context and the bookkeeping needed to
/// switch keys without losing or mis-decrypting a packet. A session pins one
/// version ([`EncryptionVersion::V2`] for any session that ever rotates); the
/// state holds the current key so it can mint and apply replacements.
///
/// All three phases of a rotation are encoded here:
/// - [`arm`](Self::arm) stages a pending new context (server side).
/// - [`commit_outbound`](Self::commit_outbound) promotes pending → current
///   after the rotation message is sent (server side).
/// - [`apply_received_key`](Self::apply_received_key) rebuilds both directions
///   from a key recovered off the wire (peer side).
///
/// [`encrypt`](Self::encrypt) / [`decrypt`](Self::decrypt) wrap the active
/// context, with [`decrypt`](Self::decrypt) implementing the dual-key
/// acceptance window described in the module docs.
#[derive(Debug, Clone)]
pub struct RotationState {
    /// Wire version (always v2 when rotation is in play; carried so a future
    /// path that constructs a [`RotationState`] for a v1 session degrades to a
    /// plain passthrough rather than mis-deriving keys).
    version: EncryptionVersion,
    /// The active session key.
    current_key: [u8; SESSION_KEY_LEN],
    /// The active encryption context, derived from `current_key`.
    current: MercuryEncryption,
    /// Pending context staged by [`arm`](Self::arm) but not yet promoted —
    /// `Some` between arming and [`commit_outbound`](Self::commit_outbound).
    pending: Option<PendingRotation>,
    /// During the inbound acceptance window, the previous context is kept so an
    /// in-flight packet sent under the old key still decrypts. Cleared once a
    /// packet has decrypted under the new key.
    previous: Option<MercuryEncryption>,
}

/// A rotation that has been armed (new key minted) but whose outbound switch
/// has not yet been committed.
#[derive(Debug, Clone)]
struct PendingRotation {
    new_key: [u8; SESSION_KEY_LEN],
    new_ctx: MercuryEncryption,
}

impl RotationState {
    /// Create a rotation state around an initial session key and version.
    pub fn new(key: [u8; SESSION_KEY_LEN], version: EncryptionVersion) -> Self {
        Self {
            version,
            current_key: key,
            current: MercuryEncryption::from_session_key_versioned(key, version),
            pending: None,
            previous: None,
        }
    }

    /// The active session key (for callers that persist it on the session).
    pub fn current_key(&self) -> [u8; SESSION_KEY_LEN] {
        self.current_key
    }

    /// True while a rotation has been armed but the outbound switch has not yet
    /// been committed.
    pub fn is_armed(&self) -> bool {
        self.pending.is_some()
    }

    /// Recover a new key from an inbound `RotateSessionKey` payload by
    /// decrypting it under the **current** context. The peer-side counterpart
    /// to [`arm`](Self::arm): the sender encrypted the key under the key both
    /// sides still hold, so the receiver decrypts under its current context
    /// before switching. Follow with [`apply_received_key`](Self::apply_received_key).
    pub fn recover_received_key(&self, payload: &[u8]) -> Result<[u8; SESSION_KEY_LEN]> {
        recover_rotation_key(&self.current, payload)
    }

    /// Encrypt under the active (current) context.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        self.current.encrypt(plaintext)
    }

    /// Arm a rotation: build the [`RotateSessionKey`] payload encrypting
    /// `new_key` under the current context and stage the new context as
    /// pending. The session keeps encrypting outbound traffic with the OLD key
    /// until [`commit_outbound`](Self::commit_outbound) runs — so the rotation
    /// message itself is decryptable by the peer under the key it still holds.
    ///
    /// Returns the rotation message payload to ship on the channel.
    ///
    /// Refuses to arm a second rotation while one is still pending: the first
    /// payload may already be on the wire and applied by the peer, so minting a
    /// different key before committing would desync the two sides. Callers must
    /// [`commit_outbound`](Self::commit_outbound) (or otherwise resolve the
    /// pending rotation) before arming again.
    pub fn arm(&mut self, new_key: [u8; SESSION_KEY_LEN]) -> Result<Vec<u8>> {
        if self.pending.is_some() {
            return Err(CimmeriaError::Encryption(
                "rotation already armed; commit the pending rotation before arming another"
                    .to_string(),
            ));
        }
        let payload = build_rotation_payload(&self.current, &new_key)?;
        let new_ctx = MercuryEncryption::from_session_key_versioned(new_key, self.version);
        self.pending = Some(PendingRotation { new_key, new_ctx });
        Ok(payload)
    }

    /// Promote the pending context to current. Call this **after** the rotation
    /// message has gone on the wire so every subsequent outbound packet uses
    /// the new key. The old context moves into the inbound acceptance window
    /// ([`previous`](Self::previous)) so packets the peer sent before it
    /// switched still decrypt.
    ///
    /// No-op if no rotation is armed.
    pub fn commit_outbound(&mut self) {
        if let Some(pending) = self.pending.take() {
            // Old context stays usable for inbound until a new-key packet is
            // observed (race window with the peer's switch).
            self.previous = Some(std::mem::replace(&mut self.current, pending.new_ctx));
            self.current_key = pending.new_key;
        }
    }

    /// Apply a key recovered from an inbound `RotateSessionKey` message (peer
    /// side). Rebuilds the active context from `new_key` for BOTH directions
    /// and keeps the old context in the inbound acceptance window so any
    /// already-in-flight packet still decrypts.
    pub fn apply_received_key(&mut self, new_key: [u8; SESSION_KEY_LEN]) {
        let new_ctx = MercuryEncryption::from_session_key_versioned(new_key, self.version);
        self.previous = Some(std::mem::replace(&mut self.current, new_ctx));
        self.current_key = new_key;
        // A peer that applied a received key has no separately-armed pending
        // rotation of its own to keep.
        self.pending = None;
    }

    /// Decrypt an inbound datagram, implementing the dual-key acceptance
    /// window.
    ///
    /// Tries the current context first. If that fails AND a previous context is
    /// still in the window, falls back to it — this catches a packet the peer
    /// sent under the old key just before switching. The first success under
    /// the **current** context closes the window (drops `previous`), since once
    /// a new-key packet is seen the peer has definitively switched and no
    /// further old-key packets can arrive in order.
    pub fn decrypt(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        match self.current.decrypt(data) {
            Ok(plaintext) => {
                // A successful current-key decrypt means the peer has switched;
                // retire the old context so a future rotation reuses the slot
                // and an attacker can't replay an old-key packet later.
                self.previous = None;
                Ok(plaintext)
            }
            Err(current_err) => {
                if let Some(prev) = &self.previous {
                    if let Ok(plaintext) = prev.decrypt(data) {
                        return Ok(plaintext);
                    }
                }
                Err(current_err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> [u8; SESSION_KEY_LEN] {
        [byte; SESSION_KEY_LEN]
    }

    #[test]
    fn rotation_gated_to_v2_with_nonzero_cadence() {
        assert!(rotation_enabled(EncryptionVersion::V2, 3600));
        assert!(rotation_enabled(EncryptionVersion::V2, 1));
        // v1 never rotates, even with a cadence configured.
        assert!(!rotation_enabled(EncryptionVersion::V1, 3600));
        // Zero cadence disables rotation even on v2.
        assert!(!rotation_enabled(EncryptionVersion::V2, 0));
        assert!(!rotation_enabled(EncryptionVersion::V1, 0));
    }

    #[test]
    fn rotation_payload_does_not_contain_raw_key() {
        let initial = key(0x11);
        let ctx = MercuryEncryption::from_session_key_versioned(initial, EncryptionVersion::V2);
        let new_key = key(0xAB);
        let payload = build_rotation_payload(&ctx, &new_key).unwrap();

        // The raw 32-byte key must not appear as a contiguous run anywhere in
        // the on-wire payload — it is inside the v2 ciphertext envelope.
        assert!(
            !payload.windows(SESSION_KEY_LEN).any(|w| w == new_key),
            "raw new key leaked into rotation payload"
        );
        // And the v2 envelope is present (leading version byte).
        assert_eq!(payload[0], 0x02, "rotation payload must be a v2 frame");
    }

    #[test]
    fn recover_round_trips_the_armed_key() {
        let initial = key(0x22);
        let mut server = RotationState::new(initial, EncryptionVersion::V2);
        let new_key = key(0x55);
        let payload = server.arm(new_key).unwrap();

        // Peer decrypts under the key it still holds (the initial key).
        let peer_ctx =
            MercuryEncryption::from_session_key_versioned(initial, EncryptionVersion::V2);
        let recovered = recover_rotation_key(&peer_ctx, &payload).unwrap();
        assert_eq!(recovered, new_key);
    }

    #[test]
    fn recover_rejects_wrong_size_plaintext() {
        let initial = key(0x33);
        let ctx = MercuryEncryption::from_session_key_versioned(initial, EncryptionVersion::V2);
        // Encrypt a 16-byte blob (not a valid key length).
        let payload = ctx.encrypt(&[0u8; 16]).unwrap();
        assert!(recover_rotation_key(&ctx, &payload).is_err());
    }

    #[test]
    fn outbound_switches_only_after_commit() {
        let initial = key(0x44);
        let mut server = RotationState::new(initial, EncryptionVersion::V2);
        let new_key = key(0x99);

        // Before arming: encrypts under the initial key.
        let initial_ctx =
            MercuryEncryption::from_session_key_versioned(initial, EncryptionVersion::V2);

        // Arm — outbound must still be the old key (the rotation message must be
        // decryptable by the peer under the key it holds).
        let _payload = server.arm(new_key).unwrap();
        assert!(server.is_armed());
        let pre_commit = server.encrypt(b"hello").unwrap();
        assert!(
            initial_ctx.decrypt(&pre_commit).is_ok(),
            "pre-commit outbound must still decrypt under the old key"
        );

        // Commit — outbound now uses the new key.
        server.commit_outbound();
        assert!(!server.is_armed());
        let new_ctx = MercuryEncryption::from_session_key_versioned(new_key, EncryptionVersion::V2);
        let post_commit = server.encrypt(b"hello").unwrap();
        assert!(
            new_ctx.decrypt(&post_commit).is_ok(),
            "post-commit outbound must decrypt under the new key"
        );
        assert!(
            initial_ctx.decrypt(&post_commit).is_err(),
            "post-commit outbound must NOT decrypt under the old key"
        );
    }

    #[test]
    fn inbound_window_accepts_old_then_new_key() {
        let initial = key(0x66);
        let mut server = RotationState::new(initial, EncryptionVersion::V2);
        let new_key = key(0xCC);
        server.arm(new_key).unwrap();
        server.commit_outbound();

        // A packet the peer sent under the OLD key, in flight across the
        // switch, must still decrypt during the window.
        let old_ctx = MercuryEncryption::from_session_key_versioned(initial, EncryptionVersion::V2);
        let in_flight = old_ctx.encrypt(b"old-key packet").unwrap();
        assert_eq!(server.decrypt(&in_flight).unwrap(), b"old-key packet");

        // A packet under the NEW key decrypts and closes the window.
        let new_ctx = MercuryEncryption::from_session_key_versioned(new_key, EncryptionVersion::V2);
        let fresh = new_ctx.encrypt(b"new-key packet").unwrap();
        assert_eq!(server.decrypt(&fresh).unwrap(), b"new-key packet");

        // Window closed: an old-key packet no longer decrypts.
        let after = old_ctx.encrypt(b"too late").unwrap();
        assert!(
            server.decrypt(&after).is_err(),
            "old key must be retired once a new-key packet is observed"
        );
    }

    #[test]
    fn peer_apply_round_trips_both_directions() {
        // Server and peer both start on the same key.
        let initial = key(0x77);
        let mut server = RotationState::new(initial, EncryptionVersion::V2);
        let mut peer = RotationState::new(initial, EncryptionVersion::V2);

        // Server arms + sends the rotation message, then commits.
        let new_key = generate_session_key();
        let payload = server.arm(new_key).unwrap();
        // Peer recovers under the still-current key and applies.
        let recovered = recover_rotation_key(&peer.current, &payload).unwrap();
        assert_eq!(recovered, new_key);
        peer.apply_received_key(recovered);
        server.commit_outbound();

        // Both directions now talk under the new key.
        let s2p = server.encrypt(b"server to peer").unwrap();
        assert_eq!(peer.decrypt(&s2p).unwrap(), b"server to peer");

        let p2s = peer.encrypt(b"peer to server").unwrap();
        assert_eq!(server.decrypt(&p2s).unwrap(), b"peer to server");
    }

    #[test]
    fn build_and_recover_reject_v1_context() {
        // Defense-in-depth: the rotation primitives must refuse a v1 context
        // even though the upstream `rotation_enabled` gate would normally keep
        // them away from v1 sessions.
        let v1 = MercuryEncryption::from_session_key_versioned(key(0x10), EncryptionVersion::V1);
        assert!(
            build_rotation_payload(&v1, &key(0x20)).is_err(),
            "rotation payload must not be built under a v1 context"
        );
        assert!(
            recover_rotation_key(&v1, &[0u8; 64]).is_err(),
            "rotation key must not be recovered under a v1 context"
        );
    }

    #[test]
    fn arm_twice_without_commit_errors() {
        let mut server = RotationState::new(key(0x88), EncryptionVersion::V2);
        server.arm(key(0xA1)).expect("first arm succeeds");
        assert!(
            server.arm(key(0xA2)).is_err(),
            "arming again before commit must error to avoid a server/peer desync"
        );
        // Once the pending rotation is committed, arming is allowed again.
        server.commit_outbound();
        assert!(
            server.arm(key(0xA3)).is_ok(),
            "arming after commit must succeed"
        );
    }
}
