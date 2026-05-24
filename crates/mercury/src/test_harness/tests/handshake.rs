//! Category 5 — channel lifecycle handshake.
//!
//! Cimmeria's current handshake shape is the minimal one: both peers
//! send an `FLAG_ON_CHANNEL` empty packet, both observe arrival, both
//! flip to `Connected`. These tests exercise the state-machine
//! observation: `unconnected` peers stay in `Connecting` until the
//! exchange completes; an unanswered first send doesn't advance state.
//!
//! When real handshake-content tests land (Phase-3 login bytes, key
//! exchange echoes), they belong here.

use crate::channel::ChannelState;
use crate::test_harness::LoopbackSession;

/// `unconnected()` returns with both channels in `Connecting`.
#[tokio::test]
async fn unconnected_session_starts_both_channels_in_connecting() {
    let session = LoopbackSession::unconnected(None).await.unwrap();

    assert_eq!(
        session.a.channel.lock().unwrap().state,
        ChannelState::Connecting,
    );
    assert_eq!(
        session.b.channel.lock().unwrap().state,
        ChannelState::Connecting,
    );
}

/// `connected()` drives the handshake and returns with both channels
/// in `Connected`.
#[tokio::test]
async fn connected_session_drives_handshake_to_connected() {
    let session = LoopbackSession::connected(None).await.unwrap();

    assert_eq!(
        session.a.channel.lock().unwrap().state,
        ChannelState::Connected,
    );
    assert_eq!(
        session.b.channel.lock().unwrap().state,
        ChannelState::Connected,
    );
}

/// A sends to B but B never replies; A's channel stays in `Connecting`.
/// The state machine doesn't advance just because we **sent** something.
#[tokio::test]
async fn unrequited_first_packet_does_not_advance_state() {
    let session = LoopbackSession::unconnected(None).await.unwrap();

    session.a.send_bundle(b"hello", false).await.unwrap();

    // Sync on B's recv pump: wait for B to actually receive the packet
    // (rather than wall-clock-sleeping and hoping). The wait succeeds
    // when B's recv pump has processed the packet end-to-end.
    let bundles = session
        .b
        .recv_n_bundles(1, std::time::Duration::from_secs(1))
        .await;
    assert_eq!(bundles.len(), 1, "B's recv pump must observe A's packet");

    // A sent and B received; neither side has acked anything. A's
    // state must stay `Connecting` — the handshake state machine
    // does not advance on `send` alone, only on observed peer
    // acknowledgement (which never came).
    assert_eq!(
        session.a.channel.lock().unwrap().state,
        ChannelState::Connecting,
        "A's state must not advance without a peer-originated acknowledgement",
    );
}

/// **Pins the current state-machine shape.** Exchanging the
/// handshake packets in both directions does NOT auto-advance either
/// channel to `Connected` — only [`LoopbackSession::connected`]'s
/// explicit `state = Connected` writes do that.
///
/// Today the `Channel` struct has no built-in observer that watches
/// inbound packets and decides "the handshake is now complete." When
/// real handshake state-machine logic lands on `Channel` (mirroring
/// the C++ `Mercury::Channel`'s lifecycle hooks), this test should
/// flip to asserting `ChannelState::Connected` after the bilateral
/// exchange — at which point [`LoopbackSession::connected`]'s manual
/// state writes can be deleted.
///
/// Keeping the assertion negative ("still in Connecting") rather
/// than positive makes the test fail when the auto-transition lands,
/// surfacing the cleanup work in the same PR that ships the feature.
#[tokio::test]
async fn bilateral_packet_exchange_does_not_yet_auto_transition_to_connected() {
    let session = LoopbackSession::unconnected(None).await.unwrap();

    // Bilateral exchange of the same empty FLAG_ON_CHANNEL packet
    // shape `LoopbackSession::connected` uses internally.
    session.a.send_bundle(&[], false).await.unwrap();
    session.b.send_bundle(&[], false).await.unwrap();

    // Sync on both recv pumps so the assertion is made *after* the
    // packets land.
    let a_received = session
        .a
        .recv_n_bundles(1, std::time::Duration::from_secs(1))
        .await;
    let b_received = session
        .b
        .recv_n_bundles(1, std::time::Duration::from_secs(1))
        .await;
    assert_eq!(a_received.len(), 1);
    assert_eq!(b_received.len(), 1);

    // Even with both sides having observed each other's handshake
    // bytes, the state stays `Connecting`. When this assertion
    // starts failing, the Channel has gained real handshake logic
    // and `LoopbackSession::connected`'s manual writes should be
    // deleted in the same PR.
    assert_eq!(
        session.a.channel.lock().unwrap().state,
        ChannelState::Connecting,
        "A: Channel has no auto-handshake observer yet — state must stay Connecting",
    );
    assert_eq!(
        session.b.channel.lock().unwrap().state,
        ChannelState::Connecting,
        "B: Channel has no auto-handshake observer yet — state must stay Connecting",
    );
}
