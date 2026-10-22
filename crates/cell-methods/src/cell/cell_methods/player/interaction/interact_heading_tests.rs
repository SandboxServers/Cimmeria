//! Tests for the `heading_age_ms` field on the `"interact"` INFO row.
//! The age is how long ago the cell processed the player's last position update.

use super::{handle_interact, heading_age_ms};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{make_space_manager, LogCapture};
use cimmeria_content_engine::chain::ChainEngine;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::Level;

/// Builds an Agnos scene: a player at the origin and a neutral NPC at `npc_x` on the x axis.
fn scene(player: u32, npc_x: f32) -> (SpaceManager, u32) {
    let mut mgr = make_space_manager();
    mgr.create_entity(player, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    {
        let actor = mgr.get_entity_mut(player).unwrap();
        actor.is_player = true;
        actor.player_id = Some(12);
        actor.account_id = Some(6);
        actor.session_kind = Some("player");
        actor.clear_all_state_flags();
    }
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc(npc, "Agnos", [npc_x, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let target = mgr.get_entity_mut(npc).unwrap();
    target.faction = 1;
    target.clear_all_state_flags();
    (mgr, npc)
}

/// Pins that no recorded position update gives the sentinel `-1`.
#[test]
fn heading_age_is_minus_one_without_a_position_update() {
    assert_eq!(heading_age_ms(Instant::now(), None), -1);
}

/// Pins that the age is the elapsed milliseconds since the recorded update.
#[test]
fn heading_age_counts_elapsed_milliseconds() {
    let now = Instant::now();
    let age = heading_age_ms(now, Some(now - Duration::from_millis(1500)));
    assert!((1500..=1550).contains(&age), "age {age}");
}

/// Pins that the `"interact"` INFO row carries the age of the player's last position update.
#[tokio::test]
async fn interact_row_reports_heading_age() {
    const PLAYER: u32 = 9401;
    let (mut mgr, npc) = scene(PLAYER, 3.0);
    mgr.movement_validator
        .touch_clock(PLAYER, Instant::now() - Duration::from_millis(1500));
    let (tx, _rx) = mpsc::channel(16);
    let capture = LogCapture::install();

    handle_interact(
        PLAYER,
        &(npc as i32).to_le_bytes(),
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;

    let row = capture
        .all()
        .into_iter()
        .find(|c| c.level == Level::INFO && c.message.as_deref() == Some("interact"))
        .expect("interact INFO row");
    let age: i64 = row.fields["heading_age_ms"].parse().unwrap();
    assert!((1500..=5000).contains(&age), "heading_age_ms {age}");
}

/// Pins that an interact with no prior position update reports `-1` on the row.
#[tokio::test]
async fn interact_row_reports_minus_one_without_position_update() {
    const PLAYER: u32 = 9402;
    let (mut mgr, npc) = scene(PLAYER, 3.0);
    let (tx, _rx) = mpsc::channel(16);
    let capture = LogCapture::install();

    handle_interact(
        PLAYER,
        &(npc as i32).to_le_bytes(),
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;

    let row = capture
        .all()
        .into_iter()
        .find(|c| c.level == Level::INFO && c.message.as_deref() == Some("interact"))
        .expect("interact INFO row");
    assert_eq!(row.fields.get("heading_age_ms").map(String::as_str), Some("-1"));
}
