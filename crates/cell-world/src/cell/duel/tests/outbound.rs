//! `duel.send_failed`: a send the base channel refused still names who it
//! was for (instrumentation discipline rule 5). Simulated by dropping the
//! receiver, so every `tx.send` fails.

use std::time::Instant;

use tokio::sync::mpsc;
use tracing::Level;

use super::*;
use crate::test_support::LogCapture;

/// The challenge sends two things: the prompt to the target and the
/// acknowledgement to the challenger. Both fail; each failure row carries
/// the recipient's account, player and entity and the other duelist.
#[tokio::test]
async fn send_failure_logs_the_recipient_and_the_other_duelist() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr();
    let (tx, rx) = mpsc::channel(16);
    drop(rx);
    challenge(
        &mut mgr,
        &tx,
        (A_EID, A_PID),
        (B_EID, B_PID),
        Instant::now(),
    )
    .await;
    let failed: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.level == Level::WARN && c.has_field("event", "duel.send_failed"))
        .collect();
    assert_eq!(failed.len(), 2, "{failed:#?}");
    let prompt = failed
        .iter()
        .find(|c| c.has_field("method_index", "143"))
        .expect("prompt failure");
    for (k, v) in [
        ("account_id", "600"),
        ("player_id", "2000"),
        ("entity_id", "20"),
        ("target_player_id", "1000"),
        ("reason", "cell_to_base_closed"),
    ] {
        assert!(
            prompt.has_field(k, v),
            "prompt row lacks {k}={v}: {prompt:?}"
        );
    }
    let ack = failed
        .iter()
        .find(|c| c.has_field("method_index", "28"))
        .expect("acknowledgement failure");
    for (k, v) in [
        ("account_id", "500"),
        ("player_id", "1000"),
        ("entity_id", "10"),
        ("target_player_id", "2000"),
    ] {
        assert!(ack.has_field(k, v), "ack row lacks {k}={v}: {ack:?}");
    }
}
