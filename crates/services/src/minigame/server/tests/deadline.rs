//! The handshake deadline: a connection must finish verChk + login in time,
//! a trickle cannot extend it, and it does not reach into the game loop.

use super::*;

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
    let task = spawn_connection(server, peer, SessionRegistry::new(), tx, SHORT_DEADLINE);

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
    join_within(task, "silent client").await;
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
    let task = spawn_connection(server, peer, SessionRegistry::new(), tx, SHORT_DEADLINE);

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
    join_within(task, "trickling client").await;
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
    let task = spawn_connection(server, peer, registry, tx, SHORT_DEADLINE);

    assert!(
        completes_version_check(&mut client).await,
        "verChk must be served"
    );
    send_null_terminated(&mut client, &hack_login(4311, &ticket))
        .await
        .expect("login send must succeed");
    read_until_game_begin(&mut client).await;

    // Idle well past the handshake deadline, then win.
    tokio::time::sleep(SHORT_DEADLINE * 3).await;
    send_null_terminated(&mut client, VICTORY)
        .await
        .expect("victory send must succeed");
    drop(client);
    join_within(task, "session idle past the deadline").await;

    assert_eq!(
        drain_results(&mut rx),
        vec![(RESULT_VICTORY, vec![4242])],
        "a victory sent after the handshake deadline must be reported once, \
         with its chains — the deadline must not reach into the game loop",
    );
}
