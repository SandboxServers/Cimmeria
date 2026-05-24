//! Category 2 — fragment reassembly across paired channels.
//!
//! Existing unit tests in `unpacker/tests.rs` reassemble fragments
//! against a single in-memory `FragmentAssembler`. These tests pair
//! two real channels and verify reassembly survives the round-trip
//! over loopback, including out-of-order arrival and per-fragment loss.

use std::time::Duration;

use crate::encryption::MercuryEncryption;
use crate::packet::FRAGMENT_BODY_SIZE;
use crate::test_harness::LoopbackSession;

/// 3-fragment payload sent reliably; B reassembles to the byte-identical
/// original.
#[tokio::test]
async fn large_payload_fragments_send_and_reassemble() {
    let session = LoopbackSession::connected(None).await.unwrap();

    // ~3 fragments worth of body (deterministic content).
    let body: Vec<u8> = (0..(FRAGMENT_BODY_SIZE * 3 - 17))
        .map(|i| (i & 0xFF) as u8)
        .collect();
    session.a.send_bundle(&body, true).await.unwrap();

    let bundles = session.b.recv_n_bundles(1, Duration::from_secs(2)).await;
    assert_eq!(bundles.len(), 1, "B must reassemble exactly one bundle");
    assert_eq!(
        bundles[0].as_ref(),
        body.as_slice(),
        "reassembled body must be byte-identical to the original"
    );
}

/// A 4-fragment send with the A→B latency policy active — fragments
/// arrive over a (real but small) latency window and B still
/// reassembles. Proves the assembler tolerates time-spread fragment
/// arrival.
#[tokio::test]
async fn fragments_reassemble_under_latency_policy() {
    let session = LoopbackSession::connected(None).await.unwrap();

    session.policy.lock().unwrap().latency.a_to_b = Duration::from_millis(5);

    let body: Vec<u8> = (0..(FRAGMENT_BODY_SIZE * 4))
        .map(|i| ((i * 31) & 0xFF) as u8)
        .collect();
    session.a.send_bundle(&body, true).await.unwrap();

    let bundles = session.b.recv_n_bundles(1, Duration::from_secs(2)).await;
    assert_eq!(bundles.len(), 1);
    assert_eq!(bundles[0].as_ref(), body.as_slice());
}

/// Drop the **middle fragment** of a 3-fragment bundle on its way
/// out. Without retransmit, B's assembler hangs waiting for the
/// missing fragment. After advancing A's clock past RTO and ticking,
/// A's retransmit scan re-emits **only the lost fragment** (per-
/// fragment TX-window entries — not the whole bundle), B's assembler
/// completes, and the reassembled bundle equals the original.
#[tokio::test]
async fn single_fragment_loss_causes_retransmit_of_only_lost_fragment() {
    let session = LoopbackSession::connected(None).await.unwrap();

    let body: Vec<u8> = (0..(FRAGMENT_BODY_SIZE * 3))
        .map(|i| (i & 0xFF) as u8)
        .collect();

    // Reset counters so `drop_at_send_count` indexes from the
    // upcoming send_bundle call rather than including any prior
    // policy/handshake traffic.
    {
        let mut policy = session.policy.lock().unwrap();
        policy.reset_counters();
        // Drop the 2nd send (= fragment 2 of the 3-fragment bundle).
        policy.drop_at_send_count.a_to_b = Some(2);
    }

    session.a.send_bundle(&body, true).await.unwrap();

    // With fragment 2 dropped, B has fragments 1 and 3 buffered but
    // the bundle is incomplete. recv_n_bundles times out empty-handed.
    let early = session
        .b
        .recv_n_bundles(1, Duration::from_millis(100))
        .await;
    assert!(
        early.is_empty(),
        "bundle must NOT reassemble while fragment 2 is missing"
    );
    assert_eq!(
        session.a.tx_window_len(),
        3,
        "all 3 fragments must still be in A's TX window awaiting ack"
    );

    // Advance A's clock past RTO so the retransmit scan fires.
    session.a.clock.advance(Duration::from_secs(2));
    let (a_actions, _) = session.tick().await.unwrap();
    assert_eq!(
        a_actions.retransmits.len(),
        3,
        "all 3 unacked TX-window entries are past RTO, so all 3 are re-emitted; \
         fragments 1 and 3 are duplicates (B already has them), fragment 2 is \
         the one that completes the bundle"
    );

    // B's assembler now has fragment 2; the bundle completes.
    let bundles = session.b.recv_n_bundles(1, Duration::from_secs(2)).await;
    assert_eq!(bundles.len(), 1, "bundle must reassemble after retransmit");
    assert_eq!(bundles[0].as_ref(), body.as_slice());
}

/// Encrypted 3-fragment bundle round-trips. Each fragment is
/// independently AES-256-CBC encrypted with a 16-byte HMAC-MD5 tag
/// appended — so the per-fragment wire payload grows by up to 16
/// bytes (PKCS7 padding) + 16 bytes (HMAC tag) over the plaintext.
/// `FRAGMENT_BODY_SIZE = 1300` was chosen with that overhead in
/// mind; this test pins that the chunker still produces fragments
/// that fit under the MTU after encryption and that B's recv pump
/// reassembles to the byte-identical original plaintext.
#[tokio::test]
async fn fragments_round_trip_through_encryption() {
    let enc = MercuryEncryption::from_session_key([0x73u8; 32]);
    let session = LoopbackSession::connected(Some(enc)).await.unwrap();

    let body: Vec<u8> = (0..(FRAGMENT_BODY_SIZE * 3 - 7))
        .map(|i| ((i * 17) & 0xFF) as u8)
        .collect();
    session.a.send_bundle(&body, true).await.unwrap();

    let bundles = session.b.recv_n_bundles(1, Duration::from_secs(2)).await;
    assert_eq!(
        bundles.len(),
        1,
        "B must reassemble the encrypted fragmented bundle"
    );
    assert_eq!(
        bundles[0].as_ref(),
        body.as_slice(),
        "encrypted-and-fragmented round-trip must recover the byte-identical plaintext"
    );
}

/// Two concurrent reliable fragmented bundles. Each fragments
/// independently and the assembler keeps the two bundle states
/// separate (no cross-contamination).
#[tokio::test]
async fn concurrent_bundles_dont_cross_contaminate_reassembly() {
    let session = LoopbackSession::connected(None).await.unwrap();

    let body_x: Vec<u8> = (0..(FRAGMENT_BODY_SIZE * 3))
        .map(|i| (i & 0xFF) as u8)
        .collect();
    let body_y: Vec<u8> = (0..(FRAGMENT_BODY_SIZE * 2))
        .map(|i| ((i ^ 0xAA) & 0xFF) as u8)
        .collect();

    session.a.send_bundle(&body_x, true).await.unwrap();
    session.a.send_bundle(&body_y, true).await.unwrap();

    let bundles = session.b.recv_n_bundles(2, Duration::from_secs(3)).await;
    assert_eq!(bundles.len(), 2, "B must reassemble both bundles");

    // Arrival order: bundle X first (sent first; per-direction ordering
    // is preserved by the harness contract).
    assert_eq!(bundles[0].as_ref(), body_x.as_slice());
    assert_eq!(bundles[1].as_ref(), body_y.as_slice());
}
