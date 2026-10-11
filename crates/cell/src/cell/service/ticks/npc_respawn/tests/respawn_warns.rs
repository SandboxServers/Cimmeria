//! TG-NPC-12: the respawn tick logs the sends it used to drop and the
//! NPCs it cannot place, instead of skipping them silently.

use super::super::*;
use super::fixtures::make_mgr_with_dead_npc;
use crate::test_support::LogCapture;

/// A dropped receiver makes the loot-window close send fail; the failed
/// send is a WARN row carrying `reason = "base_channel_closed"`.
#[tokio::test]
async fn a_dropped_loot_close_send_logs_a_warn_row() {
    let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
    let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));
    if let Some(player) = mgr.get_entity_mut(1) {
        player.looting_entity = Some(50);
    }
    let (tx, rx) = mpsc::channel(64);
    drop(rx);
    let logs = LogCapture::install();

    npc_respawn_tick(&tx, &mut mgr).await;

    let all = logs.all();
    let row = all
        .iter()
        .find(|c| {
            c.level == tracing::Level::WARN
                && c.target == "spawner.npc_respawn"
                && c.fields.get("player_id").map(String::as_str) == Some("1")
        })
        .expect("a failed loot-close send must log a WARN row");
    assert!(
        row.has_field("event", "respawn_loot_close_failed"),
        "{row:?}"
    );
    assert!(row.has_field("reason", "base_channel_closed"), "{row:?}");
    assert!(row.has_field("npc_id", "50"), "{row:?}");
    assert!(row.fields.contains_key("error"), "{row:?}");
}

/// A dropped receiver makes the witness re-create send fail; the failed
/// send is a WARN row carrying `reason = "base_channel_closed"` and the
/// `error` field. Pins the shared cell→base reason name (a revert of the
/// rename fails here).
#[tokio::test]
async fn a_dropped_recreate_send_logs_a_warn_row() {
    let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
    let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));
    let (tx, rx) = mpsc::channel(64);
    drop(rx);
    let logs = LogCapture::install();

    npc_respawn_tick(&tx, &mut mgr).await;

    let all = logs.all();
    let row = all
        .iter()
        .find(|c| {
            c.level == tracing::Level::WARN
                && c.target == "spawner.npc_respawn"
                && c.fields.get("witness_id").map(String::as_str) == Some("1")
        })
        .expect("a failed witness re-create send must log a WARN row");
    assert!(row.has_field("reason", "base_channel_closed"), "{row:?}");
    assert!(row.has_field("npc_id", "50"), "{row:?}");
    assert!(row.fields.contains_key("error"), "{row:?}");
}

/// An NPC with no recorded spawn position still respawns, but the tick
/// says it could not put the NPC back on its spawn point.
#[tokio::test]
async fn a_no_position_npc_logs_a_respawn_warn_row() {
    let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
    let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));
    let space_id = mgr.get_entity(50).map(|n| n.space_id.0 as u32);
    if let Some(npc) = mgr.get_entity_mut(50) {
        npc.spawn_position = None;
        npc.template_id = Some(4242);
    }
    let (tx, _rx) = mpsc::channel(64);
    let logs = LogCapture::install();

    npc_respawn_tick(&tx, &mut mgr).await;

    let all = logs.all();
    let row = all
        .iter()
        .find(|c| c.fields.get("event").map(String::as_str) == Some("respawn_no_spawn_position"))
        .expect("a no-position respawn must log respawn_no_spawn_position");
    assert_eq!(row.level, tracing::Level::WARN, "{row:?}");
    assert_eq!(row.target, "spawner.npc_respawn", "{row:?}");
    assert!(row.has_field("npc_id", "50"), "{row:?}");
    assert!(row.has_field("template_id", "4242"), "{row:?}");
    let space_id = space_id.expect("fixture NPC is in a space");
    assert!(row.has_field("space_id", &space_id.to_string()), "{row:?}");
    assert!(row.has_field("world", "Castle"), "{row:?}");
}
