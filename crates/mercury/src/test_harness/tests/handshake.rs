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

use std::time::Duration;

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

    session.a.send_bundle(&[], false).await.unwrap();

    // Let B's recv pump observe the packet; A's state shouldn't change.
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert_eq!(
        session.a.channel.lock().unwrap().state,
        ChannelState::Connecting,
        "A's state must not advance on a one-way send",
    );
}
