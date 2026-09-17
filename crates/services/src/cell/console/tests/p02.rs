//! P02 regression suite: `.info` / `.facing` / `.combatinfo`.
//!
//! Split out from the sibling `tests` module (rather than growing it past
//! the file-size cap) per `docs/analysis/legacy-command-parity/work-packets.md`
//! P02. Reaches back into `super` for the shared `setup`/`decode_feedback`
//! fixtures.

use cimmeria_common::Vector3;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::{decode_feedback, setup};
use crate::cell::console::handle_console_command;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Collect every decoded GM-feedback line still queued on `rx`, in order.
fn drain_feedback_lines(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<String> {
    let mut lines = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let Some(text) = decode_feedback(&msg) {
            lines.push(text);
        }
    }
    lines
}

/// Distinct `entity_id` recipients of every `EntityMethodCall` still queued
/// on `rx` — used to assert single-recipient (caller-only) delivery.
fn feedback_recipients(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> std::collections::HashSet<u32> {
    let mut recipients = std::collections::HashSet::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { entity_id, .. } = msg {
            recipients.insert(entity_id);
        }
    }
    recipients
}

/// Place `id` at `pos` facing `dir`, for `.facing` geometry fixtures.
fn place(mgr: &mut SpaceManager, id: u32, pos: [f32; 3], dir: [f32; 3]) {
    if let Some(e) = mgr.get_entity_mut(id) {
        e.position = Vector3::new(pos[0], pos[1], pos[2]);
        e.direction = Vector3::new(dir[0], dir[1], dir[2]);
    }
}

// ---- .info ------------------------------------------------------------

#[tokio::test]
async fn legacy_p02_info_selection_wins_over_explicit_id() {
    let (mut mgr, gm, npc) = setup();
    let other = mgr.allocate_npc_id();
    mgr.spawn_npc(other, "Agnos", [50.0, 0.0, 50.0], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(other) {
        e.template_id = Some(99);
    }
    // gm's current selection (set by `setup()`) is `npc`; the explicit
    // [entityId] arg points at `other` — selection must still win.
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);
    handle_console_command(gm, &format!(".info {other}"), &tx, &mut mgr, &engine).await;
    let lines = drain_feedback_lines(&mut rx);
    assert!(
        lines.iter().any(|l| l.contains(&format!("({npc}) "))),
        "selection (npc) must win over the explicit entityId arg: {lines:?}"
    );
    assert!(
        !lines.iter().any(|l| l.contains(&format!("({other}) "))),
        "explicit entityId arg must be ignored when a selection is set: {lines:?}"
    );
}

#[tokio::test]
async fn legacy_p02_info_explicit_id_used_when_no_selection() {
    let (mut mgr, gm, npc) = setup();
    if let Some(e) = mgr.get_entity_mut(gm) {
        e.current_target_id = None;
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);
    handle_console_command(gm, &format!(".info {npc}"), &tx, &mut mgr, &engine).await;
    let lines = drain_feedback_lines(&mut rx);
    assert!(
        lines.iter().any(|l| l.contains(&format!("({npc}) "))),
        "explicit entityId must be used as a fallback with no selection: {lines:?}"
    );
}

#[tokio::test]
async fn legacy_p02_info_neither_selection_nor_id_reports_not_found() {
    let (mut mgr, gm, _npc) = setup();
    if let Some(e) = mgr.get_entity_mut(gm) {
        e.current_target_id = None;
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".info", &tx, &mut mgr, &engine).await;
    assert_eq!(
        drain_feedback_lines(&mut rx),
        vec!["Could not find entity".to_string()],
        "no selection and no arg must report the exact legacy wording"
    );
}

#[tokio::test]
async fn legacy_p02_info_unknown_explicit_id_reports_not_found() {
    let (mut mgr, gm, _npc) = setup();
    if let Some(e) = mgr.get_entity_mut(gm) {
        e.current_target_id = None;
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".info 999999", &tx, &mut mgr, &engine).await;
    assert_eq!(
        drain_feedback_lines(&mut rx),
        vec!["Could not find entity".to_string()]
    );
}

#[tokio::test]
async fn legacy_p02_info_present_fields_are_shown() {
    let (mut mgr, gm, npc) = setup();
    if let Some(e) = mgr.get_entity_mut(npc) {
        e.tag = Some("MyTag".into());
        e.static_mesh = Some("CA-Props.Thing".into());
        e.body_set = Some("bodyset_x".into());
        e.archetype_id = Some(6); // Goauld
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);
    handle_console_command(gm, ".info", &tx, &mut mgr, &engine).await;
    let lines = drain_feedback_lines(&mut rx);
    assert!(lines.iter().any(|l| l == "Template: 1"), "{lines:?}");
    assert!(lines.iter().any(|l| l == "Tag: MyTag"), "{lines:?}");
    assert!(
        lines.iter().any(|l| l == "Static mesh: CA-Props.Thing"),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|l| l == "Body set: bodyset_x"),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|l| l == "Archetype: Goauld (6)"),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|l| l == "Alignment: undefined (0)"),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|l| l == "Faction: neutral (0)"),
        "{lines:?}"
    );
    assert!(lines.iter().any(|l| l == "Level: 1"), "{lines:?}");
}

#[tokio::test]
async fn legacy_p02_info_absent_fields_are_omitted() {
    // setup()'s npc has no tag/static_mesh/body_set/archetype_id/name_id/
    // event_set_id, and zero entity_flags/interaction_type_flags — none of
    // those lines may appear.
    let (mut mgr, gm, _npc) = setup();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);
    handle_console_command(gm, ".info", &tx, &mut mgr, &engine).await;
    let lines = drain_feedback_lines(&mut rx);
    for prefix in [
        "Tag:",
        "Static mesh:",
        "Body set:",
        "Archetype:",
        "Flags:",
        "Interaction:",
        "Kismet event set:",
        "Name ID:",
    ] {
        assert!(
            !lines.iter().any(|l| l.starts_with(prefix)),
            "'{prefix}' line must be omitted when the field is unset/zero: {lines:?}"
        );
    }
}

#[tokio::test]
async fn legacy_p02_info_output_is_caller_only() {
    let (mut mgr, gm, _npc) = setup();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);
    handle_console_command(gm, ".info", &tx, &mut mgr, &engine).await;
    assert_eq!(
        feedback_recipients(&mut rx),
        std::collections::HashSet::from([gm]),
        ".info output must go only to the caller"
    );
}

// ---- .facing ------------------------------------------------------------

#[tokio::test]
async fn legacy_p02_facing_requires_a_target() {
    let (mut mgr, gm, _npc) = setup();
    if let Some(e) = mgr.get_entity_mut(gm) {
        e.current_target_id = None;
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".facing", &tx, &mut mgr, &engine).await;
    let lines = drain_feedback_lines(&mut rx);
    assert!(
        lines.iter().any(|l| l.contains("target is required")),
        "{lines:?}"
    );
}

#[tokio::test]
async fn legacy_p02_facing_radians_and_degrees_agree() {
    let (mut mgr, gm, npc) = setup();
    place(&mut mgr, gm, [0.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
    place(&mut mgr, npc, [20.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".facing", &tx, &mut mgr, &engine).await;
    let lines = drain_feedback_lines(&mut rx);
    let facing_line = lines
        .iter()
        .find(|l| l.starts_with("Facing:"))
        .expect("a Facing line must be sent");
    // "Facing: {rad} rad / {deg} deg ({class})"
    let words: Vec<&str> = facing_line.split_whitespace().collect();
    let rad: f64 = words[1].parse().unwrap();
    let deg: f64 = words[4].parse().unwrap();
    assert!(
        (deg - rad * 180.0 / std::f64::consts::PI).abs() < 1e-3,
        "deg must equal rad * 180/pi: {facing_line}"
    );
}

#[tokio::test]
async fn legacy_p02_facing_class_front() {
    let (mut mgr, gm, npc) = setup();
    place(&mut mgr, gm, [0.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
    place(&mut mgr, npc, [0.0, 0.0, 20.0], [0.0, 0.0, 0.0]);
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".facing", &tx, &mut mgr, &engine).await;
    let lines = drain_feedback_lines(&mut rx);
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("Facing:") && l.contains("(Front)")),
        "{lines:?}"
    );
}

#[tokio::test]
async fn legacy_p02_facing_class_rear() {
    let (mut mgr, gm, npc) = setup();
    place(&mut mgr, gm, [0.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
    place(&mut mgr, npc, [0.0, 0.0, -20.0], [0.0, 0.0, 0.0]);
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".facing", &tx, &mut mgr, &engine).await;
    let lines = drain_feedback_lines(&mut rx);
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("Facing:") && l.contains("(Rear)")),
        "{lines:?}"
    );
}

#[tokio::test]
async fn legacy_p02_facing_class_flank() {
    let (mut mgr, gm, npc) = setup();
    place(&mut mgr, gm, [0.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
    place(&mut mgr, npc, [20.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".facing", &tx, &mut mgr, &engine).await;
    let lines = drain_feedback_lines(&mut rx);
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("Facing:") && l.contains("(Flank)")),
        "{lines:?}"
    );
}

#[tokio::test]
async fn legacy_p02_facing_class_above() {
    let (mut mgr, gm, npc) = setup();
    place(&mut mgr, gm, [0.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
    place(&mut mgr, npc, [0.0, 10.0, 0.0], [0.0, 0.0, 0.0]);
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".facing", &tx, &mut mgr, &engine).await;
    let lines = drain_feedback_lines(&mut rx);
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("Facing:") && l.contains("(Above)")),
        "{lines:?}"
    );
}

#[tokio::test]
async fn legacy_p02_facing_class_below() {
    let (mut mgr, gm, npc) = setup();
    place(&mut mgr, gm, [0.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
    place(&mut mgr, npc, [0.0, -10.0, 0.0], [0.0, 0.0, 0.0]);
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".facing", &tx, &mut mgr, &engine).await;
    let lines = drain_feedback_lines(&mut rx);
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("Facing:") && l.contains("(Below)")),
        "{lines:?}"
    );
}

#[tokio::test]
async fn legacy_p02_facing_exact_distance() {
    let (mut mgr, gm, npc) = setup();
    place(&mut mgr, gm, [0.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
    place(&mut mgr, npc, [3.0, 4.0, 0.0], [0.0, 0.0, 0.0]);
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".facing", &tx, &mut mgr, &engine).await;
    let lines = drain_feedback_lines(&mut rx);
    let dist_line = lines
        .iter()
        .find(|l| l.starts_with("Distance:"))
        .expect("a Distance line must be sent");
    assert_eq!(dist_line, "Distance: 5.000000");
}

// ---- .combatinfo ----------------------------------------------------------

#[tokio::test]
async fn legacy_p02_combatinfo_requires_mob_target() {
    let (mut mgr, gm, npc) = setup();
    // Wrong target type: mark the selected entity as a player so the
    // generic Target::Mob rejection fires at the dispatch level.
    if let Some(e) = mgr.get_entity_mut(npc) {
        e.is_player = true;
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".combatinfo", &tx, &mut mgr, &engine).await;
    let lines = drain_feedback_lines(&mut rx);
    assert!(
        lines
            .iter()
            .any(|l| l.contains("expected an NPC as a target")),
        "{lines:?}"
    );
}

#[tokio::test]
async fn legacy_p02_combatinfo_no_template_reports_problem() {
    let (mut mgr, gm, npc) = setup();
    if let Some(e) = mgr.get_entity_mut(npc) {
        e.template_id = None;
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".combatinfo", &tx, &mut mgr, &engine).await;
    let lines = drain_feedback_lines(&mut rx);
    assert_eq!(
        lines,
        vec![" - Entity is not spawned from a template".to_string()]
    );
}

#[tokio::test]
async fn legacy_p02_combatinfo_no_ability_set_reports_problem() {
    // setup()'s npc is spawned via `spawn_npc`, which grants
    // `NPC_DEFAULT_ABILITY` — clear it back to a genuinely empty ability set.
    let (mut mgr, gm, npc) = setup();
    if let Some(e) = mgr.get_entity_mut(npc) {
        e.abilities = cimmeria_entity::abilities::AbilityManager::new();
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".combatinfo", &tx, &mut mgr, &engine).await;
    let lines = drain_feedback_lines(&mut rx);
    assert_eq!(lines, vec![" - Entity has no ability set".to_string()]);
}

#[tokio::test]
async fn legacy_p02_combatinfo_healthy_target_is_silent() {
    let (mut mgr, gm, npc) = setup();
    if let Some(e) = mgr.get_entity_mut(npc) {
        // template_id is already Some(1) from setup(); add a known ability so
        // both currently-checked conditions are satisfied.
        e.abilities.add_ability(1);
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".combatinfo", &tx, &mut mgr, &engine).await;
    assert!(
        drain_feedback_lines(&mut rx).is_empty(),
        "a target with no known problems must produce zero feedback lines"
    );
}
