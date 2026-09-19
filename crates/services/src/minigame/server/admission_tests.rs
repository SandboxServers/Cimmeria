//! Admission guards for the public minigame port: the handshake deadline in
//! [`super::handle_connection`] and the connection cap in [`super::serve`].
//!
//! Like the lifecycle tests in `tests.rs`, these drive real loopback TCP
//! sockets — the behaviour under test is who gets to keep a socket open and
//! for how long, which a mock would have to model anyway. They run on the
//! real clock with deadlines shrunk to a few hundred milliseconds rather
//! than on a paused clock: tokio's auto-advance can fire a timer before the
//! kernel has delivered a loopback write, which would make "the server
//! waited for the bytes" indistinguishable from "the server never saw them".

use std::time::Instant;

use tokio::io::AsyncReadExt;

use super::*;

/// Handshake deadline used by the tests. Short enough to keep the suite
/// fast, long enough that loopback scheduling jitter can't trip it early.
const SHORT_DEADLINE: Duration = Duration::from_millis(200);

/// Upper bound on how long any single observation here may take. A server
/// that never hangs up fails on this instead of wedging the suite.
const OBSERVE_WITHIN: Duration = Duration::from_secs(5);

/// Connect a loopback pair and return `(client, server, client_addr)`.
async fn loopback_pair() -> (TcpStream, TcpStream, std::net::SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let addr = listener.local_addr().expect("local_addr");
    let client = TcpStream::connect(addr).await.expect("connect loopback");
    let (server, peer) = listener.accept().await.expect("accept loopback");
    (client, server, peer)
}

/// Read until the server closes the connection or `within` runs out.
/// Returns `true` if the server hung up (EOF or reset).
async fn server_hangs_up(client: &mut TcpStream, within: Duration) -> bool {
    let mut sink = vec![0u8; MAX_MESSAGE_LEN];
    let hang_up = async {
        loop {
            match client.read(&mut sink).await {
                Ok(0) | Err(_) => return,
                Ok(_) => continue,
            }
        }
    };
    tokio::time::timeout(within, hang_up).await.is_ok()
}

/// A client that connects and never speaks must not hold its connection
/// task forever. Before the deadline existed, the first handshake read
/// awaited with no timeout, so one idle socket pinned a task and a 4 KB
/// buffer until the peer went away — and a flood of them exhausted the
/// server.
#[tokio::test]
async fn a_client_silent_in_the_handshake_is_disconnected_at_the_deadline() {
    let (mut client, server, peer) = loopback_pair().await;
    let (tx, _rx) = mpsc::channel(16);
    let started = Instant::now();
    let handle = tokio::spawn(handle_connection(
        server,
        peer,
        SessionRegistry::new(),
        tx,
        9339,
        SHORT_DEADLINE,
    ));

    assert!(
        server_hangs_up(&mut client, OBSERVE_WITHIN).await,
        "a client that never sends verChk must be disconnected at the \
         handshake deadline, not held open indefinitely",
    );
    assert!(
        started.elapsed() >= SHORT_DEADLINE,
        "the server hung up after {:?}, before the {SHORT_DEADLINE:?} \
         deadline — a silent client must get the whole window",
        started.elapsed(),
    );
    tokio::time::timeout(OBSERVE_WITHIN, handle)
        .await
        .expect("the connection task must end once the socket is dropped")
        .expect("connection task must not panic");
}

/// The deadline covers the whole handshake, not each read. A slowloris
/// client that trickles one byte at a time — never a NUL, so never a
/// complete frame — must still lose its socket at the deadline. With a
/// per-read timeout instead, every byte would reset the clock and the
/// connection would survive until the 4 KB frame cap, minutes later.
#[tokio::test]
async fn a_trickling_client_cannot_extend_the_handshake_deadline() {
    let (client, server, peer) = loopback_pair().await;
    let (tx, _rx) = mpsc::channel(16);
    let handle = tokio::spawn(handle_connection(
        server,
        peer,
        SessionRegistry::new(),
        tx,
        9339,
        SHORT_DEADLINE,
    ));

    let (mut reader, mut writer) = client.into_split();
    // One byte every quarter-deadline: always well inside any per-read
    // window, and the frame never terminates.
    let trickle = tokio::spawn(async move {
        while tokio::io::AsyncWriteExt::write_all(&mut writer, b"<")
            .await
            .is_ok()
        {
            tokio::time::sleep(SHORT_DEADLINE / 4).await;
        }
    });

    let mut sink = vec![0u8; MAX_MESSAGE_LEN];
    let hung_up = tokio::time::timeout(SHORT_DEADLINE * 10, async {
        loop {
            match reader.read(&mut sink).await {
                Ok(0) | Err(_) => return,
                Ok(_) => continue,
            }
        }
    })
    .await
    .is_ok();
    trickle.abort();

    assert!(
        hung_up,
        "a client trickling bytes must still be disconnected at the \
         handshake deadline; it was still connected after {:?}",
        SHORT_DEADLINE * 10,
    );
    tokio::time::timeout(OBSERVE_WITHIN, handle)
        .await
        .expect("the connection task must end at the deadline")
        .expect("connection task must not panic");
}

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

/// Send `verChk` and wait for `apiOK`. `true` means the server is actively
/// serving this socket — a positive proof it was admitted, not merely that
/// it has not been closed yet.
async fn completes_version_check(client: &mut TcpStream) -> bool {
    if send_null_terminated(
        client,
        "<msg t='sys'><body action='verChk' r='0'><ver v='154'/></body></msg>",
    )
    .await
    .is_err()
    {
        return false;
    }
    let mut seen = String::new();
    let mut chunk = vec![0u8; MAX_MESSAGE_LEN];
    let api_ok = async {
        loop {
            match client.read(&mut chunk).await {
                Ok(0) | Err(_) => return false,
                Ok(n) => {
                    seen.push_str(&String::from_utf8_lossy(&chunk[..n]));
                    if seen.contains("action='apiOK'") {
                        return true;
                    }
                }
            }
        }
    };
    tokio::time::timeout(OBSERVE_WITHIN, api_ok)
        .await
        .unwrap_or(false)
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

/// The deadline bounds the handshake only. A player who logged in and then
/// sits on the board past it must keep the session: the original had no
/// play-time cap, and per-game timers end a round as a normal Defeat.
#[tokio::test]
async fn a_session_that_logged_in_outlives_the_handshake_deadline() {
    let (mut client, server, peer) = loopback_pair().await;
    let registry = SessionRegistry::new();
    let ticket = registry
        .register(4311, 7, "Hack".into(), 1, 1, 0, 0, 0, 1, vec![4242])
        .await
        .expect("fresh registry must accept the session");
    let (tx, mut rx) = mpsc::channel(16);
    let handle = tokio::spawn(handle_connection(
        server,
        peer,
        registry,
        tx,
        9339,
        SHORT_DEADLINE,
    ));

    assert!(
        completes_version_check(&mut client).await,
        "verChk must be served"
    );
    send_null_terminated(
        &mut client,
        &format!(
            "<msg t='sys'><body action='login' r='0'><login z='Hack'>\
             <nick><![CDATA[4311]]></nick><pword><![CDATA[{ticket}]]></pword>\
             </login></body></msg>"
        ),
    )
    .await
    .expect("login send must succeed");
    super::tests::read_until_game_begin(&mut client).await;

    // Idle well past the handshake deadline, then win.
    tokio::time::sleep(SHORT_DEADLINE * 3).await;
    send_null_terminated(
        &mut client,
        "<msg t='xt'><body action='xtReq'>\
         <![CDATA[<dataObj><var n='cmd' t='s'>victory</var></dataObj>]]>\
         </body></msg>",
    )
    .await
    .expect("victory send must succeed");
    drop(client);
    tokio::time::timeout(OBSERVE_WITHIN, handle)
        .await
        .expect("the session must end once the client leaves")
        .expect("connection task must not panic");

    assert!(
        matches!(
            rx.try_recv(),
            Ok(CellToBaseMsg::MinigameResult { result_code, .. })
                if result_code == RESULT_VICTORY
        ),
        "a victory sent after the handshake deadline must still be reported — \
         the deadline must not reach into the game loop",
    );
}

/// Drive one pre-login frame through [`handle_connection`] and return every
/// WARN-or-worse event it produced. WARN events are forwarded to Discord,
/// so anything an anonymous probe can trigger must stay below WARN.
async fn warnings_from_probe(frame: &str) -> Vec<crate::test_support::Captured> {
    let capture = crate::test_support::LogCapture::install();
    let (mut client, server, peer) = loopback_pair().await;
    let (tx, _rx) = mpsc::channel(16);
    let handle = tokio::spawn(handle_connection(
        server,
        peer,
        SessionRegistry::new(),
        tx,
        9339,
        OBSERVE_WITHIN,
    ));
    send_null_terminated(&mut client, frame)
        .await
        .expect("probe send must succeed");
    assert!(
        server_hangs_up(&mut client, OBSERVE_WITHIN).await,
        "a frame that is not a valid handshake step must close the connection",
    );
    handle.await.expect("connection task must not panic");
    capture
        .all()
        .into_iter()
        .filter(|e| e.level <= tracing::Level::WARN)
        .collect()
}

/// An internet scanner speaking HTTP (or any non-SFS bytes) at the port is
/// expected noise. It must be dropped without a WARN, or every probe posts
/// to the Discord errors channel.
#[tokio::test]
async fn a_non_sfs_probe_is_dropped_without_a_warning() {
    let warnings = warnings_from_probe("GET / HTTP/1.1\r\nHost: example\r\n\r\n").await;
    assert!(
        warnings.is_empty(),
        "a non-SFS probe must not log at WARN or above; got {warnings:#?}",
    );
}

/// A well-formed SFS frame out of order — login before verChk — is just as
/// anonymous as garbage and gets the same quiet treatment.
#[tokio::test]
async fn an_out_of_order_handshake_frame_is_dropped_without_a_warning() {
    let warnings = warnings_from_probe(
        "<msg t='sys'><body action='login' r='0'><login z='Hack'>\
         <nick><![CDATA[1]]></nick><pword><![CDATA[x]]></pword></login></body></msg>",
    )
    .await;
    assert!(
        warnings.is_empty(),
        "an out-of-order handshake frame must not log at WARN or above; got {warnings:#?}",
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
