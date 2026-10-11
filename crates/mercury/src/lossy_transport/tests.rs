//! Unit tests for [super::LossyTransport] on real loopback sockets.

use super::*;
use crate::transport::UdpTransport;
use tokio::net::UdpSocket;

fn make_inner(port: u16) -> Arc<dyn BidirectionalTransport> {
    let socket = std::net::UdpSocket::bind(format!("127.0.0.1:{port}")).unwrap();
    socket.set_nonblocking(true).unwrap();
    let tokio_socket = UdpSocket::from_std(socket).unwrap();
    Arc::new(UdpTransport::new(Arc::new(tokio_socket)))
}

#[tokio::test]
async fn lossy_lan_profile_round_trips_a_datagram() {
    let receiver = make_inner(0);
    let recv_addr = receiver.local_addr().unwrap();

    let sender_inner = make_inner(0);
    let lossy =
        LossyTransport::new_symmetric(sender_inner, LossyConfig::from_profile(LossyProfile::Lan));

    let payload = b"lossy-lan";
    let n = lossy.send_to(payload, recv_addr).await.unwrap();
    assert_eq!(n, payload.len());

    let mut buf = [0u8; 64];
    let (len, _) = receiver.recv_from(&mut buf).await.unwrap();
    assert_eq!(&buf[..len], payload);
}

#[tokio::test]
async fn recv_side_drop_loops_until_a_packet_passes() {
    // Inner receiver: real UDP socket. Wrapped in LossyTransport
    // that drops 50% of receives.
    let inner = make_inner(0);
    let recv_addr = inner.local_addr().unwrap();
    let lossy = LossyTransport::new_symmetric(
        inner,
        LossyConfig {
            latency: Duration::ZERO,
            jitter: Duration::ZERO,
            drop_per_thousand: 500, // 50%
            duplicate_per_thousand: 0,
            reorder_buffer_size: 0,
            lossless_first_sends: 0,
            rng_seed: 42,
        },
    );

    // Sender: plain UDP socket; we want the chaos to act only on
    // the recv side of the lossy transport.
    let sender_sock = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    sender_sock.set_nonblocking(true).unwrap();
    let sender = UdpSocket::from_std(sender_sock).unwrap();

    // Send 20 packets in a tight loop.
    for i in 0..20u32 {
        sender
            .send_to(format!("p-{i}").as_bytes(), recv_addr)
            .await
            .unwrap();
    }

    // The lossy recv loops past drops and returns the first
    // surviving packet. With seeded 50% loss over 20 packets,
    // at least one will get through.
    let mut buf = [0u8; 64];
    let result = tokio::time::timeout(Duration::from_secs(2), lossy.recv_from(&mut buf))
        .await
        .expect("recv must yield a non-dropped packet within timeout");
    let (n, _) = result.unwrap();
    assert!(n > 0, "delivered packet must have content");
}

#[tokio::test]
async fn drop_100pct_never_delivers() {
    let receiver = make_inner(0);
    let recv_addr = receiver.local_addr().unwrap();

    let sender_inner = make_inner(0);
    let cfg = LossyConfig::new(Duration::ZERO, 1000, 0, 1);
    let lossy = LossyTransport::new_symmetric(sender_inner, cfg);

    for i in 0..10 {
        let _ = lossy.send_to(format!("p-{i}").as_bytes(), recv_addr).await;
    }

    let mut buf = [0u8; 64];
    let timed_out = tokio::time::timeout(Duration::from_millis(100), receiver.recv_from(&mut buf))
        .await
        .is_err();
    assert!(timed_out, "100% drop must deliver nothing");
}

#[tokio::test]
async fn recv_100pct_drop_returns_err_not_infinite_loop() {
    // Sender: plain UDP — wire delivers every packet. Receiver:
    // wrapped in 100% recv-drop LossyTransport.
    let inner = make_inner(0);
    let recv_addr = inner.local_addr().unwrap();
    let lossy = LossyTransport::new_symmetric(inner, LossyConfig::new(Duration::ZERO, 1000, 0, 0));

    let sender_sock = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    sender_sock.set_nonblocking(true).unwrap();
    let sender = UdpSocket::from_std(sender_sock).unwrap();

    // Stream more than MAX_RECV_DROP_LOOPS packets so the recv
    // loop has plenty of arrivals to chew on.
    for i in 0..(MAX_RECV_DROP_LOOPS + 10) {
        sender
            .send_to(format!("p-{i}").as_bytes(), recv_addr)
            .await
            .unwrap();
    }

    let mut buf = [0u8; 64];
    let err = tokio::time::timeout(Duration::from_secs(3), lossy.recv_from(&mut buf))
        .await
        .expect("recv must return Err, not infinite-loop until timeout")
        .expect_err("100% drop must yield an error after MAX_RECV_DROP_LOOPS retries");
    let msg = err.to_string();
    assert!(
        msg.contains("dropped") && msg.contains("consecutive"),
        "error must explain the cause; got: {msg}"
    );
}

#[test]
fn lossy_config_clamps_per_thousand_above_1000() {
    let cfg = LossyConfig::new(Duration::ZERO, 2000, 9999, 0);
    assert_eq!(cfg.drop_per_thousand, MAX_PER_THOUSAND);
    assert_eq!(cfg.duplicate_per_thousand, MAX_PER_THOUSAND);
}

#[tokio::test]
async fn asymmetric_config_drops_only_one_direction() {
    // Lomiada shape: outbound (server→client) fine, inbound
    // (client→server acks) dropped.
    let inner = make_inner(0);
    let lossy = LossyTransport::new_asymmetric(
        inner,
        LossyConfig::new(Duration::ZERO, 0, 0, 1), // send: lossless
        LossyConfig::new(Duration::ZERO, 1000, 0, 1), // recv: all-drop
    );
    let peer_inner = make_inner(0);
    let peer_addr = peer_inner.local_addr().unwrap();
    let our_addr = lossy.local_addr().unwrap();

    // Send 5 packets in our direction — every one arrives at peer.
    for i in 0..5u32 {
        lossy
            .send_to(format!("out-{i}").as_bytes(), peer_addr)
            .await
            .unwrap();
    }

    // The peer observed every send.
    let mut buf = [0u8; 64];
    let mut received = 0;
    for _ in 0..5 {
        match tokio::time::timeout(Duration::from_millis(200), peer_inner.recv_from(&mut buf)).await
        {
            Ok(Ok(_)) => received += 1,
            _ => break,
        }
    }
    assert_eq!(received, 5, "outbound direction is lossless");

    // The peer sends back. Recv-side 100% drop should make us
    // never observe them. Stream more than MAX_RECV_DROP_LOOPS so
    // the loop hits the guard and errors.
    let peer_sender = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    peer_sender.set_nonblocking(true).unwrap();
    let peer_sender = UdpSocket::from_std(peer_sender).unwrap();
    for i in 0..(MAX_RECV_DROP_LOOPS + 10) {
        peer_sender
            .send_to(format!("in-{i}").as_bytes(), our_addr)
            .await
            .unwrap();
    }
    let result = tokio::time::timeout(Duration::from_secs(3), lossy.recv_from(&mut buf)).await;
    assert!(
        matches!(result, Ok(Err(_))),
        "100% recv-side drop must yield Err (asymmetric drop direction working)"
    );
}

#[test]
fn with_jitter_and_with_reorder_buffer_are_builders() {
    let cfg = LossyConfig::new(Duration::ZERO, 0, 0, 0)
        .with_jitter(Duration::from_millis(10))
        .with_reorder_buffer(3);
    assert_eq!(cfg.jitter, Duration::from_millis(10));
    assert_eq!(cfg.reorder_buffer_size, 3);
    // Defaults from `new` alone are zero -- no jitter, no reorder.
    let plain = LossyConfig::new(Duration::ZERO, 0, 0, 0);
    assert_eq!(plain.jitter, Duration::ZERO);
    assert_eq!(plain.reorder_buffer_size, 0);
}

#[tokio::test]
async fn drop_next_sends_forces_exact_count_then_resumes() {
    // Zero probabilistic loss so only the deterministic counter can
    // cause a drop.
    let receiver = make_inner(0);
    let recv_addr = receiver.local_addr().unwrap();
    let sender_inner = make_inner(0);
    let lossy =
        LossyTransport::new_symmetric(sender_inner, LossyConfig::new(Duration::ZERO, 0, 0, 0));

    lossy.drop_next_sends(3);

    for i in 0..3u32 {
        let n = lossy
            .send_to(format!("dropped-{i}").as_bytes(), recv_addr)
            .await
            .unwrap();
        assert!(
            n > 0,
            "forced-drop send still reports the kernel's-view byte count"
        );
    }
    // The 4th send must land -- the counter is exhausted.
    lossy.send_to(b"survivor", recv_addr).await.unwrap();

    let mut buf = [0u8; 64];
    let (len, _) = tokio::time::timeout(Duration::from_millis(500), receiver.recv_from(&mut buf))
        .await
        .expect("the 4th send must be delivered")
        .unwrap();
    assert_eq!(&buf[..len], b"survivor");

    // Nothing else arrives -- the 3 forced drops never hit the wire.
    let timed_out = tokio::time::timeout(Duration::from_millis(100), receiver.recv_from(&mut buf))
        .await
        .is_err();
    assert!(
        timed_out,
        "exactly 3 sends must have been dropped, not fewer"
    );
}

/// The wire client reads the two phase-3 handshake datagrams
/// positionally, so a loss scenario must let them through or the
/// connect itself fails (#1371).
#[tokio::test]
async fn lossless_first_sends_lets_each_destinations_handshake_through() {
    let receiver_a = make_inner(0);
    let addr_a = receiver_a.local_addr().unwrap();
    let receiver_b = make_inner(0);
    let addr_b = receiver_b.local_addr().unwrap();
    let lossy = LossyTransport::new_symmetric(
        make_inner(0),
        LossyConfig::new(Duration::ZERO, MAX_PER_THOUSAND, 0, 7).with_lossless_first_sends(2),
    );

    for (receiver, addr) in [(&receiver_a, addr_a), (&receiver_b, addr_b)] {
        for payload in [
            b"first".as_slice(),
            b"second".as_slice(),
            b"third".as_slice(),
        ] {
            lossy.send_to(payload, addr).await.unwrap();
        }
        let mut buf = [0u8; 64];
        for expected in [b"first".as_slice(), b"second".as_slice()] {
            let (len, _) =
                tokio::time::timeout(Duration::from_millis(500), receiver.recv_from(&mut buf))
                    .await
                    .expect("the first two sends to each destination skip the drop")
                    .unwrap();
            assert_eq!(&buf[..len], expected);
        }
        let third =
            tokio::time::timeout(Duration::from_millis(200), receiver.recv_from(&mut buf)).await;
        assert!(third.is_err(), "the third send is past the grace and drops");
    }
}

#[tokio::test]
async fn drop_next_sends_to_only_affects_the_targeted_destination() {
    // Two distinct receivers -- the targeted drop must hit only the
    // one it names, leaving traffic to the other receiver untouched
    // even when sends to both are interleaved.
    let receiver_a = make_inner(0);
    let addr_a = receiver_a.local_addr().unwrap();
    let receiver_b = make_inner(0);
    let addr_b = receiver_b.local_addr().unwrap();
    let sender_inner = make_inner(0);
    let lossy =
        LossyTransport::new_symmetric(sender_inner, LossyConfig::new(Duration::ZERO, 0, 0, 0));

    lossy.drop_next_sends_to(1, addr_a, 0);

    // Interleave: to B first (must land), then to A (must drop), then
    // to B again (must land) -- proves the filter doesn't just drop
    // "whatever's next" globally.
    lossy.send_to(b"to-b-1", addr_b).await.unwrap();
    lossy.send_to(b"to-a-dropped", addr_a).await.unwrap();
    lossy.send_to(b"to-b-2", addr_b).await.unwrap();

    let mut buf = [0u8; 64];
    for expected in [b"to-b-1".as_slice(), b"to-b-2".as_slice()] {
        let (len, _) =
            tokio::time::timeout(Duration::from_millis(500), receiver_b.recv_from(&mut buf))
                .await
                .expect("sends to B must be unaffected by the A-targeted drop")
                .unwrap();
        assert_eq!(&buf[..len], expected);
    }
    let timed_out =
        tokio::time::timeout(Duration::from_millis(100), receiver_a.recv_from(&mut buf))
            .await
            .is_err();
    assert!(timed_out, "the targeted send to A must have been dropped");
}

#[tokio::test]
async fn drop_next_sends_to_min_len_skips_smaller_sends() {
    // A `min_len` filter must let a short "keepalive"-shaped send pass
    // through untouched and instead wait for a send at least that
    // long, still to the same address -- this is what lets a test
    // target "the next *substantial* packet to this witness" without
    // an interleaved small periodic send (tickSync, in the real
    // server) spending the armed drop's budget first.
    let receiver = make_inner(0);
    let recv_addr = receiver.local_addr().unwrap();
    let sender_inner = make_inner(0);
    let lossy =
        LossyTransport::new_symmetric(sender_inner, LossyConfig::new(Duration::ZERO, 0, 0, 0));

    lossy.drop_next_sends_to(1, recv_addr, 50);

    lossy.send_to(b"short", recv_addr).await.unwrap(); // 5 bytes, under min_len -- must land
    lossy.send_to(&[b'x'; 64], recv_addr).await.unwrap(); // 64 bytes, at/over min_len -- must drop
    lossy.send_to(b"survivor", recv_addr).await.unwrap(); // budget exhausted -- must land

    let mut buf = [0u8; 128];
    let (len, _) = tokio::time::timeout(Duration::from_millis(500), receiver.recv_from(&mut buf))
        .await
        .expect("the short send must pass the size filter and land")
        .unwrap();
    assert_eq!(&buf[..len], b"short");

    let (len, _) = tokio::time::timeout(Duration::from_millis(500), receiver.recv_from(&mut buf))
        .await
        .expect("the survivor send must land once the budget is exhausted")
        .unwrap();
    assert_eq!(&buf[..len], b"survivor");

    // The 64-byte send never arrives -- it was the one dropped.
    let timed_out = tokio::time::timeout(Duration::from_millis(100), receiver.recv_from(&mut buf))
        .await
        .is_err();
    assert!(timed_out, "the 64-byte send must have been the one dropped");
}

#[tokio::test]
async fn reorder_buffer_flushes_in_reverse_arrival_order() {
    let receiver = make_inner(0);
    let recv_addr = receiver.local_addr().unwrap();
    let sender_inner = make_inner(0);
    let cfg = LossyConfig::new(Duration::ZERO, 0, 0, 0).with_reorder_buffer(3);
    let lossy = LossyTransport::new_symmetric(sender_inner, cfg);

    // Nothing is on the wire yet -- all 3 are held until the buffer fills.
    lossy.send_to(b"first", recv_addr).await.unwrap();
    lossy.send_to(b"second", recv_addr).await.unwrap();
    let mut buf = [0u8; 64];
    assert!(
        tokio::time::timeout(Duration::from_millis(50), receiver.recv_from(&mut buf))
            .await
            .is_err(),
        "buffer isn't full yet -- nothing should have been sent"
    );

    // The 3rd send fills the buffer and flushes all 3, reversed:
    // arrival order was first, second, third -> wire order third,
    // second, first.
    lossy.send_to(b"third", recv_addr).await.unwrap();

    let mut order = Vec::new();
    for _ in 0..3 {
        let (len, _) =
            tokio::time::timeout(Duration::from_millis(500), receiver.recv_from(&mut buf))
                .await
                .expect("held packets must flush once the buffer fills")
                .unwrap();
        order.push(String::from_utf8_lossy(&buf[..len]).to_string());
    }
    assert_eq!(order, vec!["third", "second", "first"]);
}

#[tokio::test]
async fn flush_reorder_buffer_drains_a_partial_buffer() {
    let receiver = make_inner(0);
    let recv_addr = receiver.local_addr().unwrap();
    let sender_inner = make_inner(0);
    // Buffer size 5, but we only ever send 2 -- without an explicit
    // flush these would never reach the wire.
    let cfg = LossyConfig::new(Duration::ZERO, 0, 0, 0).with_reorder_buffer(5);
    let lossy = LossyTransport::new_symmetric(sender_inner, cfg);

    lossy.send_to(b"a", recv_addr).await.unwrap();
    lossy.send_to(b"b", recv_addr).await.unwrap();
    lossy.flush_reorder_buffer().await.unwrap();

    let mut buf = [0u8; 64];
    let mut order = Vec::new();
    for _ in 0..2 {
        let (len, _) =
            tokio::time::timeout(Duration::from_millis(500), receiver.recv_from(&mut buf))
                .await
                .expect("explicit flush must deliver the partial buffer")
                .unwrap();
        order.push(String::from_utf8_lossy(&buf[..len]).to_string());
    }
    assert_eq!(
        order,
        vec!["b", "a"],
        "explicit flush also reverses arrival order"
    );
}

#[tokio::test]
async fn jitter_widens_delivery_latency_within_bounds() {
    // Deterministic seed -- pin that the observed latency never
    // exceeds latency + jitter, and that jitter actually moves the
    // needle (not always the minimum).
    let receiver = make_inner(0);
    let recv_addr = receiver.local_addr().unwrap();
    let sender_inner = make_inner(0);
    let base = Duration::from_millis(5);
    let jitter = Duration::from_millis(20);
    let lossy = LossyTransport::new_symmetric(sender_inner, cfg_for_replay(base, jitter));

    // Measure arrival, not how long `send_to` blocked: a send returns at
    // once and the delay line delivers the datagram later (#1371).
    let mut buf = [0u8; 64];
    let mut latencies = Vec::new();
    for i in 0..8u32 {
        let start = std::time::Instant::now();
        lossy
            .send_to(format!("j-{i}").as_bytes(), recv_addr)
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(2), receiver.recv_from(&mut buf))
            .await
            .expect("datagram must arrive")
            .unwrap();
        latencies.push(start.elapsed());
    }

    for lat in &latencies {
        assert!(
            *lat >= base,
            "observed latency {lat:?} must be at least the base latency {base:?}"
        );
        assert!(
            // Generous scheduling slack -- this asserts the jitter
            // computation is bounded, not tight real-time delivery;
            // a loaded CI/dev box can add tens of ms of scheduler
            // noise on top of the sleep itself.
            *lat <= base + jitter + Duration::from_millis(200),
            "observed latency {lat:?} must not exceed base+jitter by more than scheduling slack"
        );
    }
    // With no drop or duplicate configured, the send RNG draws only the
    // eight latencies, so replaying seed 7 gives each packet's floor. A
    // lower bound load cannot break, and it fails if jitter is ignored.
    let mut rng = ChaCha20Rng::seed_from_u64(7);
    let expected: Vec<Duration> = (0..8)
        .map(|_| filter::roll_latency(&cfg_for_replay(base, jitter), &mut rng))
        .collect();
    assert!(
        expected.iter().any(|d| *d > base),
        "seed 7 must draw some jitter, or this test checks nothing: {expected:?}"
    );
    for (lat, exp) in latencies.iter().zip(&expected) {
        assert!(
            *lat >= *exp,
            "packet arrived after {lat:?}, before its drawn latency {exp:?}"
        );
    }
}

fn cfg_for_replay(base: Duration, jitter: Duration) -> LossyConfig {
    LossyConfig::new(base, 0, 0, 7).with_jitter(jitter)
}

/// The grace overrides the drop outcome but draws every roll as before,
/// so the drop pattern after the grace window matches a run without it.
/// Fails if the grace short-circuits the roll (`!in_grace && should_drop`)
/// or changes which rolls are drawn.
#[tokio::test]
async fn lossless_first_sends_keeps_the_drop_pattern_after_the_grace() {
    async fn pattern(grace: u32) -> Vec<bool> {
        let receiver = make_inner(0);
        let addr = receiver.local_addr().unwrap();
        let lossy = LossyTransport::new_symmetric(
            make_inner(0),
            LossyConfig::new(Duration::ZERO, 500, 200, 11).with_lossless_first_sends(grace),
        );
        for i in 0..40u8 {
            lossy.send_to(&[i], addr).await.unwrap();
        }
        let mut seen = vec![false; 40];
        let mut buf = [0u8; 8];
        while let Ok(Ok((n, _))) =
            tokio::time::timeout(Duration::from_millis(200), receiver.recv_from(&mut buf)).await
        {
            if n == 1 {
                seen[buf[0] as usize] = true;
            }
        }
        seen
    }
    let with_grace = pattern(2).await;
    let without = pattern(0).await;
    assert!(
        with_grace[0] && with_grace[1],
        "the grace lets sends 0 and 1 through"
    );
    assert_eq!(
        with_grace[2..],
        without[2..],
        "sends after the grace must drop exactly as without it"
    );
    assert!(
        without[2..].iter().any(|s| !s),
        "seed 11 must drop something after the grace, or this test checks nothing"
    );
}

/// Regression guard for #1371: the old `send_to` slept the latency
/// inline, so ten sends took ten latencies and a caller serving many
/// peers was capped at one send per latency.
#[tokio::test]
async fn latency_is_a_pipeline_delay_not_a_send_cap() {
    let receiver = make_inner(0);
    let recv_addr = receiver.local_addr().unwrap();
    let cfg = LossyConfig::new(Duration::from_millis(200), 0, 0, 1);
    let lossy = LossyTransport::new_symmetric(make_inner(0), cfg);

    let sent = std::time::Instant::now();
    for i in 0..10u8 {
        lossy.send_to(&[i], recv_addr).await.unwrap();
    }
    // The old inline sleep makes ten sends take at least 2 s, so a 1 s
    // bound still catches it while leaving room for a loaded runner.
    assert!(
        sent.elapsed() < Duration::from_secs(1),
        "a send must not wait out the latency (#1371); ten sends took {:?}",
        sent.elapsed()
    );

    let mut buf = [0u8; 16];
    let mut first_arrival = None;
    for i in 0..10u8 {
        let (n, _) = tokio::time::timeout(Duration::from_secs(2), receiver.recv_from(&mut buf))
            .await
            .expect("datagram must arrive")
            .unwrap();
        assert_eq!(&buf[..n], &[i], "datagrams must arrive in send order");
        first_arrival.get_or_insert_with(std::time::Instant::now);
    }
    let first_arrival = first_arrival.expect("ten datagrams were received");
    assert!(
        first_arrival.duration_since(sent) >= Duration::from_millis(200),
        "the first datagram must not arrive before the latency"
    );
    // A line that slept the latency per item would take at least 2 s.
    assert!(
        sent.elapsed() < Duration::from_millis(1500),
        "all ten datagrams must be in within 1.5 s of the first send, took {:?}",
        sent.elapsed()
    );
}

/// Regression guard for #1371 on the recv side: the old `recv_from`
/// slept the latency after every datagram, so a backlog of ten took ten
/// latencies to read.
#[tokio::test]
async fn recv_latency_is_a_pipeline_delay_not_a_read_cap() {
    let inner = make_inner(0);
    let recv_addr = inner.local_addr().unwrap();
    let cfg = LossyConfig::new(Duration::from_millis(200), 0, 0, 1);
    let lossy = LossyTransport::new_symmetric(inner, cfg);

    let sender = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    for i in 0..10u8 {
        sender.send_to(&[i], recv_addr).await.unwrap();
    }

    let started = std::time::Instant::now();
    let mut buf = [0u8; 16];
    for i in 0..10u8 {
        let (n, _) = tokio::time::timeout(Duration::from_secs(2), lossy.recv_from(&mut buf))
            .await
            .expect("datagram must arrive")
            .unwrap();
        assert_eq!(&buf[..n], &[i], "datagrams must be read in arrival order");
    }
    let took = started.elapsed();
    assert!(
        took >= Duration::from_millis(200) && took < Duration::from_millis(1500),
        "ten backlogged reads must take one latency, not one each; took {took:?}"
    );
}

/// Pins what the phase-3 handshake needs: the reply and the time-sync
/// are read positionally, so one destination's datagrams must not
/// reorder under jitter. Both the old inline-sleep code and the delay
/// line pass it (#1371); the delay line keeps the order by design.
#[tokio::test]
async fn jitter_never_reorders_one_destination() {
    let receiver = make_inner(0);
    let recv_addr = receiver.local_addr().unwrap();
    let cfg =
        LossyConfig::new(Duration::from_millis(1), 0, 0, 9).with_jitter(Duration::from_millis(40));
    let lossy = LossyTransport::new_symmetric(make_inner(0), cfg);

    for i in 0..30u8 {
        lossy.send_to(&[i], recv_addr).await.unwrap();
    }

    let mut buf = [0u8; 16];
    for i in 0..30u8 {
        let (n, _) = tokio::time::timeout(Duration::from_secs(2), receiver.recv_from(&mut buf))
            .await
            .expect("datagram must arrive")
            .unwrap();
        assert_eq!(&buf[..n], &[i], "jitter must not reorder one destination");
    }
}

#[tokio::test]
async fn recv_100pct_drop_with_latency_returns_err() {
    // Same as `recv_100pct_drop_returns_err_not_infinite_loop`, but with
    // latency, so the drop roll runs in the recv pump and the cap error
    // has to come out of the pump's channel (#1371).
    let inner = make_inner(0);
    let recv_addr = inner.local_addr().unwrap();
    let lossy = LossyTransport::new_symmetric(
        inner,
        LossyConfig::new(Duration::from_millis(5), 1000, 0, 0),
    );

    let sender = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    for i in 0..(MAX_RECV_DROP_LOOPS + 10) {
        sender
            .send_to(format!("p-{i}").as_bytes(), recv_addr)
            .await
            .unwrap();
    }

    let mut buf = [0u8; 64];
    let err = tokio::time::timeout(Duration::from_secs(3), lossy.recv_from(&mut buf))
        .await
        .expect("recv must return Err, not infinite-loop until timeout")
        .expect_err("100% drop must yield an error after MAX_RECV_DROP_LOOPS retries");
    let msg = err.to_string();
    assert!(
        msg.contains("dropped") && msg.contains("consecutive"),
        "error must explain the cause; got: {msg}"
    );
}

/// A [`filter::FlowFilter`]'s send pattern is its seed's alone: rolling
/// receives in between does not shift it, so a replayed run drops the same
/// sends however much the peer sent. Fails if both directions share one RNG.
#[test]
fn flow_filter_send_rolls_ignore_recv_rolls() {
    let config = LossyConfig::new(Duration::ZERO, 400, 0, 11);
    let mut quiet = filter::FlowFilter::new(config.clone());
    let mut chatty = filter::FlowFilter::new(config);
    let plain: Vec<bool> = (0..64).map(|_| quiet.roll_send().drop).collect();
    let mixed: Vec<bool> = (0..64)
        .map(|_| {
            chatty.roll_recv_drop();
            chatty.roll_send().drop
        })
        .collect();
    assert_eq!(plain, mixed);
    assert!(plain.contains(&true) && plain.contains(&false));
}
