//! The connection cap: who gets admitted past accept, that a slot comes
//! back when its connection ends, and that a flood warns once.

use super::*;

/// Start [`serve`] on a loopback port with the given cap. The handshake
/// deadline is generous so it never fires inside a cap test — the cap is
/// the only thing that may close a socket here.
async fn spawn_capped_server(max_connections: usize) -> std::net::SocketAddr {
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
    let addr = spawn_capped_server(2).await;

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
    let addr = spawn_capped_server(1).await;

    let mut first = TcpStream::connect(addr).await.expect("connect first");
    assert!(
        completes_version_check(&mut first).await,
        "first must be served"
    );
    drop(first);

    // The server notices the close on its next read, so the slot comes
    // back asynchronously: retry until a new socket is served, and fail
    // only if that never happens.
    let admitted = tokio::time::timeout(OBSERVE_WITHIN, async {
        loop {
            let mut next = TcpStream::connect(addr).await.expect("connect next");
            if completes_version_check(&mut next).await {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    assert!(
        admitted.is_ok(),
        "after the only admitted connection closed, a new one must be served",
    );
}

/// A flood past the cap must warn once per interval with a refused count,
/// not once per socket — WARN events post to Discord.
#[tokio::test]
async fn a_flood_past_the_cap_warns_once_not_per_connection() {
    let capture = crate::test_support::LogCapture::install();
    let addr = spawn_capped_server(1).await;
    let mut admitted = TcpStream::connect(addr).await.expect("connect admitted");
    assert!(
        completes_version_check(&mut admitted).await,
        "must be served"
    );

    for _ in 0..4 {
        let mut refused = TcpStream::connect(addr).await.expect("connect refused");
        assert!(
            server_hangs_up(&mut refused, OBSERVE_WITHIN).await,
            "every connection past the cap must be closed",
        );
    }

    let cap_warnings = capture
        .all()
        .into_iter()
        .filter(|e| e.level == tracing::Level::WARN && e.has_field("reason", "connection_cap"))
        .count();
    assert_eq!(
        cap_warnings, 1,
        "four refusals inside one warn interval must produce exactly one WARN",
    );
}
