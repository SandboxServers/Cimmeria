//! Tests for the `.`-console framework: registry integrity, the GM
//! access gate, the parser, and end-to-end dispatch coverage.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::*;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Build a SpaceManager with a GM player (entity 1, access_level 2) targeting an
/// NPC (entity 100001) in a small test world.
fn setup() -> (SpaceManager, u32, u32) {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();

    let gm = 1u32;
    mgr.create_entity(gm, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(gm);

    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc(npc, "Agnos", [12.0, 0.0, 12.0], [0.0; 3])
        .unwrap();

    if let Some(e) = mgr.get_entity_mut(gm) {
        e.access_level = 2;
        e.is_player = true;
        e.current_target_id = Some(npc as i32);
    }
    if let Some(e) = mgr.get_entity_mut(npc) {
        e.is_player = false;
        e.template_id = Some(1);
    }
    (mgr, gm, npc)
}

/// Decode the text of an `onPlayerCommunication` feedback `EntityMethodCall`.
/// Layout: speaker WSTRING("SYSTEM") + flags + channel + text WSTRING.
fn decode_feedback(msg: &CellToBaseMsg) -> Option<String> {
    let CellToBaseMsg::EntityMethodCall {
        method_index, args, ..
    } = msg
    else {
        return None;
    };
    if *method_index != crate::mercury::method_idx::ON_PLAYER_COMMUNICATION {
        return None;
    }
    // speaker "SYSTEM" = 6 chars → 4 + 12 = 16; +flags(1)+channel(1) = 18.
    let text_off = 18;
    let len = u32::from_le_bytes(args.get(text_off..text_off + 4)?.try_into().ok()?) as usize;
    let start = text_off + 4;
    let mut chars = Vec::with_capacity(len);
    for i in 0..len {
        let o = start + i * 2;
        chars.push(u16::from_le_bytes(args.get(o..o + 2)?.try_into().ok()?));
    }
    Some(String::from_utf16_lossy(&chars))
}

#[test]
fn command_names_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for spec in COMMANDS {
        assert!(
            seen.insert(spec.name),
            "duplicate command name: {}",
            spec.name
        );
    }
}

#[test]
fn is_gm_threshold() {
    assert!(!is_gm(0));
    assert!(!is_gm(1));
    assert!(is_gm(2));
    assert!(is_gm(4));
}

#[test]
fn min_le_max_for_every_spec() {
    for spec in COMMANDS {
        assert!(spec.min <= spec.max, "{}: min > max", spec.name);
    }
}

/// Every registered command must have a dispatch arm — driving `exec` for each
/// must never hit the "not implemented" catch-all.
#[tokio::test]
async fn every_spec_is_dispatched() {
    let engine = ChainEngine::new();
    for spec in COMMANDS {
        let (mut mgr, gm, npc) = setup();
        let (tx, mut rx) = mpsc::channel(256);
        // Supply `min` dummy numeric args (parse as both int and float; also
        // valid as opaque strings), within the command's bounds.
        let dummy: Vec<&str> = vec!["10"; spec.min];
        exec(spec.name, gm, &dummy, Some(npc), &tx, &mut mgr, &engine).await;
        while let Ok(msg) = rx.try_recv() {
            if let Some(text) = decode_feedback(&msg) {
                assert!(
                    !text.contains("not implemented"),
                    "command .{} fell through to the not-implemented arm",
                    spec.name
                );
            }
        }
    }
}

#[tokio::test]
async fn unknown_command_reports_unknown() {
    let (mut mgr, gm, _npc) = setup();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".boguscmd", &tx, &mut mgr, &engine).await;
    let mut saw_unknown = false;
    while let Ok(msg) = rx.try_recv() {
        if let Some(text) = decode_feedback(&msg) {
            if text.contains("Unknown command") {
                saw_unknown = true;
            }
        }
    }
    assert!(
        saw_unknown,
        "unknown command should report 'Unknown command'"
    );
}

#[tokio::test]
async fn too_few_args_is_rejected() {
    let (mut mgr, gm, _npc) = setup();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    // .searchitem requires >= 1 arg.
    handle_console_command(gm, ".searchitem", &tx, &mut mgr, &engine).await;
    let mut saw = false;
    while let Ok(msg) = rx.try_recv() {
        if let Some(text) = decode_feedback(&msg) {
            if text.contains("not enough arguments") {
                saw = true;
            }
        }
    }
    assert!(saw, "missing required arg should be rejected");
}

#[tokio::test]
async fn typed_command_requires_target() {
    let (mut mgr, gm, _npc) = setup();
    // Clear the GM's target so a target-typed command must refuse.
    if let Some(e) = mgr.get_entity_mut(gm) {
        e.current_target_id = None;
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".primarystats", &tx, &mut mgr, &engine).await;
    let mut saw = false;
    while let Ok(msg) = rx.try_recv() {
        if let Some(text) = decode_feedback(&msg) {
            if text.contains("target is required") {
                saw = true;
            }
        }
    }
    assert!(saw, "a target-typed command must require a target");
}

#[tokio::test]
async fn help_lists_commands() {
    let (mut mgr, gm, _npc) = setup();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(512);
    handle_console_command(gm, ".help", &tx, &mut mgr, &engine).await;
    let mut count = 0;
    while let Ok(msg) = rx.try_recv() {
        if decode_feedback(&msg).is_some() {
            count += 1;
        }
    }
    assert!(count >= COMMANDS.len(), "help should list every command");
}

/// Regression guard: `parse_f32` must reject non-finite values — `NaN`/`inf`
/// would Display as a bareword and produce invalid SQL in the authoring path.
#[tokio::test]
async fn parse_f32_rejects_non_finite() {
    let (tx, _rx) = mpsc::channel(8);
    assert_eq!(parse_f32(1, &["1.5"], 0, "x", &tx).await, Some(1.5));
    assert_eq!(parse_f32(1, &["2"], 0, "x", &tx).await, Some(2.0));
    assert_eq!(parse_f32(1, &["NaN"], 0, "x", &tx).await, None);
    assert_eq!(parse_f32(1, &["inf"], 0, "x", &tx).await, None);
    assert_eq!(parse_f32(1, &["-inf"], 0, "x", &tx).await, None);
}

/// Regression guard: `.path_clear` must record TWO single-statement SQL strings,
/// not one multi-statement string (sqlx's prepared-statement executor rejects
/// multiple commands in one `query().execute()`).
#[tokio::test]
async fn path_clear_records_single_statements() {
    let (mut mgr, gm, _npc) = setup();
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(32);
    handle_console_command(gm, ".path_clear 7", &tx, &mut mgr, &engine).await;
    let recorded = mgr.authoring_changes.get(&gm).cloned().unwrap_or_default();
    assert_eq!(recorded.len(), 2, "path_clear should record two statements");
    for (_file, sql) in &recorded {
        assert_eq!(
            sql.matches(';').count(),
            1,
            "each recorded statement must be single-statement: {sql}"
        );
    }
}

/// Regression guard: a negative waypoint index must be rejected before it
/// reaches the `OFFSET {index}` subquery (Postgres rejects negative OFFSET).
#[tokio::test]
async fn negative_waypoint_index_is_rejected() {
    let (mut mgr, gm, _npc) = setup();
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(16);
    handle_console_command(gm, ".path_set_tp 7 -1", &tx, &mut mgr, &engine).await;
    assert!(
        mgr.authoring_changes.get(&gm).is_none_or(Vec::is_empty),
        "a negative index must not record any authoring SQL"
    );
}

/// Regression guard: `.delspawn` must key the recorded DELETE on the exact
/// `spawn_id`, never a fuzzy position/template match that could hit siblings.
#[tokio::test]
async fn delspawn_keys_on_spawn_id() {
    let (mut mgr, gm, npc) = setup();
    if let Some(e) = mgr.get_entity_mut(npc) {
        e.spawn_id = Some(42);
    }
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(32);
    handle_console_command(gm, ".delspawn", &tx, &mut mgr, &engine).await;
    let recorded = mgr.authoring_changes.get(&gm).cloned().unwrap_or_default();
    assert_eq!(recorded.len(), 1, "delspawn records exactly one statement");
    assert!(
        recorded[0].1.contains("spawn_id = 42"),
        "delspawn must target the exact spawn_id: {}",
        recorded[0].1
    );
}

/// A command-spawned NPC (no `spawn_id`) cannot be `.delspawn`'d — there's no
/// spawnlist row to delete, so nothing is recorded.
#[tokio::test]
async fn delspawn_refuses_without_spawn_id() {
    let (mut mgr, gm, _npc) = setup(); // setup NPC has no spawn_id
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(16);
    handle_console_command(gm, ".delspawn", &tx, &mut mgr, &engine).await;
    assert!(
        mgr.authoring_changes.get(&gm).is_none_or(Vec::is_empty),
        "delspawn with no spawn_id must record nothing"
    );
}

#[tokio::test]
async fn path_add_buffers_and_records() {
    let (mut mgr, gm, _npc) = setup();
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(64);
    handle_console_command(gm, ".path_add 7", &tx, &mut mgr, &engine).await;
    assert_eq!(
        mgr.patrol_authoring.get(&7).map(Vec::len),
        Some(1),
        "path_add should buffer one waypoint"
    );
    // First waypoint records EXACTLY two seed statements: the point_sets
    // header + the point row. Asserting the exact count (not `>= 2`) catches
    // a fan-out regression where extra authoring statements start landing.
    assert_eq!(
        mgr.authoring_changes.get(&gm).map(Vec::len),
        Some(2),
        "first path_add waypoint must record exactly the point_sets header + one point row"
    );
}
