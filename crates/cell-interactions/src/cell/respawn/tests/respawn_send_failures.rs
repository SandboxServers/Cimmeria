//! The `onEndAidWait` send in `handle_respawn`: a closed base channel leaves
//! the Defeat Window open, so the unsent close is logged, not dropped.

use super::super::handle_respawn;
use super::make_mgr_with_player;
use tokio::sync::mpsc;

#[tokio::test]
async fn a_closed_base_channel_logs_the_unsent_end_aid_wait() {
    let capture = crate::test_support::LogCapture::install();
    let mut mgr = make_mgr_with_player("Castle_CellBlock");
    let (tx, rx) = mpsc::channel(16);
    drop(rx);

    handle_respawn(1, -1, &tx, &mut mgr).await;

    let ev = capture
        .find_message(tracing::Level::WARN, "onEndAidWait not sent")
        .expect("a closed base channel must log the unsent onEndAidWait");
    for (key, want) in [
        ("entity_id", "1"),
        ("player_id", "100"),
        ("event", "wire_send_failed"),
        ("method", "onEndAidWait"),
        ("reason", "base_channel_closed"),
    ] {
        assert!(
            ev.has_field(key, want),
            "expected {key}={want}; got {ev:#?}"
        );
    }
}
