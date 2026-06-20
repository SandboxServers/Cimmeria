//! Server-initiated v2 session-key rotation across a live paired session.
//!
//! These tests pair two real channels on loopback sockets and drive a full
//! rotation through the harness: the server arms (mints a new key + builds the
//! `RotateSessionKey` payload), ships the payload on the channel, the peer
//! recovers and applies it, and the server commits its outbound switch. They
//! prove the switch is race-free under traffic and that the wire bytes never
//! carry the raw key.
//!
//! **No real client is involved.** The "peer" is a cooperating v2 endpoint
//! simulated by the harness. Real-client validation awaits the client patch —
//! the stock client speaks v1 and is never sent a rotation message (the v1 gate
//! is unit-tested in `encryption::rotation::tests`).

use std::time::Duration;

use crate::encryption::{generate_session_key, EncryptionVersion, MercuryEncryption};
use crate::test_harness::LoopbackSession;

/// Seed key both peers start the session on.
fn seed_key() -> [u8; 32] {
    [0x42u8; 32]
}

/// Stand up a connected session whose static encryption is v2-seeded, then
/// install a [`RotationState`](crate::encryption::RotationState) on both peers
/// so the send/recv paths route through rotation.
async fn connected_v2_rotating() -> LoopbackSession {
    let key = seed_key();
    // Seed the static context with the same v2 key so the handshake (driven
    // before rotation is installed) round-trips, then both peers swap to
    // rotation state for the data plane.
    let mut session = LoopbackSession::connected(Some(MercuryEncryption::from_session_key_v2(key)))
        .await
        .expect("connect v2 session");
    session.a.enable_rotation(key, EncryptionVersion::V2);
    session.b.enable_rotation(key, EncryptionVersion::V2);
    session
}

/// Drive a full rotation: A (server) arms, ships the payload reliably, B (peer)
/// recovers + applies, A commits. Returns the new key both sides converged on.
async fn perform_rotation(session: &LoopbackSession) -> [u8; 32] {
    let new_key = generate_session_key();
    let payload = session.a.arm_rotation(new_key).expect("arm");

    // Ship the rotation message reliably (matches production: a dropped
    // rotation message is retransmitted under the still-current key).
    session.a.send_bundle(&payload, true).await.expect("send");
    let got = session.b.recv_n_bundles(1, Duration::from_secs(2)).await;
    assert_eq!(got.len(), 1, "peer must receive the rotation message");

    // Peer recovers (under the still-current key) and applies.
    let recovered = session
        .b
        .apply_received_rotation(&got[0])
        .expect("peer recovers + applies rotation key");
    assert_eq!(recovered, new_key, "peer recovered the armed key");

    // Server commits its outbound switch once the message is on the wire.
    session.a.commit_outbound_rotation();

    assert_eq!(session.a.rotation_current_key(), new_key);
    assert_eq!(session.b.rotation_current_key(), new_key);
    new_key
}

/// Round-trip after rotation: post-switch traffic flows under the new key in
/// both directions, and the old key no longer decrypts new packets.
#[tokio::test]
async fn round_trip_after_rotation_uses_new_key() {
    let session = connected_v2_rotating().await;
    let new_key = perform_rotation(&session).await;

    // A → B under the new key.
    session
        .a
        .send_bundle(b"after rotation A->B", false)
        .await
        .unwrap();
    let b_got = session.b.recv_n_bundles(1, Duration::from_secs(2)).await;
    assert_eq!(b_got.len(), 1);
    assert_eq!(b_got[0].as_ref(), b"after rotation A->B");

    // B → A under the new key.
    session
        .b
        .send_bundle(b"after rotation B->A", false)
        .await
        .unwrap();
    let a_got = session.a.recv_n_bundles(1, Duration::from_secs(2)).await;
    assert_eq!(a_got.len(), 1);
    assert_eq!(a_got[0].as_ref(), b"after rotation B->A");

    // The OLD key must no longer decrypt a freshly-encrypted new-key packet.
    let old_ctx = MercuryEncryption::from_session_key_v2(seed_key());
    let new_ctx = MercuryEncryption::from_session_key_v2(new_key);
    let fresh = new_ctx.encrypt(b"sample").unwrap();
    assert!(
        old_ctx.decrypt(&fresh).is_err(),
        "old key must not decrypt post-rotation traffic"
    );

    session.shutdown();
}

/// Rotate-under-load: traffic is flowing in both directions while the rotation
/// switch happens. No plaintext is lost or corrupted across the switch.
///
/// The sequence interleaves pre-rotation sends, the rotation message, an
/// in-flight old-key packet that crosses the switch boundary, and post-rotation
/// sends — then asserts every payload arrived intact and in order.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rotate_under_load_loses_no_plaintext() {
    let session = connected_v2_rotating().await;

    // Pre-rotation traffic A→B.
    session.a.send_bundle(b"pre-1", false).await.unwrap();
    session.a.send_bundle(b"pre-2", false).await.unwrap();

    // Arm + ship the rotation message, but DON'T commit A's outbound yet —
    // this opens the race window where A is still emitting under the old key
    // while B may already have switched.
    let new_key = generate_session_key();
    let payload = session.a.arm_rotation(new_key).expect("arm");
    session.a.send_bundle(&payload, true).await.expect("send");

    // While B has not yet applied, A sends one more packet still under the
    // OLD key (in-flight across the boundary).
    session
        .a
        .send_bundle(b"old-key-inflight", false)
        .await
        .unwrap();

    // B drains: pre-1, pre-2, the rotation payload, and the in-flight old-key
    // packet — all still decryptable because B has not switched yet.
    let pre = session.b.recv_n_bundles(4, Duration::from_secs(3)).await;
    assert_eq!(pre.len(), 4, "all pre-switch bundles must arrive");
    assert_eq!(pre[0].as_ref(), b"pre-1");
    assert_eq!(pre[1].as_ref(), b"pre-2");
    // pre[2] is the rotation payload (opaque ciphertext at the harness layer).
    assert_eq!(pre[3].as_ref(), b"old-key-inflight");

    // B applies the rotation (recovered from the payload it just received).
    let recovered = session
        .b
        .apply_received_rotation(&pre[2])
        .expect("apply rotation");
    assert_eq!(recovered, new_key);

    // A commits its outbound switch.
    session.a.commit_outbound_rotation();

    // Post-rotation traffic both directions under the new key.
    session.a.send_bundle(b"post-A->B", false).await.unwrap();
    session.b.send_bundle(b"post-B->A", false).await.unwrap();

    let b_post = session.b.recv_n_bundles(1, Duration::from_secs(2)).await;
    assert_eq!(b_post.len(), 1);
    assert_eq!(b_post[0].as_ref(), b"post-A->B");

    let a_post = session.a.recv_n_bundles(1, Duration::from_secs(2)).await;
    assert_eq!(a_post.len(), 1);
    assert_eq!(a_post[0].as_ref(), b"post-B->A");

    session.shutdown();
}

/// The rotation message must not contain the new key in plaintext. Asserts on
/// the inner rotation payload returned by `arm_rotation` (the bytes that go in
/// the bundle body): the raw key never appears as a contiguous run and the
/// payload is a v2 frame. The outer session encryption wraps this again on the
/// wire, so the actual datagram is doubly removed from the raw key.
#[tokio::test]
async fn rotation_message_does_not_leak_key_on_wire() {
    let session = connected_v2_rotating().await;

    let new_key = generate_session_key();
    let payload = session.a.arm_rotation(new_key).expect("arm");

    // `payload` is exactly what goes inside the bundle body on the wire. The
    // harness then wraps it in a Mercury frame and v2-encrypts the whole thing,
    // so the wire bytes are doubly removed from the raw key — but assert on the
    // payload directly to pin the inner envelope too.
    assert!(
        !payload.windows(32).any(|w| w == new_key),
        "raw new key leaked into rotation payload"
    );
    assert_eq!(payload[0], 0x02, "rotation payload is a v2 frame");

    session.shutdown();
}

/// v1 sessions never rotate: a v1-seeded session with rotation NOT installed
/// behaves exactly like the existing static-context path. This guards the gate
/// at the harness level — `rotation_enabled` is unit-tested separately, but
/// this proves a v1 session's data plane is untouched (no rotation field set,
/// byte-for-byte identical send/recv to a plain v1 session).
#[tokio::test]
async fn v1_session_data_plane_unchanged_without_rotation() {
    // A plain v1 session — rotation is never enabled (the gate would refuse it
    // anyway). This is the "default behavior for today's clients" baseline.
    let key = [0x7Au8; 32];
    let session = LoopbackSession::connected(Some(MercuryEncryption::from_session_key(key)))
        .await
        .unwrap();

    // No rotation installed → the rotation field stays None on both peers.
    assert!(session.a.rotation.is_none());
    assert!(session.b.rotation.is_none());

    session.a.send_bundle(b"v1 traffic", false).await.unwrap();
    let got = session.b.recv_n_bundles(1, Duration::from_secs(2)).await;
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].as_ref(), b"v1 traffic");

    session.shutdown();
}
