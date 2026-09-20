//! The connection caps — global and per peer IP: who gets admitted past
//! accept, that a slot comes back when its connection ends, and that a
//! flood warns once.

use super::*;

/// Start [`serve`] on a loopback port with the given caps. The handshake
/// deadline is generous so it never fires inside a cap test — the caps are
/// the only thing that may close a socket here.
async fn spawn_capped_server(max_connections: usize, max_per_ip: usize) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let addr = listener.local_addr().expect("local_addr");
    let (tx, _rx) = mpsc::channel(16);
    tokio::spawn(serve(
        listener,
        SessionRegistry::new(),
        tx,
        9339,
        ConnectionLimits {
            max_connections,
            max_per_ip,
            handshake_timeout: Duration::from_secs(30),
        },
    ));
    addr
}

/// Past the cap, a new connection is closed straight away instead of
/// getting a task and a buffer. The connections already admitted are
/// untouched and still served.
#[tokio::test]
async fn a_connection_over_the_cap_is_closed_while_admitted_ones_are_served() {
    let addr = spawn_capped_server(2, 100).await;

    let mut first = TcpStream::connect(addr).await.expect("connect first");
    let mut second = TcpStream::connect(addr).await.expect("connect second");
    // Completing verChk on both proves they hold the two slots before the
    // third arrives, so which socket gets refused is not a race.
    assert!(
        completes_version_check(&mut first).await,
        "first must be served"
    );
    assert!(
        completes_version_check(&mut second).await,
        "second must be served"
    );

    let mut third = TcpStream::connect(addr).await.expect("connect third");
    assert!(
        server_hangs_up(&mut third, OBSERVE_WITHIN).await,
        "a connection beyond max_connections must be closed on accept",
    );

    // The refusal must not have disturbed the admitted sockets: both are
    // still open, parked in the login read.
    for (name, admitted) in [("first", &mut first), ("second", &mut second)] {
        assert!(
            !server_hangs_up(admitted, SHORT_DEADLINE).await,
            "{name} was admitted and must stay open after a later refusal",
        );
    }
}

/// A slot is released when its connection ends. Without that, every
/// connection that ever finished would permanently shrink the cap, and the
/// port would stop admitting anyone after `max_connections` sessions.
#[tokio::test]
async fn a_slot_frees_when_an_admitted_connection_ends() {
    let addr = spawn_capped_server(1, 100).await;

    let mut first = TcpStream::connect(addr).await.expect("connect first");
    assert!(
        completes_version_check(&mut first).await,
        "first must be served"
    );
    drop(first);

    assert!(
        admits_again(addr, "127.0.0.1").await,
        "after the only admitted connection closed, a new one must be served",
    );
}

/// Hold one admitted socket, flood four more from the same IP, and count
/// the WARN events carrying `reason`.
async fn warnings_for_a_flood(addr: std::net::SocketAddr, reason: &str) -> usize {
    let capture = crate::test_support::LogCapture::install();
    let mut admitted = connect_from("127.0.0.1", addr).await;
    assert!(
        completes_version_check(&mut admitted).await,
        "must be served"
    );

    for _ in 0..4 {
        let mut refused = connect_from("127.0.0.1", addr).await;
        assert!(
            server_hangs_up(&mut refused, OBSERVE_WITHIN).await,
            "every connection past the cap must be closed",
        );
    }

    capture
        .all()
        .into_iter()
        .filter(|e| e.level == tracing::Level::WARN && e.has_field("reason", reason))
        .count()
}

/// A flood past the cap must warn once per interval with a refused count,
/// not once per socket — WARN events post to Discord.
#[tokio::test]
async fn a_flood_past_the_cap_warns_once_not_per_connection() {
    let addr = spawn_capped_server(1, 100).await;
    assert_eq!(
        warnings_for_a_flood(addr, "connection_cap").await,
        1,
        "four refusals inside one warn interval must produce exactly one WARN",
    );
}

/// Connect to `addr` from a chosen loopback source address, so one test can
/// play several peers. The whole of 127.0.0.0/8 is loopback.
async fn connect_from(source: &str, addr: std::net::SocketAddr) -> TcpStream {
    let socket = tokio::net::TcpSocket::new_v4().expect("create socket");
    socket
        .bind(format!("{source}:0").parse().expect("source address"))
        .expect("bind loopback source address");
    socket.connect(addr).await.expect("connect loopback")
}

/// Retry connecting from `source` until a socket is served. A slot comes
/// back asynchronously — the server notices a close on its next read — so
/// this polls, and returns `false` only if no socket is served in time.
async fn admits_again(addr: std::net::SocketAddr, source: &str) -> bool {
    tokio::time::timeout(OBSERVE_WITHIN, async {
        loop {
            let mut next = connect_from(source, addr).await;
            if completes_version_check(&mut next).await {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .is_ok()
}

/// One host may not hold more than `max_per_ip` sockets. Without this
/// limit, a single host reconnecting every few seconds can fill the global
/// cap alone. Every SWF refused at the cap then strands its player's
/// pending session until the TTL expires. Other hosts are still admitted
/// while one sits at its limit.
#[tokio::test]
async fn a_connection_over_the_per_ip_cap_is_closed_while_other_ips_are_served() {
    let addr = spawn_capped_server(10, 2).await;

    let mut first = connect_from("127.0.0.1", addr).await;
    let mut second = connect_from("127.0.0.1", addr).await;
    assert!(
        completes_version_check(&mut first).await,
        "first must be served"
    );
    assert!(
        completes_version_check(&mut second).await,
        "second must be served"
    );

    let mut third = connect_from("127.0.0.1", addr).await;
    assert!(
        server_hangs_up(&mut third, OBSERVE_WITHIN).await,
        "a third socket from one IP must be closed at max_per_ip = 2",
    );

    let mut other_host = connect_from("127.0.0.2", addr).await;
    assert!(
        completes_version_check(&mut other_host).await,
        "another IP must still be admitted while the first sits at its per-IP cap",
    );
}

/// A per-IP slot is released when its connection ends, or a host that
/// reconnects `max_per_ip` times is locked out for good.
#[tokio::test]
async fn a_per_ip_slot_frees_when_its_connection_ends() {
    let addr = spawn_capped_server(10, 1).await;

    let mut first = connect_from("127.0.0.1", addr).await;
    assert!(
        completes_version_check(&mut first).await,
        "first must be served"
    );
    drop(first);

    assert!(
        admits_again(addr, "127.0.0.1").await,
        "after a host's only connection closed, its next one must be served",
    );
}

/// A flood from one host past its per-IP cap warns once per interval, like
/// the global cap, not once per socket.
#[tokio::test]
async fn a_flood_past_the_per_ip_cap_warns_once_not_per_connection() {
    let addr = spawn_capped_server(100, 1).await;
    assert_eq!(
        warnings_for_a_flood(addr, "per_ip_cap").await,
        1,
        "four per-IP refusals inside one warn interval must produce exactly one WARN",
    );
}
