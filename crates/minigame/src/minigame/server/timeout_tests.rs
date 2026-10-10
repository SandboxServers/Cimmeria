//! Guards for the time-based limits: the idle deadline's reset, the login
//! deadline against a trickling peer, the per-send timeout, and the log
//! throttle's suppression count, plus TCP keepalive on accepted sockets.
//!
//! The socket tests run on tokio's paused clock, which jumps to the next
//! timer whenever every task is waiting. Each wraps its wait in an outer
//! guard far longer than the deadline under test, so a missing deadline
//! fails the test instead of hanging it.

use tokio::io::AsyncWriteExt;
use tokio::time::Instant;

use super::accept::{enable_keepalive, LogThrottle};
use super::listener_tests::{loopback_pair, spawn_connection};
use super::tests::{drain_results, spawn_placeholder_session_with_idle};
use super::*;
use crate::test_support::{LogCapture, LogCaptureGuard};

/// Let the server task read what the client just wrote. Under the paused
/// clock, waiting on a timer could let time jump past a server deadline
/// before the kernel reports the bytes, so this polls in real time
/// (`yield_now` lets the runtime check the socket without advancing the
/// clock) until the server's own log row shows the frame was handled.
async fn wait_for_row(capture: &LogCaptureGuard, message: &str) {
    for _ in 0..500 {
        if capture
            .find_message(tracing::Level::DEBUG, message)
            .is_some()
        {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
        tokio::task::yield_now().await;
    }
    panic!("the server never logged {message:?}: {:#?}", capture.all());
}

/// Every inbound frame pushes the idle deadline back. A frame at 50 s of a
/// 60 s idle timeout keeps the session alive at 100 s. Deleting the
/// `reset` call ends it at 60 s and fails this.
#[tokio::test(start_paused = true)]
async fn an_inbound_frame_resets_the_idle_deadline() {
    let capture = LogCapture::install();
    let idle = Duration::from_secs(60);
    let start = Instant::now();
    let (mut client, mut rx, handle) = spawn_placeholder_session_with_idle(4320, idle).await;

    tokio::time::sleep_until(start + Duration::from_secs(50)).await;
    // `PlaceholderGame` ignores unknown commands, at DEBUG.
    let noop = "<msg t='xt'><body action='xtReq'>\
         <![CDATA[<dataObj><var n='cmd' t='s'>noop</var></dataObj>]]>\
         </body></msg>";
    send_null_terminated(&mut client, noop).await.expect("send");
    wait_for_row(&capture, "ignoring unknown command").await;

    tokio::time::sleep_until(start + Duration::from_secs(100)).await;
    assert!(
        !handle.is_finished(),
        "a frame at 50 s must keep a 60 s idle session alive at 100 s; \
         results so far: {:?}",
        drain_results(&mut rx),
    );

    let victory = "<msg t='xt'><body action='xtReq'>\
         <![CDATA[<dataObj><var n='cmd' t='s'>victory</var></dataObj>]]>\
         </body></msg>";
    send_null_terminated(&mut client, victory)
        .await
        .expect("send");
    tokio::time::timeout(Duration::from_secs(3600), handle)
        .await
        .expect("the session must end on victory")
        .expect("session task must not panic");
    assert_eq!(drain_results(&mut rx), vec![(RESULT_VICTORY, vec![4242])]);
}

/// The login deadline covers the whole handshake, not each read: a peer
/// that sends one byte every 5 s (never a terminator) is still closed at
/// 30 s. A per-read timeout would keep it open indefinitely.
#[tokio::test(start_paused = true)]
async fn a_trickling_peer_is_cut_at_the_handshake_deadline() {
    let limits = ListenerLimits::default();
    let deadline = limits.handshake_timeout;
    let start = Instant::now();
    let (mut client, _rx, handle) = spawn_connection(limits).await;

    let mut tick = 0u64;
    while !handle.is_finished() && tick < 60 {
        tick += 1;
        tokio::time::sleep_until(start + Duration::from_secs(5 * tick)).await;
        // After the server closes, writes may fail; that is the point.
        let _ = client.write_all(b"A").await;
        // Real-time pause so the byte reaches the server before the next
        // clock jump (see `wait_for_row`).
        std::thread::sleep(std::time::Duration::from_millis(5));
        tokio::task::yield_now().await;
    }
    assert!(
        handle.is_finished(),
        "a peer trickling a byte every 5 s must still be cut at the {deadline:?} deadline",
    );
    let closed_at = start.elapsed();
    assert!(
        closed_at >= deadline && closed_at <= deadline + Duration::from_secs(5),
        "closed at {closed_at:?}, expected at the {deadline:?} deadline",
    );
}

/// One deadline covers both phases. A peer that completes `verChk` at 25 s
/// and then trickles its login is still closed at 30 s from accept, not
/// 30 s after `verChk` (55 s), which a separate deadline per phase would
/// allow.
#[tokio::test(start_paused = true)]
async fn the_handshake_deadline_spans_both_phases() {
    let limits = ListenerLimits::default();
    let deadline = limits.handshake_timeout;
    let start = Instant::now();
    let (mut client, _rx, handle) = spawn_connection(limits).await;

    tokio::time::sleep_until(start + Duration::from_secs(25)).await;
    send_null_terminated(
        &mut client,
        "<msg t='sys'><body action='verChk' r='0'><ver v='154'/></body></msg>",
    )
    .await
    .expect("verChk send");
    // The policy reply goes out only once verChk is handled. Wait for it in
    // real time with a non-blocking read, which arms no timer, so the
    // paused clock stays at 25 s (see `wait_for_row`).
    let mut reply = [0u8; 512];
    let mut answered = false;
    for _ in 0..500 {
        if matches!(client.try_read(&mut reply), Ok(n) if n > 0) {
            answered = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
        tokio::task::yield_now().await;
    }
    assert!(answered, "the server never answered verChk");
    assert!(start.elapsed() < deadline, "verChk was handled late");

    let mut tick = 0u64;
    while !handle.is_finished() && tick < 20 {
        tick += 1;
        tokio::time::sleep_until(start + Duration::from_secs(25 + 2 * tick)).await;
        // Login bytes, never a terminator.
        let _ = client.write_all(b"<").await;
        std::thread::sleep(std::time::Duration::from_millis(5));
        tokio::task::yield_now().await;
    }
    assert!(handle.is_finished(), "the trickled login was never cut");
    let closed_at = start.elapsed();
    assert!(
        closed_at >= deadline && closed_at <= deadline + Duration::from_secs(2),
        "closed at {closed_at:?}; the deadline runs {deadline:?} from accept, not from verChk",
    );
}

/// A send that cannot finish (the peer never reads, so both socket buffers
/// fill) fails after `SEND_TIMEOUT` instead of blocking forever.
#[tokio::test(start_paused = true)]
async fn a_blocked_send_fails_after_the_send_timeout() {
    let (_client_never_reads, mut server, _peer) = loopback_pair().await;
    let frame = "A".repeat(64 * 1024);
    let start = Instant::now();

    let outcome = tokio::time::timeout(Duration::from_secs(3600), async {
        // Far more than any loopback buffer holds.
        for _ in 0..20_000 {
            if send_null_terminated(&mut server, &frame).await.is_err() {
                return true;
            }
        }
        false
    })
    .await
    .expect("a blocked send must give up at SEND_TIMEOUT, not hang");

    assert!(outcome, "the buffers never filled, so nothing was tested");
    assert!(
        start.elapsed() >= framing::SEND_TIMEOUT,
        "gave up after {:?}",
        start.elapsed()
    );
}

/// The throttle writes the first row, counts the rest of the window, and
/// reports that count on the next row.
#[test]
fn log_throttle_reports_the_suppressed_count() {
    let t0 = Instant::now();
    let mut throttle = LogThrottle::new(Duration::from_secs(60));
    assert_eq!(throttle.admit(t0), Some(0));
    assert_eq!(throttle.admit(t0 + Duration::from_secs(1)), None);
    assert_eq!(throttle.admit(t0 + Duration::from_secs(59)), None);
    assert_eq!(throttle.admit(t0 + Duration::from_secs(61)), Some(2));
    assert_eq!(throttle.admit(t0 + Duration::from_secs(62)), None);
    assert_eq!(throttle.admit(t0 + Duration::from_secs(200)), Some(1));
}

/// Accepted sockets get TCP keepalive, so a peer that vanished without a FIN
/// is dropped by the OS instead of holding its session until the idle
/// timeout.
#[tokio::test]
async fn accepted_sockets_get_tcp_keepalive() {
    let (_client, server, peer) = loopback_pair().await;
    let before = socket2::SockRef::from(&server)
        .keepalive()
        .expect("SO_KEEPALIVE readable");
    assert!(!before, "keepalive starts off, or this proves nothing");
    enable_keepalive(&server, peer);
    assert!(socket2::SockRef::from(&server)
        .keepalive()
        .expect("SO_KEEPALIVE readable"));
}
