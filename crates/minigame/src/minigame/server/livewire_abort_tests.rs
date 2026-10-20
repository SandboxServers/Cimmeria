//! Livewire abort through the real session loop (MG-LW1 delta 14): a client that
//! closes mid-game gets one `RESULT_CANCELED` and no `failure` frame.

use tokio::net::TcpListener;

use super::tests::{drain_results, read_until_game_begin};
use super::*;
use crate::minigame::game::create_game;

/// Close only the client's write half, then read the server's frames to EOF.
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
            Ok(Ok(n)) if n > 0 => out.extend_from_slice(&chunk[..n]),
            _ => break,
        }
    }
    String::from_utf8_lossy(&out).replace('\0', "\n")
}

/// A Livewire game closed by its client reports `RESULT_CANCELED` exactly once,
/// and `LivewireGame::aborted` sends no `failure` frame. Reverting `aborted` to
/// send one, or dropping the teardown's result, fails this.
#[tokio::test]
async fn livewire_abort_reports_canceled_with_no_failure_frame() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let addr = listener.local_addr().expect("local_addr");
    let mut client = TcpStream::connect(addr).await.expect("connect loopback");
    let (server, _) = listener.accept().await.expect("accept loopback");

    let registry = SessionRegistry::new();
    let ticket = registry
        .register(
            4305,
            7,
            "Livewire".into(),
            1,
            30,
            17,
            0,
            0,
            1,
            vec![],
            vec![],
            None,
            Some(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        )
        .await
        .expect("fresh registry must accept the session");
    let session = registry
        .authenticate_and_claim(4305, &ticket, "Livewire")
        .await
        .expect("session must authenticate and claim");
    let game = create_game(&session).expect("livewire instance");

    let (tx, mut rx) = mpsc::channel(16);
    let handle = tokio::spawn(async move {
        run_session(
            server,
            &registry,
            &tx,
            session,
            game,
            ReadState::empty(),
            Duration::from_secs(30),
        )
        .await
    });

    // Wait for the game loop, then close the client's side mid-game.
    read_until_game_begin(&mut client).await;
    let stream = half_close_and_read_to_eof(client).await;
    handle.await.expect("session task must not panic");

    assert_eq!(
        drain_results(&mut rx),
        vec![(RESULT_CANCELED, vec![])],
        "an abandoned Livewire game must report Canceled exactly once",
    );
    assert_eq!(
        stream.matches("<var n='_cmd' t='s'>failure</var>").count(),
        0,
        "the Livewire abort must send no failure frame. Captured stream: {stream}",
    );
}
