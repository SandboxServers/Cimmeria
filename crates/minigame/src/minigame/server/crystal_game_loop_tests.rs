//! CrystalGame's clock through the real session loop (MG-CG3): one click
//! starts the 16 s clock, and with no further input the 250 ms tick interval
//! alone ends the game as a defeat that reports its failure chains.

use tokio::net::TcpListener;

use super::result_dispatch::RESULT_DEFEAT;
use super::tests::read_until_game_begin;
use super::*;
use crate::minigame::game::create_game;

/// One `processmove` at (0, 0): the parameters sit in an `obj` of type `a`,
/// as the parser in `protocol.rs` reads them.
const FIRST_CLICK: &str = "<msg t='xt'><body action='xtReq'>\
     <![CDATA[<dataObj><var n='cmd' t='s'>processmove</var>\
     <obj o='param' t='a'><var n='x' t='n'>0</var><var n='y' t='n'>0</var></obj>\
     </dataObj>]]></body></msg>";

/// Read the client's side until the server closes it, and return the frames.
/// Only called after the session has returned, which drops the socket, so no
/// timeout is needed; a timer here would race the paused clock.
async fn read_to_eof(mut client: TcpStream) -> String {
    let mut out = Vec::new();
    let mut chunk = vec![0u8; MAX_MESSAGE_LEN];
    loop {
        match tokio::io::AsyncReadExt::read(&mut client, &mut chunk).await {
            Ok(n) if n > 0 => out.extend_from_slice(&chunk[..n]),
            _ => break,
        }
    }
    String::from_utf8_lossy(&out).replace('\0', "\n")
}

/// The first click starts the clock and nothing else ends the game: the 64
/// ticks of the 250 ms interval (16 s) lose it, and that defeat reports
/// `RESULT_DEFEAT` with the failure chains and sends one `failure` frame.
///
/// The socket handshake and the click run on real time, because paused time
/// lets the runtime skip ahead of loopback socket reads (the 30 min idle timer
/// then ends the session first). The clock is paused only once the click's
/// `timerupdate` has arrived; the std `Instant` wall deadline is real time and
/// is far from its 16 s, so the ticks are what end the game. Making
/// `needs_tick` false while the clock is off leaves no tick and fails this.
#[tokio::test]
async fn the_tick_interval_alone_loses_a_clock_that_was_started() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let addr = listener.local_addr().expect("local_addr");
    let mut client = TcpStream::connect(addr).await.expect("connect loopback");
    let (server, _) = listener.accept().await.expect("accept loopback");

    let registry = SessionRegistry::new();
    let ticket = registry
        .register(
            4306,
            7,
            "CrystalGame".into(),
            1,
            30,
            17,
            0,
            0,
            1,
            vec![],
            vec![4243],
            None,
            Some(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        )
        .await
        .expect("fresh registry must accept the session");
    let session = registry
        .authenticate_and_claim(4306, &ticket, "CrystalGame")
        .await
        .expect("session must authenticate and claim");
    let game = create_game(&session).expect("CrystalGame instance");

    let (tx, mut rx) = mpsc::channel(16);
    let handle = tokio::spawn(async move {
        run_session(
            server,
            &registry,
            &tx,
            session,
            game,
            ReadState::empty(),
            Duration::from_secs(1800),
        )
        .await
    });

    // Start the clock with one click, and wait for its `timerupdate` so the
    // server has processed the click before time is paused and advanced.
    read_until_game_begin(&mut client).await;
    send_null_terminated(&mut client, FIRST_CLICK)
        .await
        .expect("click send must succeed");
    let mut seen = String::new();
    let mut chunk = vec![0u8; MAX_MESSAGE_LEN];
    while !seen.contains("<var n='_cmd' t='s'>timerupdate</var>") {
        let n = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::io::AsyncReadExt::read(&mut client, &mut chunk),
        )
        .await
        .unwrap_or_else(|_| panic!("timerupdate must arrive after the click: {seen}"))
        .expect("read after click");
        assert!(n > 0, "the session closed before the clock started: {seen}");
        seen.push_str(&String::from_utf8_lossy(&chunk[..n]));
    }

    // From here on time is virtual. 17 s covers the 64 ticks of the first 16 s.
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(17)).await;
    handle.await.expect("session task must not panic");

    // `drain_results` keeps only the victory chains, so read the failure chains
    // from the message itself.
    let mut results = Vec::new();
    while let Ok(CellToBaseMsg::MinigameResult {
        result_code,
        on_failure_chains,
        ..
    }) = rx.try_recv()
    {
        results.push((result_code, on_failure_chains));
    }
    assert_eq!(
        results,
        vec![(RESULT_DEFEAT, vec![4243])],
        "the clock running out must report one defeat with the failure chains",
    );
    let stream = read_to_eof(client).await;
    assert_eq!(
        stream.matches("<var n='_cmd' t='s'>failure</var>").count(),
        1,
        "the timeout must send exactly one failure frame. Captured stream: {stream}",
    );
}
