//! Loop-level guards for GoauldCrystals: the ticker from the first click, and
//! a result whose frame cannot be sent. These run the real session loop
//! against the game, not the placeholder, so the deadline and the send path
//! are the production ones.

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

use super::result_dispatch::RESULT_DEFEAT;
use super::tests::{drain_redeemed, read_until_game_begin};
use super::*;
use crate::minigame::games::create_unported_for_test;
use crate::test_support::LogCapture;

/// One `playfieldclick` frame, as the SWF sends it.
fn playfield_click_frame(x: u32, y: u32) -> String {
    format!(
        "<msg t='xt'><body action='xtReq'>\
         <![CDATA[<dataObj><var n='cmd' t='s'>playfieldclick</var><obj o='param' t='a'>\
         <var n='x' t='n'>{x}</var><var n='y' t='n'>{y}</var></obj></dataObj>]]>\
         </body></msg>"
    )
}

/// A loopback GoauldCrystals session at difficulty 2, seed 1. Shaped like
/// `spawn_placeholder_session_with_idle`, with the real game in place of the
/// placeholder. Returns `(client_half, result_rx, join_handle)`.
async fn spawn_goauld_session(
    entity_id: u32,
    idle_timeout: Duration,
) -> (
    TcpStream,
    mpsc::Receiver<CellToBaseMsg>,
    tokio::task::JoinHandle<SessionEnd>,
    SessionRegistry,
) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let addr = listener.local_addr().expect("local_addr");
    let client = TcpStream::connect(addr).await.expect("connect loopback");
    let (server, _) = listener.accept().await.expect("accept loopback");

    let registry = SessionRegistry::new();
    let ticket = registry
        .register(
            entity_id,
            7,
            "GoauldCrystals".into(),
            2,
            1,
            1,
            0,
            0,
            1,
            vec![4242],
            vec![],
            None,
            Some(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
            1,
        )
        .await
        .expect("fresh registry must accept the session");
    let session = registry
        .authenticate_and_claim(entity_id, &ticket, "GoauldCrystals")
        .await
        .expect("session must authenticate and claim");
    let game = create_unported_for_test(&session).expect("GoauldCrystals instance");

    let (tx, rx) = mpsc::channel(16);
    let kept = registry.clone();
    let handle = tokio::spawn(async move {
        run_session(
            server,
            &registry,
            &tx,
            session,
            game,
            ReadState::empty(),
            idle_timeout,
        )
        .await
    });
    (client, rx, handle, kept)
}

/// The session loop asks `needs_tick()` once, before the first click, so a
/// silent GoauldCrystals game must have its ticker from the start. The click
/// starts the clock and nothing else is sent: the 16 s deadline must end the
/// game as a defeat. Reverting `needs_tick` to wait for a running clock leaves
/// no ticker, the session only ends at the 600 s idle timeout, and the 60 s
/// bound fails.
#[tokio::test(start_paused = true)]
async fn a_silent_goauld_game_ends_in_defeat_at_its_deadline() {
    let capture = LogCapture::install();
    let (mut client, mut rx, handle, registry) =
        spawn_goauld_session(4330, Duration::from_secs(600)).await;
    send_null_terminated(&mut client, &playfield_click_frame(0, 0))
        .await
        .expect("send");

    let end = tokio::time::timeout(Duration::from_secs(60), handle)
        .await
        .unwrap_or_else(|_| {
            panic!(
                "the deadline must end a silent game within 60 s: {:#?}",
                capture.all()
            )
        })
        .expect("session task must not panic");

    assert_eq!(end.reason, EndReason::Defeat);
    let kinds: Vec<u8> = drain_redeemed(&mut rx, &registry)
        .await
        .iter()
        .map(|result| result.0)
        .collect();
    assert_eq!(kinds, vec![RESULT_DEFEAT]);
}

/// A failed send of the failure frame must not cost the result. The client
/// resets its socket after the click, so the deadline's failure frame is the
/// server's first write to a dead peer. The failed send is recorded and the
/// `Failure` that follows still runs, so the game reports Defeat once and
/// never Canceled. Breaking out of the batch on the send error leaves the
/// result unreported and fails this.
///
/// The runtime is single-threaded and paused. Real sleeps and `yield_now`
/// keep the clock from moving while the click is read and the reset lands,
/// then the advance makes the deadline due while the read arm is also ready,
/// so the biased select runs the tick first.
#[tokio::test(start_paused = true)]
async fn a_failed_failure_frame_still_reports_defeat() {
    let (mut client, mut rx, handle, registry) =
        spawn_goauld_session(4331, Duration::from_secs(600)).await;
    read_until_game_begin(&mut client).await;
    send_null_terminated(&mut client, &playfield_click_frame(0, 0))
        .await
        .expect("send");
    for _ in 0..100 {
        std::thread::sleep(Duration::from_millis(2));
        tokio::task::yield_now().await;
    }

    // Linger 0 makes close() send RST, so the next server write fails.
    socket2::SockRef::from(&client)
        .set_linger(Some(Duration::ZERO))
        .expect("SO_LINGER must be settable on a loopback TCP socket");
    drop(client);
    std::thread::sleep(Duration::from_millis(50));
    tokio::time::advance(Duration::from_secs(17)).await;

    let end = tokio::time::timeout(Duration::from_secs(60), handle)
        .await
        .expect("the failed send must not hang the session")
        .expect("session task must not panic");

    assert_eq!(
        end.reason,
        EndReason::Defeat,
        "a failed failure frame must still end the game as a defeat",
    );
    let kinds: Vec<u8> = drain_redeemed(&mut rx, &registry)
        .await
        .iter()
        .map(|result| result.0)
        .collect();
    assert_eq!(
        kinds,
        vec![RESULT_DEFEAT],
        "the result must be reported once, as a defeat, not Canceled",
    );
}
