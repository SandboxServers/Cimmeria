//! Session-lifecycle guards for [`super::run_session`] and
//! [`super::handle_connection`] (Castle CA04, defect B4).
//!
//! These drive a real loopback TCP pair rather than a fake, because the
//! behaviour under test *is* the socket lifecycle: what the server reports
//! upstream when the client half goes away versus when the game produced
//! an outcome of its own. A mock socket would have to model EOF anyway.
//!
//! `Hack` is used as the game type: `PlaceholderGame` wins instantly on the
//! `victory` extension command and reports `needs_tick() == false`, so no
//! test here depends on the 250 ms tick timer.

use super::*;
use crate::minigame::game::create_game;

/// Stand up a loopback TCP pair and drive [`run_session`] over the server
/// half with a placeholder instance.
///
/// Returns `(client_half, result_rx, join_handle)`. The caller drives the
/// client half — sending `victory` to finish the game, or simply dropping
/// it to simulate the player closing the SWF.
async fn spawn_placeholder_session(
    entity_id: u32,
) -> (
    TcpStream,
    mpsc::Receiver<CellToBaseMsg>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let addr = listener.local_addr().expect("local_addr");
    let client = TcpStream::connect(addr).await.expect("connect loopback");
    let (server, _) = listener.accept().await.expect("accept loopback");

    let registry = SessionRegistry::new();
    let ticket = registry
        .register(entity_id, 7, "Hack".into(), 1, 1, 0, 0, 0, 1, vec![4242])
        .await
        .expect("fresh registry must accept the session");
    // Go through the claiming path rather than building a `MinigameSession`
    // by hand, so the test walks the same call the login handler makes.
    let session = registry
        .authenticate_and_claim(entity_id, &ticket, "Hack")
        .await
        .expect("session must authenticate and claim");
    let game = create_game(&session).expect("placeholder instance");

    let (tx, rx) = mpsc::channel(16);
    let handle = tokio::spawn(async move {
        run_session(
            server,
            &registry,
            &tx,
            session,
            game,
            vec![0u8; MAX_MESSAGE_LEN],
            0,
        )
        .await;
    });
    (client, rx, handle)
}

/// Wait until the server is in its game loop, then drop the socket. Reading
/// first both keeps the server's writes from blocking and guarantees the
/// close lands mid-game rather than mid-handshake.
async fn drain_and_close(mut client: TcpStream) {
    read_until_game_begin(&mut client).await;
    drop(client);
}

/// Close only the client's write half, then read the server's side to EOF.
///
/// A half-close is what lets a test *observe* the teardown: the server sees
/// EOF on its next read and runs the abort path, and the client is still
/// able to receive everything the abort path writes. Dropping the whole
/// socket instead throws those bytes away, which is why the abort tests
/// could not tell whether `aborted()` had actually run.
async fn half_close_and_read_to_eof(mut client: TcpStream) -> String {
    tokio::io::AsyncWriteExt::shutdown(&mut client)
        .await
        .expect("client write-half shutdown must succeed");

    let mut out = Vec::new();
    let mut chunk = vec![0u8; MAX_MESSAGE_LEN];
    loop {
        match tokio::time::timeout(
            Duration::from_secs(5),
            tokio::io::AsyncReadExt::read(&mut client, &mut chunk),
        )
        .await
        {
            // EOF: the server dropped its end after teardown.
            Ok(Ok(0)) | Err(_) => break,
            Ok(Ok(n)) => out.extend_from_slice(&chunk[..n]),
            Ok(Err(_)) => break,
        }
    }
    // Frames are NUL-delimited on the wire. Swap the terminators for
    // newlines: a raw NUL in a captured stream makes the whole assertion
    // message register as binary, and `grep` on a failing CI log then
    // prints "Binary file matches" instead of the failure.
    String::from_utf8_lossy(&out).replace('\0', "\n")
}

/// Count the `failure` extension frames in a captured server stream.
///
/// `PlaceholderGame::aborted` is the only thing that emits one — `started()`
/// sends `fullgamestate` and the teardown sends onPlayerLeaveGame /
/// onGameEnd / roomDel — so this is a direct observation of the
/// `aborted()` call.
fn count_failure_frames(stream: &str) -> usize {
    stream.matches("<var n='_cmd' t='s'>failure</var>").count()
}

/// One bounded read, for a phase with no later frame to miss.
async fn read_one_chunk(client: &mut TcpStream) {
    let mut sink = vec![0u8; MAX_MESSAGE_LEN];
    let _ = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::io::AsyncReadExt::read(client, &mut sink),
    )
    .await;
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
pub(super) async fn read_until_game_begin(client: &mut TcpStream) {
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

/// **Defect B4 regression guard (abort half).** A client that closes its
/// socket mid-game must produce exactly one upstream result coded
/// `RESULT_CANCELED`, *and* the instance's `aborted()` must actually run.
///
/// Two independent reverts fail this. Removing the `if !result_reported`
/// block drops the upstream report. Removing just the
/// `for output in game.aborted()` loop inside it leaves the report intact
/// but stops the `failure` frame reaching the client — which is why the
/// client stream is asserted on rather than only the result code.
#[tokio::test]
async fn closing_the_swf_reports_canceled_and_runs_aborted() {
    let (client, mut rx, handle) = spawn_placeholder_session(4301).await;
    let stream = half_close_and_read_to_eof(client).await;
    handle.await.expect("session task must not panic");

    let results = drain_results(&mut rx);
    assert_eq!(
        results.len(),
        1,
        "an abandoned session must report exactly once; got {results:?}",
    );
    assert_eq!(
        results[0],
        (RESULT_CANCELED, vec![]),
        "the original reported MinigameCanceled (0) on abort, never Defeat \
         (2) — Defeat is reserved for a game the player actually lost",
    );
    assert!(
        stream.contains("<var n='_cmd' t='s'>onGameBegin</var>"),
        "the capture must show the server reached its game loop, or this is \
         not the mid-game close the test claims to exercise. Captured \
         stream: {stream}",
    );
    assert_eq!(
        count_failure_frames(&stream),
        1,
        "`MinigameInstance::aborted()` must run exactly once on an abandoned \
         session and its output must reach the client; the placeholder's \
         only `failure` frame comes from `aborted()`. Captured stream: \
         {stream}",
    );
}

/// The abort report must not fire when the game produced a real outcome.
/// Without the `result_reported` flag the teardown would append a bogus
/// `RESULT_CANCELED` after every victory, and that second message would be
/// indistinguishable from a real cancel upstream.
#[tokio::test]
async fn a_won_game_reports_victory_once_and_never_canceled() {
    let (mut client, mut rx, handle) = spawn_placeholder_session(4302).await;

    // Wait until the server is actually in its game loop, then win.
    // `PlaceholderGame` treats the `victory` extension command as an
    // instant win.
    read_until_game_begin(&mut client).await;
    let victory = "<msg t='xt'><body action='xtReq'>\
         <![CDATA[<dataObj><var n='cmd' t='s'>victory</var></dataObj>]]>\
         </body></msg>";
    send_null_terminated(&mut client, victory)
        .await
        .expect("victory send must succeed");
    // The milestone is already behind us, so just let go of the socket —
    // waiting for a second `onGameBegin` would hang until the timeout.
    drop(client);
    handle.await.expect("session task must not panic");

    let results = drain_results(&mut rx);
    assert_eq!(
        results,
        vec![(RESULT_VICTORY, vec![4242])],
        "a won game must report victory exactly once, carrying its victory \
         chains, with no trailing cancel",
    );
}

/// **Defect B4 regression guard (unregister half), end to end.** Drives
/// the real [`handle_connection`] — version check, ticket login, then the
/// client walks away — and asserts the entity can launch again at once.
///
/// This is the half that makes the player whole: the TTL sweep covers a
/// session whose SWF never connected, and `handle_connection`'s
/// `remove_if_ticket` covers one that connected and then went away. Fails
/// with that removal deleted, and fails if any `run_session` exit path is
/// ever allowed to return past it — which is exactly what the four
/// handshake sends used to do before they were moved behind `run_session`.
///
/// It also pins that login claims the session, which happens at exactly one
/// production call site. Without this assertion, dropping the claim passes
/// every other test here while reopening a worse bug than B4: a Livewire
/// round running past `PENDING_SESSION_TTL` gets swept mid-play and a second
/// interaction registers a fresh session for the same entity.
#[tokio::test]
async fn a_closed_connection_leaves_the_entity_free_to_relaunch() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let addr = listener.local_addr().expect("local_addr");
    let mut client = TcpStream::connect(addr).await.expect("connect loopback");
    let (server, peer) = listener.accept().await.expect("accept loopback");

    let registry = SessionRegistry::new();
    let ticket = registry
        .register(4303, 7, "Hack".into(), 1, 1, 0, 0, 0, 1, vec![])
        .await
        .expect("fresh registry must accept the session");

    let (tx, _rx) = mpsc::channel(16);
    let reg = registry.clone();
    let handle = tokio::spawn(handle_connection(
        server,
        peer,
        reg,
        tx,
        9339,
        Duration::from_secs(30),
    ));

    // Phase 1 — verChk. The server answers with the cross-domain policy
    // and apiOK; 154 is the version the original SWFs were built against.
    send_null_terminated(
        &mut client,
        "<msg t='sys'><body action='verChk' r='0'><ver v='154'/></body></msg>",
    )
    .await
    .expect("verChk send must succeed");
    // One bounded read is right here: the version phase answers with the
    // cross-domain policy and apiOK and then waits for login, so there is no
    // later frame to miss.
    read_one_chunk(&mut client).await;

    // Phase 2 — login. `z` is the game name, `nick` the entity id, `pword`
    // the ticket minted above.
    send_null_terminated(
        &mut client,
        &format!(
            "<msg t='sys'><body action='login' r='0'><login z='Hack'>\
             <nick><![CDATA[4303]]></nick><pword><![CDATA[{ticket}]]></pword>\
             </login></body></msg>"
        ),
    )
    .await
    .expect("login send must succeed");

    // The login handler has to have claimed the session before it starts the
    // room sends. Poll rather than assert immediately: this test races the
    // spawned task, so give it a bounded window.
    let connected = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(s) = registry.authenticate(4303, &ticket, "Hack").await {
                if s.connected {
                    return true;
                }
            } else {
                // Session already torn down — connected was never observed.
                return false;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the claim must happen well within 5s");
    assert!(
        connected,
        "login must claim the session; without it the sweep can evict a \
         session that is being actively played",
    );

    // Player closes the minigame window mid-game.
    drain_and_close(client).await;
    handle.await.expect("connection task must not panic");

    assert!(
        registry
            .register(4303, 7, "Hack".into(), 1, 1, 0, 0, 0, 1, vec![])
            .await
            .is_some(),
        "after a connection closes, the entity must be able to launch again \
         immediately — not after waiting out PENDING_SESSION_TTL, and not \
         only after a relog",
    );
}

/// **Regression guard for the `break 'session` routing.** A send that
/// fails during the handshake must still reach the common teardown:
/// `aborted()` runs and exactly one `RESULT_CANCELED` is reported.
///
/// Before this, each of the seven handshake / `started()` sends `return`ed
/// straight past the teardown block, so a socket that died between
/// `game.started()` and the game loop produced no `aborted()` call and no
/// upstream result at all — the session just evaporated.
///
/// The failure is forced deterministically: the client sets `SO_LINGER` to
/// zero and closes *before* the server writes anything, so the close is an
/// RST rather than a FIN and the server's first `write_all` errors. Reverting
/// any `break 'session` to `return` fails this.
#[tokio::test]
async fn a_send_failure_during_handshake_still_reports_canceled() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let addr = listener.local_addr().expect("local_addr");
    let client = TcpStream::connect(addr).await.expect("connect loopback");
    let (server, _) = listener.accept().await.expect("accept loopback");

    let registry = SessionRegistry::new();
    let ticket = registry
        .register(4304, 7, "Hack".into(), 1, 1, 0, 0, 0, 1, vec![])
        .await
        .expect("fresh registry must accept the session");
    let session = registry
        .authenticate_and_claim(4304, &ticket, "Hack")
        .await
        .expect("session must authenticate and claim");
    let game = create_game(&session).expect("placeholder instance");

    // Linger 0 makes close() emit RST instead of FIN, which turns the
    // server's next write into an error rather than a successful buffered
    // send. Done before the server task starts so the reset is already
    // queued when `run_session` writes rmList.
    //
    // A half-close (`shutdown`) or a plain drop sends FIN, and a FIN does
    // not stop the peer writing -- the server's sends would all succeed and
    // the test would stop being a guard for the routing at all. The reset is
    // the whole point, so this goes through `socket2::SockRef`, which
    // borrows the tokio socket and offers the same option without tokio's
    // deprecated `TcpStream::set_linger` wrapper.
    socket2::SockRef::from(&client)
        .set_linger(Some(Duration::ZERO))
        .expect("SO_LINGER must be settable on a loopback TCP socket");
    drop(client);
    // Let the RST land before the first server write.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let (tx, mut rx) = mpsc::channel(16);
    let reg = registry.clone();
    tokio::spawn(async move {
        run_session(
            server,
            &reg,
            &tx,
            session,
            game,
            vec![0u8; MAX_MESSAGE_LEN],
            0,
        )
        .await;
        reg.remove_if_ticket(4304, &ticket).await;
    })
    .await
    .expect("session task must not panic");

    let results = drain_results(&mut rx);
    assert_eq!(
        results,
        vec![(RESULT_CANCELED, vec![])],
        "a handshake send failure must route to the shared teardown and \
         report Canceled exactly once, not return past it",
    );
    assert!(
        registry
            .register(4304, 7, "Hack".into(), 1, 1, 0, 0, 0, 1, vec![])
            .await
            .is_some(),
        "the session must still be unregistered after a handshake failure",
    );
}
