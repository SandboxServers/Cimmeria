//! The login claim: one connection per ticket.

use super::*;

const LOGIN_FAILED: &str = "<var n='_cmd' t='s'>loginFailed</var>";
const GAME_BEGIN: &str = "<var n='_cmd' t='s'>onGameBegin</var>";

/// Read until one of `markers` arrives or the socket closes, and return
/// everything received with NULs swapped for newlines.
async fn read_until_any(client: &mut TcpStream, markers: &[&str]) -> String {
    let mut seen = String::new();
    let mut chunk = vec![0u8; MAX_MESSAGE_LEN];
    let _ = tokio::time::timeout(OBSERVE_WITHIN, async {
        loop {
            match client.read(&mut chunk).await {
                Ok(0) | Err(_) => return,
                Ok(n) => {
                    seen.push_str(&String::from_utf8_lossy(&chunk[..n]));
                    if markers.iter().any(|m| seen.contains(m)) {
                        return;
                    }
                }
            }
        }
    })
    .await;
    seen.replace('\0', "\n")
}

/// One ticket, one game. A player who opens a second socket with their own
/// ticket while a round is in play must not get a second game instance —
/// each instance's victory would fire `on_victory_chains` again, paying the
/// reward twice. The original's cell dropped a second result; this server
/// has no such guard, so the refusal has to happen at login. The second
/// socket gets `loginFailed`, as the original sent for a rejected login.
#[tokio::test]
async fn a_second_login_with_a_ticket_in_play_is_refused() {
    let registry = SessionRegistry::new();
    let ticket = registry
        .register(4312, 7, "Hack".into(), 1, 1, 0, 0, 0, 1, vec![4242])
        .await
        .expect("fresh registry must accept the session");
    let (tx, mut rx) = mpsc::channel(16);

    let (mut first, server, peer) = loopback_pair().await;
    let first_task = spawn_connection(server, peer, registry.clone(), tx.clone(), OBSERVE_WITHIN);
    assert!(completes_version_check(&mut first).await, "first verChk");
    send_null_terminated(&mut first, &hack_login(4312, &ticket))
        .await
        .expect("first login send must succeed");
    read_until_game_begin(&mut first).await;

    // Same ticket, second socket, while the first round is live.
    let (mut second, server, peer) = loopback_pair().await;
    let second_task = spawn_connection(server, peer, registry.clone(), tx, OBSERVE_WITHIN);
    assert!(completes_version_check(&mut second).await, "second verChk");
    send_null_terminated(&mut second, &hack_login(4312, &ticket))
        .await
        .expect("second login send must succeed");
    let reply = read_until_any(&mut second, &[LOGIN_FAILED, GAME_BEGIN]).await;
    if reply.contains(GAME_BEGIN) {
        // Only reachable if the second login was wrongly admitted. Play out
        // the duplicate win so the payout count below shows the bug shape
        // rather than stopping at the login.
        let _ = send_null_terminated(&mut second, VICTORY).await;
    }
    let second_closed = server_hangs_up(&mut second, OBSERVE_WITHIN).await;
    join_within(second_task, "second connection").await;

    // The first connection still owns the session and can win.
    send_null_terminated(&mut first, VICTORY)
        .await
        .expect("first victory send must succeed");
    drop(first);
    join_within(first_task, "first connection").await;

    assert_eq!(
        drain_results(&mut rx),
        vec![(RESULT_VICTORY, vec![4242])],
        "two sockets on one ticket must yield exactly one victory — a second \
         would fire the victory chains again",
    );
    assert!(
        reply.contains(LOGIN_FAILED),
        "the refused login must be told loginFailed; got:\n{reply}",
    );
    assert!(
        second_closed,
        "the second socket must be closed after its login is refused"
    );
}
