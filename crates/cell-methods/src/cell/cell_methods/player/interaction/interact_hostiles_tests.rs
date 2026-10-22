//! Wiring tests for the `interaction_hostiles` row that `handle_interact` writes next to the
//! pose row, on both the normal path and the hostile right-click path (F5). The helper itself
//! is covered in `cimmeria-cell-world`; these pin that each call site invokes it.
//! Each test uses its own player id because the pose throttle is process-global.

use super::handle_interact;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{make_space_manager, LogCapture};
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

/// Builds an Agnos scene: a player at the origin, the interaction target at 3 m (faction
/// `target_faction`) and a second living faction-10 NPC at 6 m. Returns `(mgr, target, other)`.
fn scene(player: u32, target_faction: u8) -> (SpaceManager, u32, u32) {
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
    let target = mgr.allocate_npc_id();
    mgr.spawn_npc(target, "Agnos", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let t = mgr.get_entity_mut(target).unwrap();
    t.faction = target_faction;
    t.clear_all_state_flags();
    let other = mgr.allocate_npc_id();
    mgr.spawn_npc(other, "Agnos", [6.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let o = mgr.get_entity_mut(other).unwrap();
    o.faction = crate::cell::combat::HOSTILE_FACTION;
    o.clear_all_state_flags();
    (mgr, target, other)
}

/// Runs one interact and asserts the single hostiles row joins the pose row and lists `other`.
async fn assert_hostiles_row(player: u32, target_faction: u8, route: &str) {
    let (mut mgr, target, other) = scene(player, target_faction);
    let (tx, _rx) = mpsc::channel(16);
    let capture = LogCapture::install();

    handle_interact(
        player,
        &(target as i32).to_le_bytes(),
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;

    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "interaction.pose")
        .collect();
    let event = |name: &'static str| {
        rows.iter()
            .filter(|c| c.fields.get("event").map(String::as_str) == Some(name))
            .collect::<Vec<_>>()
    };
    let pose = event("interaction_pose");
    let hostiles = event("interaction_hostiles");
    assert_eq!(pose.len(), 1, "{rows:#?}");
    assert_eq!(pose[0].fields["route"], route, "{rows:#?}");
    assert_eq!(hostiles.len(), 1, "{rows:#?}");
    assert!(hostiles[0].has_field("count", "1"), "{rows:#?}");
    assert!(
        hostiles[0].has_field("hostile_1_id", &other.to_string()),
        "{rows:#?}"
    );
    assert!(
        !hostiles[0]
            .fields
            .values()
            .any(|v| *v == target.to_string()),
        "the target is not a nearby hostile: {rows:#?}"
    );
    assert_eq!(
        hostiles[0].fields["pose_seq"], pose[0].fields["pose_seq"],
        "{rows:#?}"
    );
}

/// Pins that the normal interact path writes the hostiles row (interact.rs normal-path call).
#[tokio::test]
async fn neutral_interact_writes_the_hostiles_row() {
    assert_hostiles_row(9305, 1, "fallback").await;
}

/// Pins that the hostile right-click path writes the hostiles row (interact.rs hostile call).
#[tokio::test]
async fn hostile_right_click_writes_the_hostiles_row() {
    assert_hostiles_row(9306, crate::cell::combat::HOSTILE_FACTION, "hostile_attack").await;
}
