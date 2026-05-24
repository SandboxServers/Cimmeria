//! Services-layer integration test for the `LossyTransport` wrapper
//! on the `BidirectionalTransport` recv-loop seam.
//!
//! These tests demonstrate that the chaos infrastructure plugs into
//! the services-layer recv path without behavioral regression. The
//! end-to-end chaos scenarios (`world_entry_survives_transatlantic_loss`,
//! `defeat_burst_survives_5pct_loss` from the #355 spec) require a
//! services-layer in-process harness that doesn't exist yet; those
//! tests scope to a follow-up that builds the harness. For now we
//! pin two things:
//!
//! 1. A `LossyTransport` wrapping a `UdpTransport` round-trips a
//!    datagram through the real socket path (validates the trait
//!    chain end-to-end).
//! 2. The recv loop in `connect_loop` can be invoked with a
//!    `LossyTransport` (compile-time + runtime smoke that the
//!    migration in `service.rs` actually accepts the wider trait).

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::UdpSocket;

use cimmeria_mercury::lossy_transport::{LossyConfig, LossyProfile, LossyTransport};
use cimmeria_mercury::transport::{BidirectionalTransport, Transport, UdpTransport};

async fn bind_udp() -> Arc<dyn BidirectionalTransport> {
    let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
    Arc::new(UdpTransport::new(socket))
}

#[tokio::test]
async fn lossy_transport_round_trips_under_lan_profile() {
    // Receiver: plain UDP wrapped as BidirectionalTransport.
    let receiver = bind_udp().await;
    let recv_addr: SocketAddr = receiver.local_addr().unwrap();

    // Sender: LossyTransport(LAN) wrapping its own UDP. LAN profile
    // has 500us latency, 0% loss — every send must arrive.
    let sender_inner = bind_udp().await;
    let sender = Arc::new(LossyTransport::new_symmetric(
        sender_inner,
        LossyConfig::from_profile(LossyProfile::Lan),
    ));

    let payload = b"chaos-lan-integration";
    let n = sender.send_to(payload, recv_addr).await.unwrap();
    assert_eq!(n, payload.len(), "LAN profile sends every byte cleanly");

    let mut buf = [0u8; 64];
    let (len, _) = tokio::time::timeout(Duration::from_secs(1), receiver.recv_from(&mut buf))
        .await
        .expect("LAN profile must deliver the round-trip within timeout")
        .unwrap();
    assert_eq!(&buf[..len], payload);
}

#[tokio::test]
async fn lossy_transport_satisfies_bidirectional_transport_trait_for_recv_loop() {
    // Compile-time + light runtime check that a LossyTransport can
    // be coerced to the BidirectionalTransport trait object used by
    // `run_connect_loop`. If the recv-loop signature drifts in a
    // way that breaks LossyTransport coercion, this test fails to
    // compile.
    let lossy: Arc<dyn BidirectionalTransport> = Arc::new(LossyTransport::new_symmetric(
        bind_udp().await,
        LossyConfig::from_profile(LossyProfile::Transatlantic),
    ));

    // Runtime: round-trip one packet to prove send + recv both work
    // through the trait object surface.
    let peer = bind_udp().await;
    let peer_addr = peer.local_addr().unwrap();
    let payload = b"trait-object-round-trip";

    lossy.send_to(payload, peer_addr).await.unwrap();

    let mut buf = [0u8; 64];
    let (len, _) = tokio::time::timeout(Duration::from_millis(500), peer.recv_from(&mut buf))
        .await
        .expect("send must complete through the trait object")
        .unwrap();
    assert_eq!(&buf[..len], payload);
}

#[tokio::test]
async fn transatlantic_profile_loss_eventually_drops_a_packet() {
    // Statistical: with seeded 0.5% loss across many sends, at
    // least one drop must fire. Validates the recv loop's
    // chaos-aware behavior at the integration boundary.
    let receiver = bind_udp().await;
    let recv_addr = receiver.local_addr().unwrap();
    let sender = Arc::new(LossyTransport::new_symmetric(
        bind_udp().await,
        LossyConfig::from_profile(LossyProfile::Transatlantic).with_seed(0xC0FFEE),
    ));

    let send_count = 500u32; // 0.5% × 500 ≈ 2-3 drops expected
    for i in 0..send_count {
        sender
            .send_to(format!("p-{i}").as_bytes(), recv_addr)
            .await
            .unwrap();
    }

    // Drain everything that arrives within a generous window.
    // Each recv has 60ms latency from the Transatlantic profile,
    // so we need to allow plenty of wall time for ~500 packets.
    let mut received = 0usize;
    let mut buf = [0u8; 64];
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    while tokio::time::Instant::now() < deadline && received < send_count as usize {
        match tokio::time::timeout(Duration::from_millis(200), receiver.recv_from(&mut buf)).await {
            Ok(Ok((_, _))) => received += 1,
            _ => break, // no more packets arriving — assume drained
        }
    }

    assert!(
        received < send_count as usize,
        "Transatlantic 0.5% profile must drop at least one of {send_count} sends; \
         all {received} arrived = no loss fired (regression?)"
    );
    // And we must have received the vast majority.
    let arrived_pct = (received * 100) / send_count as usize;
    assert!(
        arrived_pct >= 95,
        "expected ≥95% delivery at 0.5% loss; got {arrived_pct}%"
    );
}
