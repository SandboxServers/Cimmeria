use super::super::*; // gm module: dispatch + GM_* constants
use super::*; // shared helpers from tests/mod.rs
use cimmeria_common::Vector3;
use tokio::sync::mpsc;

#[tokio::test]
async fn show_target_location_reports_subject_position() {
    let mut mgr = mgr_with_player(1, "Castle");
    // Target an NPC at a known position; gmShowTargetLocation has no args and
    // uses the caller's current target.
    mgr.create_entity(2, "Castle", [12.0, 3.0, -4.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(1).unwrap().current_target_id = Some(2);
    let (tx, mut rx) = mpsc::channel(8);

    assert!(dispatch(1, GM_SHOW_TARGET_LOCATION, &[], &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("must feed back");
    assert!(
        fb.contains("[2]") && fb.contains("12.00") && fb.contains("-4.00"),
        "got: {fb}"
    );
}

#[tokio::test]
async fn show_target_location_falls_back_to_self() {
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.get_entity_mut(1).unwrap().position = Vector3 {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };
    let (tx, mut rx) = mpsc::channel(8);
    // No current target → reports the caller's own location.
    assert!(dispatch(1, GM_SHOW_TARGET_LOCATION, &[], &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("must feed back");
    assert!(fb.contains("[1]"), "no-target must report self, got: {fb}");
}

#[tokio::test]
async fn show_rotation_reports_heading() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    assert!(dispatch(1, GM_SHOW_ROTATION, &[], &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("must feed back");
    assert!(fb.contains("heading") && fb.contains("[1]"), "got: {fb}");
}

#[tokio::test]
async fn show_player_dumps_entity_info() {
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(2) {
        e.faction = 7;
        e.level = 30;
    }
    let (tx, mut rx) = mpsc::channel(8);
    assert!(dispatch(1, GM_SHOW_PLAYER, &2i32.to_le_bytes(), &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("must feed back");
    assert!(
        fb.contains("[2]")
            && fb.contains("faction 7")
            && fb.contains("lvl 30")
            && fb.contains("hp"),
        "got: {fb}"
    );
}

#[tokio::test]
async fn show_player_cross_space_and_missing_report_errors() {
    let mut mgr = mgr_with_player(1, "Castle");
    // An entity in a *different* space — gmShowPlayer must refuse it.
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Other" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Other" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(2, "Other", [0.0; 3], [0.0; 3]).unwrap();
    let (tx, mut rx) = mpsc::channel(8);

    // Cross-space target → "different space".
    assert!(dispatch(1, GM_SHOW_PLAYER, &2i32.to_le_bytes(), &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("must feed back");
    assert!(
        fb.contains("different space"),
        "cross-space must be refused, got: {fb}"
    );

    // Nonexistent id → "no such entity".
    assert!(dispatch(1, GM_SHOW_PLAYER, &4242i32.to_le_bytes(), &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("must feed back");
    assert!(fb.contains("no such entity"), "got: {fb}");
}

#[tokio::test]
async fn gm_users_lists_players_in_space() {
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.connect_entity(1); // all_player_entity_ids reads the connected set
    let (tx, mut rx) = mpsc::channel(8);

    assert!(dispatch(1, GM_USERS, &[], &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("gmUsers must feed back to the caller");
    assert!(
        fb.contains("gmUsers"),
        "feedback should be a gmUsers line, got: {fb}"
    );
    assert!(
        fb.contains('1'),
        "the connected player's entity id should appear, got: {fb}"
    );
}

#[tokio::test]
async fn test_los_reports_via_feedback() {
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.create_entity(2, "Castle", [5.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let (tx, mut rx) = mpsc::channel(8);

    // Both entities are in the caller's space; no navmesh is loaded in the
    // fixture, so has_line_of_sight conservatively reports CLEAR.
    let mut args = 1i32.to_le_bytes().to_vec();
    args.extend_from_slice(&2i32.to_le_bytes());
    assert!(dispatch(1, TEST_LOS, &args, &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("testLOS must feed back to the caller");
    assert!(
        fb.contains("testLOS") && fb.contains("CLEAR"),
        "expected a CLEAR verdict line, got: {fb}"
    );
}

#[tokio::test]
async fn test_los_truncated_and_missing_target_feed_back() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    // Truncated (one id) → "need two" feedback, no panic.
    assert!(dispatch(1, TEST_LOS, &1i32.to_le_bytes(), &tx, &mut mgr).await);
    assert!(
        feedback_text(&drain(&mut rx), 1).is_some(),
        "truncated testLOS still feeds back"
    );

    // Well-formed but target not in space → "not found" feedback.
    let mut args = 1i32.to_le_bytes().to_vec();
    args.extend_from_slice(&4242i32.to_le_bytes());
    assert!(dispatch(1, TEST_LOS, &args, &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("missing-target testLOS feeds back");
    assert!(
        fb.contains("not found"),
        "missing target should report not found, got: {fb}"
    );
}

#[tokio::test]
async fn list_abilities_reports_via_feedback() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    assert!(dispatch(1, LIST_ABILITIES, &[], &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("must feed back");
    assert!(fb.contains("abilities"), "got: {fb}");
}

#[tokio::test]
async fn show_flag_reports_bit_state() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    // Bit index 0 (BSF_DEAD); fresh player → clear.
    assert!(dispatch(1, GM_SHOW_FLAG, &0i32.to_le_bytes(), &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("must feed back");
    assert!(fb.contains("bit 0") && fb.contains("clear"), "got: {fb}");
    // Out-of-range bit → error feedback.
    assert!(dispatch(1, GM_SHOW_FLAG, &99i32.to_le_bytes(), &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("must feed back");
    assert!(fb.contains("bit index"), "got: {fb}");
}

#[tokio::test]
async fn get_mob_attribute_maps_known_attrs() {
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    mgr.get_entity_mut(2).unwrap().level = 25;
    let (tx, mut rx) = mpsc::channel(8);
    let mut args = 2i32.to_le_bytes().to_vec();
    write_wstring_arg(&mut args, "level");
    assert!(dispatch(1, GM_GET_MOB_ATTRIBUTE, &args, &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("must feed back");
    assert!(fb.contains("level: 25"), "got: {fb}");

    // Unknown attribute → guidance feedback.
    let mut args = 2i32.to_le_bytes().to_vec();
    write_wstring_arg(&mut args, "bogus");
    assert!(dispatch(1, GM_GET_MOB_ATTRIBUTE, &args, &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("must feed back");
    assert!(fb.contains("unknown attribute"), "got: {fb}");
}

#[tokio::test]
async fn show_mob_count_counts_npcs_in_space() {
    let mut mgr = mgr_with_player(1, "Castle");
    // Two NPCs via spawn_npc so they land in the npc set.
    mgr.spawn_npc(50, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    mgr.spawn_npc(51, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    let (tx, mut rx) = mpsc::channel(8);
    // SpaceID 0 → caller's space.
    assert!(dispatch(1, GM_SHOW_MOB_COUNT, &0i32.to_le_bytes(), &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("must feed back");
    assert!(fb.contains("2 NPC"), "expected 2 NPCs, got: {fb}");
}
