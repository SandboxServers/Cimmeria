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
    // Go through `authenticate` + `mark_connected` rather than building a
    // `MinigameSession` by hand, so the test walks the same path the login
    // handler does.
    let session = registry
        .authenticate(entity_id, &ticket, "Hack")
        .await
        .expect("session must authenticate");
    registry.mark_connected(entity_id).await;
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

/// Read whatever the server has queued, then drop the socket. Draining
/// first keeps the server's writes from ever blocking.
async fn drain_and_close(mut client: TcpStream) {
    read_once(&mut client).await;
    drop(client);
}

/// One bounded read. The whole handshake is a few hundred bytes and arrives
/// coalesced, so a single read is enough to unblock the server; anything
/// left over is discarded when the socket drops.
async fn read_once(client: &mut TcpStream) {
    let mut sink = vec![0u8; MAX_MESSAGE_LEN];
    let _ = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::io::AsyncReadExt::read(client, &mut sink),
    )
    .await;
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
/// socket mid-game must produce exactly one upstream result, coded
/// `RESULT_CANCELED`.
///
/// Fails with the `if !result_reported` block removed from `run_session`:
/// no result is dispatched at all, which is the behaviour that left
/// `MinigameInstance::aborted` with zero call sites in the tree.
#[tokio::test]
async fn closing_the_swf_reports_canceled_not_defeat() {
    let (client, mut rx, handle) = spawn_placeholder_session(4301).await;
    drain_and_close(client).await;
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
}

/// The abort report must not fire when the game produced a real outcome.
/// Without the `result_reported` flag the teardown would append a bogus
/// `RESULT_CANCELED` after every victory, and that second message would be
/// indistinguishable from a real cancel upstream.
#[tokio::test]
async fn a_won_game_reports_victory_once_and_never_canceled() {
    let (mut client, mut rx, handle) = spawn_placeholder_session(4302).await;

    // Let the handshake land, then win. `PlaceholderGame` treats the
    // `victory` extension command as an instant win.
    read_once(&mut client).await;
    let victory = "<msg t='xt'><body action='xtReq'>\
         <![CDATA[<dataObj><var n='cmd' t='s'>victory</var></dataObj>]]>\
         </body></msg>";
    send_null_terminated(&mut client, victory)
        .await
        .expect("victory send must succeed");
    drain_and_close(client).await;
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
/// `registry.remove` covers one that connected and then went away. Fails
/// with that `remove` deleted, and fails if any `run_session` exit path is
/// ever allowed to return past it — which is exactly what the four
/// handshake sends used to do before they were moved behind `run_session`.
#[tokio::test]
async fn a_closed_connection_leaves_the_entity_free_to_relaunch() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let addr = listener.local_addr().expect("local_addr");
    let mut client = TcpStream::connect(addr).await.expect("connect loopback");
    let (server, _) = listener.accept().await.expect("accept loopback");

    let registry = SessionRegistry::new();
    let ticket = registry
        .register(4303, 7, "Hack".into(), 1, 1, 0, 0, 0, 1, vec![])
        .await
        .expect("fresh registry must accept the session");

    let (tx, _rx) = mpsc::channel(16);
    let reg = registry.clone();
    let handle = tokio::spawn(handle_connection(server, reg, tx, 9339));

    // Phase 1 — verChk. The server answers with the cross-domain policy
    // and apiOK; 154 is the version the original SWFs were built against.
    send_null_terminated(
        &mut client,
        "<msg t='sys'><body action='verChk' r='0'><ver v='154'/></body></msg>",
    )
    .await
    .expect("verChk send must succeed");
    read_once(&mut client).await;

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
