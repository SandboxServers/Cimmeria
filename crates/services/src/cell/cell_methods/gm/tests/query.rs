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

/// `gmGetMobAttribute` must map each supported attribute to the entity's actual
/// value. One NPC is seeded with known stats/fields, then every attribute arm
/// is exercised and the feedback is checked for the exact value. `level` and
/// `bogus` are covered elsewhere; this pins the remaining eight arms so a
/// regression that drops or mis-maps an arm (e.g. focus→health) trips here.
#[tokio::test]
async fn get_mob_attribute_covers_all_supported_arms() {
    use cimmeria_entity::stats::{FOCUS, HEALTH};

    let mut mgr = mgr_with_player(1, "Castle");
    mgr.create_entity(2, "Castle", [11.0, 22.0, 33.0], [0.0; 3])
        .unwrap();
    {
        let e = mgr.get_entity_mut(2).unwrap();
        e.faction = 4;
        e.alignment = 3;
        e.template_id = Some(987);
        e.npc_name = Some("Jaffa Warrior".into());
        // get_mut works on a fresh entity: StatList::new() seeds HEALTH/FOCUS.
        let h = e.stats.get_mut(HEALTH).expect("HEALTH stat present");
        h.cur = 120;
        h.max = 150;
        let f = e.stats.get_mut(FOCUS).expect("FOCUS stat present");
        f.cur = 40;
        f.max = 60;
    }
    let (tx, mut rx) = mpsc::channel(8);

    let ask = |attr: &str| {
        let mut args = 2i32.to_le_bytes().to_vec();
        write_wstring_arg(&mut args, attr);
        args
    };

    for (attr, expected) in [
        ("health", "health: 120/150"),
        ("focus", "focus: 40/60"),
        ("faction", "faction: 4"),
        ("alignment", "alignment: 3"),
        ("aistate", "ai_state:"),
        ("name", "name: Jaffa Warrior"),
        ("template", "template: Some(987)"),
        ("pos", "pos: (11.0, 22.0, 33.0)"),
    ] {
        assert!(dispatch(1, GM_GET_MOB_ATTRIBUTE, &ask(attr), &tx, &mut mgr).await);
        let fb = feedback_text(&drain(&mut rx), 1).expect("must feed back");
        assert!(
            fb.contains(expected),
            "attr '{attr}' expected substring '{expected}', got: {fb}"
        );
    }
}

/// `gmDebugMobData` (args: spaceId hint + target) must dump the target mob's
/// template + faction/level. Pins the happy-path branch (the in-space match arm)
/// distinct from the truncated/not-found paths.
#[tokio::test]
async fn debug_mob_data_dumps_target_mob() {
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    {
        let e = mgr.get_entity_mut(2).unwrap();
        e.template_id = Some(555);
        e.faction = 9;
        e.level = 12;
    }
    let (tx, mut rx) = mpsc::channel(8);
    // Args: INT32 spaceId (hint, ignored) + INT32 target.
    let mut args = 0i32.to_le_bytes().to_vec();
    args.extend_from_slice(&2i32.to_le_bytes());
    assert!(dispatch(1, GM_DEBUG_MOB_DATA, &args, &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("must feed back");
    assert!(
        fb.contains("mob [2]")
            && fb.contains("tmpl Some(555)")
            && fb.contains("faction 9")
            && fb.contains("lvl 12"),
        "debug dump must include the mob's template/faction/level, got: {fb}"
    );
}

/// `gmShowFlag` must report SET when the queried bit is set on the subject's
/// `state_field`. The existing test only covers the clear + out-of-range paths;
/// this pins the SET branch and the raw `state_field` echo.
#[tokio::test]
async fn show_flag_reports_set_bit() {
    let mut mgr = mgr_with_player(1, "Castle");
    // Set bit 5 directly on the caller's raw state_field; gmShowFlag with no
    // target inspects the caller (subject_or_self).
    mgr.get_entity_mut(1).unwrap().state_field = 1u32 << 5;
    let (tx, mut rx) = mpsc::channel(8);
    assert!(dispatch(1, GM_SHOW_FLAG, &5i32.to_le_bytes(), &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("must feed back");
    assert!(
        fb.contains("bit 5") && fb.contains("SET") && fb.contains("0x00000020"),
        "set bit must report SET + raw state_field, got: {fb}"
    );
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
