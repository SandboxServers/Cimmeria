//! Guards for the minigame SmartFox server: session lifecycle, the
//! handshake deadline, the connection caps, what anonymous probes may log,
//! and the one-connection-per-ticket login rule.
//!
//! Every test here drives real loopback TCP sockets rather than a fake,
//! because the behaviour under test *is* the socket lifecycle — who gets to
//! keep a socket open, for how long, and what the server reports upstream
//! when the client half goes away. A mock socket would have to model EOF
//! anyway. Deadline tests run on the real clock with deadlines shrunk to a
//! few hundred milliseconds rather than on a paused clock: tokio's
//! auto-advance can fire a timer before the kernel has delivered a loopback
//! write, which would make "the server waited for the bytes"
//! indistinguishable from "the server never saw them".
//!
//! `Hack` is the game type throughout: `PlaceholderGame` wins instantly on
//! the `victory` extension command and reports `needs_tick() == false`, so
//! no test depends on the 250 ms tick timer.

mod connection_cap;
mod deadline;
mod lifecycle;
mod login;
mod probes;

use std::time::Instant;

use tokio::io::AsyncReadExt;

use super::*;

/// Handshake deadline used by the tests. Short enough to keep the suite
/// fast, long enough that loopback scheduling jitter can't trip it early.
const SHORT_DEADLINE: Duration = Duration::from_millis(200);

/// Upper bound on how long any single observation here may take. A server
/// that never hangs up fails on this instead of wedging the suite.
const OBSERVE_WITHIN: Duration = Duration::from_secs(5);

/// The `victory` extension command: an instant win for `PlaceholderGame`.
const VICTORY: &str = "<msg t='xt'><body action='xtReq'>\
     <![CDATA[<dataObj><var n='cmd' t='s'>victory</var></dataObj>]]>\
     </body></msg>";

/// SFS login frame for a `Hack` session: `nick` is the entity id, `pword`
/// the ticket.
fn hack_login(entity_id: u32, ticket: &str) -> String {
    format!(
        "<msg t='sys'><body action='login' r='0'><login z='Hack'>\
         <nick><![CDATA[{entity_id}]]></nick><pword><![CDATA[{ticket}]]></pword>\
         </login></body></msg>"
    )
}

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

/// Run [`handle_connection`] for the server half of a loopback pair.
fn spawn_connection(
    server: TcpStream,
    peer: std::net::SocketAddr,
    registry: SessionRegistry,
    tx: mpsc::Sender<CellToBaseMsg>,
    handshake_timeout: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(handle_connection(
        server,
        peer,
        registry,
        tx,
        9339,
        handshake_timeout,
    ))
}

/// Wait for a connection task to finish, failing the test instead of
/// wedging the suite if it never does, or if it panicked.
async fn join_within(task: tokio::task::JoinHandle<()>, what: &str) {
    tokio::time::timeout(OBSERVE_WITHIN, task)
        .await
        .unwrap_or_else(|_| {
            panic!("{what}: the connection task did not end within {OBSERVE_WITHIN:?}")
        })
        .expect("connection task must not panic");
}

/// Read until the server closes the connection (EOF or reset) or `within`
/// runs out. Returns everything received, NULs swapped for newlines so a
/// failure message stays printable, or `None` if the socket was still open.
async fn read_until_closed(client: &mut TcpStream, within: Duration) -> Option<String> {
    let mut seen = Vec::new();
    let mut chunk = vec![0u8; MAX_MESSAGE_LEN];
    let closed = async {
        loop {
            match client.read(&mut chunk).await {
                Ok(0) | Err(_) => return,
                Ok(n) => seen.extend_from_slice(&chunk[..n]),
            }
        }
    };
    tokio::time::timeout(within, closed).await.ok()?;
    Some(String::from_utf8_lossy(&seen).replace('\0', "\n"))
}

/// `true` if the server hangs up within `within`.
async fn server_hangs_up(client: &mut TcpStream, within: Duration) -> bool {
    read_until_closed(client, within).await.is_some()
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

/// Read framed messages until `onGameBegin`, the last frame the server
/// sends before entering its game loop.
///
/// A single bounded read is not enough: it can return after any one early
/// startup frame, so closing the socket on the strength of it can make a
/// *later* startup write fail. The session then exits through the handshake
/// path instead of the mid-game path, and a test meaning to exercise a
/// mid-game close silently exercises something else. Waiting for the
/// milestone makes "the server is in its game loop" an assertion rather
/// than an assumption.
///
/// Panics on timeout, EOF or I/O error — reaching the game loop is a
/// precondition of every caller, not something to paper over.
async fn read_until_game_begin(client: &mut TcpStream) {
    const MILESTONE: &str = "<var n='_cmd' t='s'>onGameBegin</var>";
    let mut seen = String::new();
    let mut chunk = vec![0u8; MAX_MESSAGE_LEN];
    loop {
        match tokio::time::timeout(
            Duration::from_secs(5),
            tokio::io::AsyncReadExt::read(client, &mut chunk),
        )
        .await
        {
            Ok(Ok(n)) if n > 0 => {
                seen.push_str(&String::from_utf8_lossy(&chunk[..n]));
                if seen.contains(MILESTONE) {
                    return;
                }
            }
            // EOF, read error, or timeout: the server never reached its
            // game loop, so whatever the caller meant to exercise did not
            // happen. Show the frames that did arrive, NULs swapped for
            // newlines so the message stays printable.
            other => panic!(
                "server did not reach onGameBegin ({other:?}); frames so far:\n{}",
                seen.replace('\0', "\n"),
            ),
        }
    }
}

/// Collect every `MinigameResult` the session dispatched.
fn drain_results(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<(u8, Vec<i64>)> {
    let mut out = Vec::new();
    while let Ok(CellToBaseMsg::MinigameResult {
        result_code,
        on_victory_chains,
        ..
    }) = rx.try_recv()
    {
        out.push((result_code, on_victory_chains));
    }
    out
}
