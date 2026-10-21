//! Guards for the public-port timeouts ([`super::limits`]) and for how the
//! listener logs what unauthenticated peers send.
//!
//! The port is reachable from the internet, so two things are pinned here:
//! a peer cannot hold a socket open without logging in, and bytes from a peer
//! that never logged in never raise a WARN (only WARN and ERROR reach the
//! Discord harvest). The authenticated case is the opposite: an unknown frame
//! from a real session is a protocol gap and must still WARN, with its fields
//! filled.

use std::net::SocketAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::Level;

use super::admission_tests::test_permit;
use super::tests::{
    drain_results, read_until_game_begin, spawn_placeholder_session,
    spawn_placeholder_session_with_idle,
};
use super::*;
use crate::test_support::LogCapture;

/// Loopback pair: `(client, server_half, server_peer_addr)`.
pub(super) async fn loopback_pair() -> (TcpStream, TcpStream, SocketAddr) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let addr = listener.local_addr().expect("local_addr");
    let client = TcpStream::connect(addr).await.expect("connect loopback");
    let (server, peer) = listener.accept().await.expect("accept loopback");
    (client, server, peer)
}

/// Run [`handle_connection`] over the server half of a fresh pair, against
/// `registry`.
pub(super) async fn connect_to(
    registry: SessionRegistry,
    limits: ListenerLimits,
) -> (
    TcpStream,
    mpsc::Receiver<CellToBaseMsg>,
    tokio::task::JoinHandle<()>,
) {
    let (client, server, peer) = loopback_pair().await;
    let (tx, rx) = mpsc::channel(16);
    let permit = test_permit(peer).await;
    let handle = tokio::spawn(handle_connection(
        server, peer, permit, registry, tx, 9339, limits,
    ));
    (client, rx, handle)
}

/// [`connect_to`] with an empty registry: nothing can log in.
pub(super) async fn spawn_connection(
    limits: ListenerLimits,
) -> (
    TcpStream,
    mpsc::Receiver<CellToBaseMsg>,
    tokio::task::JoinHandle<()>,
) {
    connect_to(SessionRegistry::new(), limits).await
}

/// Stand up [`accept::serve`] on an ephemeral port over `registry`.
async fn spawn_listener_on(registry: SessionRegistry, limits: ListenerLimits) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let addr = listener.local_addr().expect("local_addr");
    let (tx, rx) = mpsc::channel(16);
    tokio::spawn(async move {
        // Keep the receiver alive as long as the server.
        let _rx = rx;
        accept::serve(listener, 9339, registry, tx, limits).await;
    });
    addr
}

/// `true` if the server closed this socket within `wait`; `false` if it is
/// still open (the read timed out with nothing to read).
async fn closed_within(client: &mut TcpStream, wait: Duration) -> bool {
    let mut byte = [0u8; 1];
    match tokio::time::timeout(wait, client.read(&mut byte)).await {
        Ok(Ok(0)) | Ok(Err(_)) => true,
        Ok(Ok(_)) => panic!("a silent pre-login peer must not be sent anything"),
        Err(_) => false,
    }
}

pub(super) fn assert_no_warn_or_error(capture: &crate::test_support::LogCaptureGuard) {
    let loud: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.level == Level::WARN || c.level == Level::ERROR)
        .collect();
    assert!(
        loud.is_empty(),
        "input from a peer that never logged in must not WARN or ERROR \
         (those reach Discord): {loud:#?}",
    );
}

/// A peer no minigame session expects is closed before any byte is read: the
/// server sends nothing and the socket reaches EOF. Without the expected-peer
/// check the connection is admitted and sits waiting for a handshake, so the
/// read times out and this fails.
#[tokio::test]
async fn listener_closes_unexpected_peer() {
    let addr = spawn_listener_on(SessionRegistry::new(), ListenerLimits::default()).await;

    let mut client = TcpStream::connect(addr).await.expect("connect");

    assert!(
        closed_within(&mut client, Duration::from_secs(2)).await,
        "a peer no session expects must be closed without being read from",
    );
}

/// A session registered for another public address does not admit a loopback
/// peer (non-public peers only match other non-public ones, #1119):
/// the socket is closed, and because a session is registered the refusal is
/// the unthrottled `unexpected_peer_with_sessions` row rather than the
/// throttled `unexpected_peer` one.
#[tokio::test]
async fn listener_closes_peer_when_session_is_for_another_ip() {
    let capture = LogCapture::install();
    let registry = SessionRegistry::new();
    registry
        .register(
            4315,
            7,
            "Hack".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![],
            vec![],
            None,
            Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(
                203, 0, 113, 5,
            ))),
            1,
        )
        .await
        .expect("fresh registry must accept the session");
    let addr = spawn_listener_on(registry, ListenerLimits::default()).await;

    let mut client = TcpStream::connect(addr).await.expect("connect");

    assert!(
        closed_within(&mut client, Duration::from_secs(2)).await,
        "a peer no session expects must be closed even when sessions exist",
    );
    assert!(
        capture
            .find_event(
                Level::INFO,
                "sessions are registered",
                "unexpected_peer_with_sessions",
            )
            .is_some(),
        "no unexpected_peer_with_sessions row: {:#?}",
        capture.all(),
    );
}

/// A refused peer with one pending session (registered, its SWF never
/// connected) logs `pending_sessions=1` on the unthrottled refusal row. Without
/// the field the row has no `pending_sessions` key and this fails.
#[tokio::test]
async fn refused_peer_row_counts_pending_sessions() {
    let capture = LogCapture::install();
    let registry = SessionRegistry::new();
    registry
        .register(
            4316,
            7,
            "Hack".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![],
            vec![],
            None,
            Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(
                203, 0, 113, 6,
            ))),
            1,
        )
        .await
        .expect("fresh registry must accept the session");
    let addr = spawn_listener_on(registry, ListenerLimits::default()).await;

    let mut client = TcpStream::connect(addr).await.expect("connect");

    assert!(
        closed_within(&mut client, Duration::from_secs(2)).await,
        "a peer no session expects must be closed even with a pending session",
    );
    let row = capture
        .find_event(
            Level::INFO,
            "sessions are registered",
            "unexpected_peer_with_sessions",
        )
        .expect("the refused peer must log the unthrottled refusal row");
    assert!(
        row.has_field("pending_sessions", "1"),
        "pending_sessions must be 1 with one pending session: {row:#?}",
    );
}

/// A peer that connects and says nothing is closed at the handshake
/// deadline, not before. Paused time makes the 30 s deadline instant; the
/// outer guard is far longer, so without the deadline the guard fires and
/// the test fails rather than hanging.
#[tokio::test(start_paused = true)]
async fn a_silent_peer_is_closed_at_the_handshake_deadline() {
    let capture = LogCapture::install();
    let limits = ListenerLimits::default();
    let deadline = limits.handshake_timeout;
    let started = tokio::time::Instant::now();
    let (mut client, _rx, handle) = spawn_connection(limits).await;

    tokio::time::timeout(Duration::from_secs(3600), handle)
        .await
        .expect("a silent peer must be dropped at the handshake deadline")
        .expect("connection task must not panic");

    assert!(
        started.elapsed() >= deadline,
        "closed after {:?}, before the {deadline:?} deadline",
        started.elapsed(),
    );
    let mut byte = [0u8; 1];
    assert_eq!(
        client.read(&mut byte).await.unwrap_or(0),
        0,
        "the server must have closed the socket",
    );
    assert!(capture
        .find_event(Level::DEBUG, "did not log in", "handshake_timeout")
        .is_some());
    assert_no_warn_or_error(&capture);
}

/// Scanner bytes (a TLS ClientHello prefix, whose length field holds the
/// NUL that ends an SFS frame; an HTTP request) log at DEBUG with a reason
/// and the peer. Before the fix each one raised the empty-field
/// `Unknown SFS message type` WARN that reached Discord.
#[tokio::test]
async fn non_sfs_preauth_input_does_not_warn() {
    let capture = LogCapture::install();
    let probes: [&[u8]; 3] = [
        b"\x16\x03\x01\x00\xa5\x01\x00\x00\xa1\x03\x03",
        b"GET / HTTP/1.1\r\nHost: example\r\n\r\n\0",
        b"\x03\x00\x00\x13\x0e\xe0\x00\x00",
    ];
    for probe in probes {
        let (mut client, _rx, handle) = spawn_connection(ListenerLimits::default()).await;
        client.write_all(probe).await.expect("probe send");
        tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .expect("the server must close a non-SFS peer")
            .expect("connection task must not panic");
    }

    let row = capture
        .find_event(
            Level::DEBUG,
            "rejected a pre-login frame",
            "non_sfs_preauth",
        )
        .unwrap_or_else(|| panic!("no non_sfs_preauth row: {:#?}", capture.all()));
    assert!(row.fields.contains_key("peer"), "{row:#?}");
    assert!(row.has_field("parse_error", "no_sfs_envelope"), "{row:#?}");
    assert_no_warn_or_error(&capture);
}

/// A pre-login frame over the size cap closes the connection, at DEBUG.
#[tokio::test]
async fn an_oversized_preauth_frame_is_rejected_quietly() {
    let capture = LogCapture::install();
    let (mut client, _rx, handle) = spawn_connection(ListenerLimits::default()).await;
    // No terminator anywhere: the buffer fills and the reader gives up.
    let _ = client.write_all(&vec![b'A'; MAX_MESSAGE_LEN + 512]).await;
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("an oversized frame must close the connection")
        .expect("connection task must not panic");

    assert!(capture
        .find_event(Level::DEBUG, "size limit", "preauth_message_too_long")
        .is_some());
    assert_no_warn_or_error(&capture);
}

/// An authenticated session that sends an oversized frame is closed and
/// reported as a cancel, and that one does WARN: a logged-in SWF should
/// never do it.
#[tokio::test]
async fn an_oversized_frame_in_session_closes_it() {
    let capture = LogCapture::install();
    let (mut client, mut rx, handle) = spawn_placeholder_session(4310).await;
    read_until_game_begin(&mut client).await;
    let _ = client.write_all(&vec![b'A'; MAX_MESSAGE_LEN + 512]).await;
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("an oversized frame must end the session")
        .expect("session task must not panic");

    assert_eq!(drain_results(&mut rx).len(), 1);
    let row = capture
        .find_event(Level::WARN, "size limit", "message_too_long")
        .expect("in-session oversize must WARN");
    assert!(row.has_field("entity_id", "4310"), "{row:#?}");
}

/// The real protocol gap: an authenticated SWF sends a well-formed SFS
/// frame this server does not handle. It stays WARN, and the row names the
/// session and the `(t, action)` pair. Before the fix the codec's WARN had
/// no session fields and the game loop's own WARN had no type fields.
#[tokio::test(start_paused = true)]
async fn unknown_type_in_session_warns_with_fields() {
    let capture = LogCapture::install();
    let (mut client, mut rx, handle) = spawn_placeholder_session(4311).await;
    read_until_game_begin(&mut client).await;
    send_null_terminated(
        &mut client,
        "<msg t='sys'><body action='roundTrip' r='1'></body></msg>",
    )
    .await
    .expect("send");
    // A client-declared victory counts only after 5 s of play (D-MG13).
    tokio::time::advance(Duration::from_secs(5)).await;
    let victory = "<msg t='xt'><body action='xtReq'>\
         <![CDATA[<dataObj><var n='cmd' t='s'>victory</var></dataObj>]]>\
         </body></msg>";
    send_null_terminated(&mut client, victory)
        .await
        .expect("send");
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("the session must end on victory")
        .expect("session task must not panic");

    // The unknown frame did not end the session.
    assert_eq!(drain_results(&mut rx).len(), 1);
    let row = capture
        .find_event(Level::WARN, "Unknown SFS message type", "unknown_sfs_type")
        .unwrap_or_else(|| panic!("no unknown-type WARN: {:#?}", capture.all()));
    assert!(row.has_field("msg_type", "sys"), "{row:#?}");
    assert!(row.has_field("body_action", "roundTrip"), "{row:#?}");
    assert!(row.has_field("entity_id", "4311"), "{row:#?}");
    assert!(row.has_field("game", "Hack"), "{row:#?}");
}

/// A logged-in client that goes silent is closed at the idle deadline and
/// reported as a cancel, like a closed window. Paused time; the outer guard
/// fails the test instead of hanging if the deadline is missing.
///
/// The idle deadline is 30 s, half the shell's 60 s countdown (MG-64), so
/// the idle arm fires before the countdown can end the placeholder game.
#[tokio::test(start_paused = true)]
async fn an_idle_session_ends_as_canceled() {
    let idle = Duration::from_secs(30);
    let started = tokio::time::Instant::now();
    // The client never reads: the session's startup frames fit in the
    // socket buffer, and reading would arm timers that race the deadline.
    let (_client, mut rx, handle) = spawn_placeholder_session_with_idle(4312, idle).await;

    tokio::time::timeout(Duration::from_secs(3600), handle)
        .await
        .expect("an idle session must end at the idle deadline")
        .expect("session task must not panic");

    assert!(
        started.elapsed() >= idle,
        "ended after {:?}",
        started.elapsed()
    );
    assert_eq!(drain_results(&mut rx).len(), 1);
}

/// Send the two frames a real SWF sends on connect: `verChk`, then `login`
/// with the entity id as `nick` and the ticket as `pword`.
pub(super) async fn send_handshake(client: &mut TcpStream, entity_id: u32, ticket: &str) {
    send_null_terminated(
        client,
        "<msg t='sys'><body action='verChk' r='0'><ver v='154'/></body></msg>",
    )
    .await
    .expect("verChk send");
    send_null_terminated(
        client,
        &format!(
            "<msg t='sys'><body action='login' r='0'><login z='Hack'>\
             <nick><![CDATA[{entity_id}]]></nick><pword><![CDATA[{ticket}]]></pword>\
             </login></body></msg>"
        ),
    )
    .await
    .expect("login send");
}

/// A ticket authenticates one live connection. A second login with the
/// same ticket while the first is playing is closed, and the first session
/// plays on to its own result. Without the `connected` check the second
/// login claims the session again and starts a second game against the
/// same victory chains.
#[tokio::test(start_paused = true)]
async fn a_second_login_with_a_live_ticket_is_refused() {
    let capture = LogCapture::install();
    let registry = SessionRegistry::new();
    let ticket = registry
        .register(
            4313,
            7,
            "Hack".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![99],
            vec![],
            None,
            Some(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
            1,
        )
        .await
        .expect("fresh registry must accept the session");

    let (mut first, mut first_rx, first_handle) =
        connect_to(registry.clone(), ListenerLimits::default()).await;
    send_handshake(&mut first, 4313, &ticket).await;
    read_until_game_begin(&mut first).await;

    let (mut second, mut second_rx, second_handle) =
        connect_to(registry.clone(), ListenerLimits::default()).await;
    send_handshake(&mut second, 4313, &ticket).await;
    tokio::time::timeout(Duration::from_secs(5), second_handle)
        .await
        .expect("the second login must be closed")
        .expect("connection task must not panic");
    let mut rest = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(5), second.read_to_end(&mut rest)).await;
    let rest = String::from_utf8_lossy(&rest).replace('\0', "\n");
    assert!(
        !rest.contains("onGameBegin"),
        "the second login must not start a game:\n{rest}",
    );
    assert!(drain_results(&mut second_rx).is_empty());
    let row = capture
        .find_event(
            Level::WARN,
            "already in use by a live connection",
            "ticket_already_claimed",
        )
        .expect("a second login on a live ticket must WARN");
    assert!(row.fields.contains_key("peer"), "{row:#?}");
    assert!(row.has_field("entity_id", "4313"), "{row:#?}");

    // The first session is untouched and still wins normally, once it has
    // played the 5 s minimum (D-MG13).
    tokio::time::advance(Duration::from_secs(5)).await;
    let victory = "<msg t='xt'><body action='xtReq'>\
         <![CDATA[<dataObj><var n='cmd' t='s'>victory</var></dataObj>]]>\
         </body></msg>";
    send_null_terminated(&mut first, victory)
        .await
        .expect("send");
    drop(first);
    tokio::time::timeout(Duration::from_secs(5), first_handle)
        .await
        .expect("the first session must end on victory")
        .expect("connection task must not panic");
    assert_eq!(drain_results(&mut first_rx).len(), 1);
}

/// The login row names the peer, the player and the ticket's age, and never
/// carries the ticket. Without the `minigame.claimed` fields the row has no
/// `player_id` or `peer`, and the assertions fail.
#[tokio::test]
async fn a_successful_login_logs_the_claimed_row_with_identity() {
    let capture = LogCapture::install();
    let registry = SessionRegistry::new();
    let ticket = registry
        .register(
            4317,
            555,
            "Hack".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![],
            vec![],
            Some("Tester".into()),
            Some(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
            1,
        )
        .await
        .expect("fresh registry must accept the session");

    let (mut client, _rx, _handle) = connect_to(registry.clone(), ListenerLimits::default()).await;
    send_handshake(&mut client, 4317, &ticket).await;
    read_until_game_begin(&mut client).await;

    let row = capture
        .find_message(Level::INFO, "Minigame login successful")
        .expect("a successful login must log its row");
    assert!(row.has_field("event", "minigame.claimed"), "{row:#?}");
    assert!(row.has_field("entity_id", "4317"), "{row:#?}");
    assert!(row.has_field("player_id", "555"), "{row:#?}");
    assert!(row.has_field("player_name", "Tester"), "{row:#?}");
    assert!(row.fields.contains_key("peer"), "{row:#?}");
    assert!(row.fields.contains_key("ticket_age_ms"), "{row:#?}");
    assert!(
        !row.fields.values().any(|v| v.contains(&ticket)),
        "the login row must not carry the ticket: {row:#?}",
    );
}

/// A crafted login naming a registered (guessable) entity id with a
/// made-up ticket or the wrong game is refused below WARN, with the peer.
/// The registry used to WARN both, so a peer looping them reached Discord.
#[tokio::test]
async fn crafted_logins_for_a_registered_entity_do_not_warn() {
    let capture = LogCapture::install();
    let registry = SessionRegistry::new();
    let ticket = registry
        .register(
            4314,
            7,
            "Livewire".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![],
            vec![],
            None,
            Some(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
            1,
        )
        .await
        .expect("fresh registry must accept the session");
    // `send_handshake` logs in to the `Hack` zone.
    for (attempt, password) in [("ticket", "0".repeat(64)), ("game", ticket.clone())] {
        let (mut client, _rx, handle) =
            connect_to(registry.clone(), ListenerLimits::default()).await;
        send_handshake(&mut client, 4314, &password).await;
        tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .unwrap_or_else(|_| panic!("{attempt}: a refused login must close"))
            .expect("connection task must not panic");
    }

    for (message, reason) in [
        ("ticket mismatch", "ticket_mismatch"),
        ("game name mismatch", "game_name_mismatch"),
    ] {
        let row = capture
            .find_event(Level::INFO, message, reason)
            .unwrap_or_else(|| panic!("no {reason} row: {:#?}", capture.all()));
        assert!(row.fields.contains_key("peer"), "{row:#?}");
        assert!(row.has_field("entity_id", "4314"), "{row:#?}");
    }
    assert_no_warn_or_error(&capture);
    // Neither attempt claimed the session.
    assert!(registry
        .authenticate_and_claim(4314, &ticket, "Livewire")
        .await
        .is_ok());
}
