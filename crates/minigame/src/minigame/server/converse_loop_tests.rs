//! Loop-level guard for ConverseBasicHumanoid (MG-CH3 review N1).
//!
//! Every CH3 unit test calls `tick()` by hand, and the other loop tests use
//! the `Hack` placeholder, which never needs a tick. This one runs a real
//! ConverseBasicHumanoid session through [`super::session_loop`]: the loop
//! must sample `needs_tick()` and drive the timers, or a silent player idles
//! to the 120 s cancel instead of losing on the first hand timer.

use tokio::net::TcpListener;
use tokio::time::Instant;

use super::result_dispatch::RESULT_DEFEAT;
use super::tests::drain_results;
use super::*;
use crate::minigame::game::create_game;

/// A ConverseBasicHumanoid session over a loopback pair, driven by
/// [`run_session`] with the given idle timeout. The client half is returned
/// so the caller can keep the socket open; the server sends its frames into
/// the kernel buffer and nothing reads them.
async fn spawn_converse_session(
    entity_id: u32,
    idle_timeout: Duration,
) -> (
    TcpStream,
    mpsc::Receiver<CellToBaseMsg>,
    tokio::task::JoinHandle<SessionEnd>,
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
            "ConverseBasicHumanoid".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![4242],
            vec![],
            None,
            Some(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        )
        .await
        .expect("fresh registry must accept the session");
    let session = registry
        .authenticate_and_claim(entity_id, &ticket, "ConverseBasicHumanoid")
        .await
        .expect("session must authenticate and claim");
    let game = create_game(&session).expect("ConverseBasicHumanoid instance");

    let (tx, rx) = mpsc::channel(16);
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
    (client, rx, handle)
}

/// A silent player makes no play, so the first hand timer (tick 24, 6.0 s)
/// ends the hand as a loss and the game fails. The session loop must tick
/// the game for that to happen. Stopping the loop from ticking (for example
/// `needs_tick` returning false) lets the 120 s idle deadline end it as a
/// cancel, outside the 5 to 10 s window, and this fails.
///
/// The paused clock is stepped in 250 ms increments, the game's own tick.
/// A bare await on the handle let the clock run ahead of the loop and
/// measured the end at about 14 s, so the steps keep the timing honest.
#[tokio::test(start_paused = true)]
async fn a_silent_converse_game_ends_on_the_hand_timer_in_the_loop() {
    let idle = Duration::from_secs(120);
    let (_client, mut rx, handle) = spawn_converse_session(4330, idle).await;
    // Taken after the helper: its loopback connect can move the paused clock.
    let start = Instant::now();

    // Bounded at 60 s: a loop that never ticks is still parked at 120 s, so
    // the step limit ends the wait and the assertions below report it.
    for _ in 0..240 {
        if handle.is_finished() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    let end = tokio::time::timeout(Duration::from_secs(3600), handle)
        .await
        .expect("the hand timer must end the game")
        .expect("session task must not panic");

    let elapsed = start.elapsed();
    assert_eq!(end.reason, EndReason::Defeat, "ended after {elapsed:?}");
    assert!(
        elapsed >= Duration::from_secs(5) && elapsed < Duration::from_secs(10),
        "the first hand timer runs at 6 s, ended after {elapsed:?}",
    );
    assert_eq!(drain_results(&mut rx), vec![(RESULT_DEFEAT, vec![])]);
}
