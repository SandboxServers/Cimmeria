//! Tests for the `interaction.pose` row that `handle_interact` writes on an accepted interact.
//! Each test uses its own player id (9301 to 9304) because the pose throttle is process-global.

use super::handle_interact;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{make_space_manager, LogCapture};
use cimmeria_content_engine::{
    actions::Action,
    chain::{Chain, ChainEngine},
    triggers::Trigger,
};
use tokio::sync::mpsc;

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

/// Pins that an accepted interact with no content chain writes one `fallback` row at the true geometry.
#[tokio::test]
async fn accepted_interact_writes_one_fallback_row() {
    const PLAYER: u32 = 9301;
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

    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "interaction.pose")
        .collect();
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert_eq!(
        rows[0].fields.get("route").map(String::as_str),
        Some("fallback")
    );
    assert_eq!(
        rows[0].fields.get("kind").map(String::as_str),
        Some("interact")
    );
    let radius: f32 = rows[0].fields["radius_m"].parse().unwrap();
    let bearing: f32 = rows[0].fields["bearing_deg"].parse().unwrap();
    assert!((radius - 3.0).abs() < 0.01, "radius_m {radius}");
    assert!((bearing - 180.0).abs() < 0.1, "bearing_deg {bearing}");
}

/// Pins that an interact claimed by an `OnInteractTag` chain reports the `chain_tag` route.
#[tokio::test]
async fn chain_tag_interact_reports_chain_tag() {
    const PLAYER: u32 = 9302;
    let (mut mgr, npc) = scene(PLAYER, 3.0);
    mgr.get_entity_mut(npc).unwrap().tag = Some("PoseProbe".into());
    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 93020,
        name: "pose probe".into(),
        enabled: true,
        trigger: Trigger::OnInteractTag {
            entity_tag: "PoseProbe".into(),
        },
        conditions: vec![],
        actions: vec![Action::IncrementCounter {
            counter_name: "pose_probe".into(),
            amount: 1,
        }],
        action_delays: vec![],
        priority: 0,
        once: false,
    });
    let (tx, _rx) = mpsc::channel(16);
    let capture = LogCapture::install();

    handle_interact(PLAYER, &(npc as i32).to_le_bytes(), &tx, &mut mgr, &engine).await;

    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "interaction.pose")
        .collect();
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert_eq!(
        rows[0].fields.get("route").map(String::as_str),
        Some("chain_tag")
    );
}

/// Pins that a too-far interact is rejected before the capture, so it writes no pose row.
#[tokio::test]
async fn out_of_range_interact_writes_no_row() {
    const PLAYER: u32 = 9303;
    let (mut mgr, npc) = scene(PLAYER, 100.0);
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

    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "interaction.pose")
        .collect();
    assert!(rows.is_empty(), "{rows:#?}");
}

/// Pins that a right-click on a living hostile writes one `hostile_attack` pose row (F5).
/// The target is the only hostile in the scene, so no hostile-context row follows it.
#[tokio::test]
async fn hostile_right_click_emits_hostile_attack_pose() {
    const PLAYER: u32 = 9304;
    let (mut mgr, npc) = scene(PLAYER, 3.0);
    mgr.get_entity_mut(npc).unwrap().faction = crate::cell::combat::HOSTILE_FACTION;
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

    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "interaction.pose")
        .collect();
    let pose: Vec<_> = rows
        .iter()
        .filter(|c| c.fields.get("event").map(String::as_str) == Some("interaction_pose"))
        .collect();
    assert_eq!(pose.len(), 1, "{rows:#?}");
    assert_eq!(
        pose[0].fields.get("route").map(String::as_str),
        Some("hostile_attack")
    );
    let hostiles: Vec<_> = rows
        .iter()
        .filter(|c| c.fields.get("event").map(String::as_str) == Some("interaction_hostiles"))
        .collect();
    assert!(
        hostiles.is_empty(),
        "the only hostile is the target: {rows:#?}"
    );
}
