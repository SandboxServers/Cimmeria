//! Spawn authoring regression suite: `.lookat`, `.movehere`, and the
//! queue-then-confirm path of `.savespawn` / `.seedconfirm`.
//!
//! What is proven here:
//!
//!   * `.lookat` writes a yaw into `direction.y`. It used to write a
//!     Cartesian facing vector into the `[pitch, yaw, roll]` field, which left
//!     yaw at 0 and saved heading 0;
//!   * `.movehere` puts an NPC at the caller's position and facing, and makes
//!     that its home so the leash / stop / respawn paths don't undo it;
//!   * `.savespawn` touches no database until `.seedconfirm`, and
//!     `.seedcancel` writes nothing;
//!   * re-saving one NPC before confirming keeps one change, and re-saving a
//!     confirmed new spawn is refused, so no NPC is ever inserted twice;
//!   * `.seedconfirm` emits one SigNoz event per spawn with every column
//!     needed to rebuild the row.
//!
//! Filter prefix: `spawn_authoring_`.

use cimmeria_common::Vector3;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;
use tracing::Level;

use super::{decode_feedback, setup};
use crate::cell::console::handle_console_command;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{SpaceManager, SpawnRowOp};
use crate::test_support::LogCapture;

/// Run one `.`-command as the GM from `setup()` (whose selected target is the
/// NPC), returning how many live-DB writes it sent and its feedback lines.
async fn run(mgr: &mut SpaceManager, gm: u32, line: &str) -> (usize, Vec<String>) {
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);
    handle_console_command(gm, line, &tx, mgr, &engine).await;
    let mut live = 0;
    let mut feedback = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if matches!(msg, CellToBaseMsg::ExecuteAuthoringSql { .. }) {
            live += 1;
        } else if let Some(text) = decode_feedback(&msg) {
            feedback.push(text);
        }
    }
    (live, feedback)
}

fn queued(mgr: &SpaceManager, gm: u32) -> usize {
    mgr.authoring_changes.get(&gm).map_or(0, Vec::len)
}

// ── .lookat ─────────────────────────────────────────────────────────────────

/// GM at (10, 10), NPC at (12, 12): the NPC must turn to yaw
/// `atan2(-2, -2)` = -135 deg, with pitch and roll zero.
#[tokio::test]
async fn spawn_authoring_lookat_writes_yaw_not_a_vector() {
    let (mut mgr, gm, npc) = setup();
    run(&mut mgr, gm, ".lookat").await;

    let expected = (-2.0f32).atan2(-2.0);
    let e = mgr.get_entity(npc).unwrap();
    assert_eq!(e.direction, Vector3::new(0.0, expected, 0.0));
    assert_eq!(
        e.spawn_direction,
        Some(Vector3::new(0.0, expected, 0.0)),
        "the new facing must become the NPC's home facing, or it turns back when it stops"
    );
}

/// The bug's visible symptom: `.lookat` then `.savespawn` saved heading 0.
#[tokio::test]
async fn spawn_authoring_lookat_then_savespawn_saves_the_heading() {
    let (mut mgr, gm, _npc) = setup();
    run(&mut mgr, gm, ".lookat").await;
    run(&mut mgr, gm, ".savespawn").await;

    let row = mgr.authoring_changes[&gm][0].spawn.clone().unwrap();
    assert!((row.heading - (-2.0f32).atan2(-2.0)).abs() < 1e-6);
    let sql = &mgr.authoring_changes[&gm][0].sql;
    assert!(sql.contains(&(-2.0f32).atan2(-2.0).to_string()), "{sql}");
}

// ── .movehere ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn spawn_authoring_movehere_places_npc_at_caller_facing_and_home() {
    let (mut mgr, gm, npc) = setup();
    if let Some(e) = mgr.get_entity_mut(gm) {
        e.direction = Vector3::new(0.0, 1.25, 0.0);
    }
    if let Some(e) = mgr.get_entity_mut(npc) {
        e.nav_path.push_back(Vector3::new(50.0, 0.0, 50.0));
    }

    let (live, feedback) = run(&mut mgr, gm, ".movehere").await;

    let e = mgr.get_entity(npc).unwrap();
    assert_eq!(e.position, Vector3::new(10.0, 0.0, 10.0));
    assert_eq!(e.direction, Vector3::new(0.0, 1.25, 0.0));
    assert_eq!(e.spawn_position, Some(Vector3::new(10.0, 0.0, 10.0)));
    assert_eq!(e.spawn_direction, Some(Vector3::new(0.0, 1.25, 0.0)));
    assert!(e.nav_path.is_empty(), "a stale path would walk it away");
    assert_eq!(live, 0);
    assert_eq!(
        queued(&mgr, gm),
        0,
        "no autosave unless .autosavespawn is on"
    );
    assert!(
        feedback.iter().any(|f| f.contains("movehere [")),
        "{feedback:?}"
    );
}

/// `.movehere` is `Target::Mob`: a selected player is refused and not moved.
#[tokio::test]
async fn spawn_authoring_movehere_refuses_a_player() {
    let (mut mgr, gm, _npc) = setup();
    let other = 77;
    mgr.create_entity(other, "Agnos", [40.0, 0.0, 40.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(other);
    if let Some(e) = mgr.get_entity_mut(other) {
        e.is_player = true;
    }
    if let Some(e) = mgr.get_entity_mut(gm) {
        e.current_target_id = Some(other as i32);
        e.witnesses.insert(cimmeria_common::EntityId(other as i32));
    }

    run(&mut mgr, gm, ".movehere").await;

    assert_eq!(
        mgr.get_entity(other).unwrap().position,
        Vector3::new(40.0, 0.0, 40.0)
    );
}

#[tokio::test]
async fn spawn_authoring_autosave_queues_a_savespawn_after_movehere() {
    let (mut mgr, gm, npc) = setup();
    run(&mut mgr, gm, ".autosavespawn 1").await;
    run(&mut mgr, gm, ".movehere").await;

    assert_eq!(queued(&mgr, gm), 1);
    let row = mgr.authoring_changes[&gm][0].spawn.clone().unwrap();
    assert_eq!(row.entity_id, npc);
    assert_eq!((row.x, row.y, row.z), (10.0, 0.0, 10.0));
}

// ── queue, confirm, cancel ──────────────────────────────────────────────────

#[tokio::test]
async fn spawn_authoring_savespawn_queues_without_touching_the_db() {
    let (mut mgr, gm, npc) = setup();
    let (live, _) = run(&mut mgr, gm, ".savespawn").await;

    assert_eq!(
        live, 0,
        "savespawn must not write the live DB before .seedconfirm"
    );
    let change = &mgr.authoring_changes[&gm][0];
    let row = change.spawn.clone().unwrap();
    assert_eq!(row.op, SpawnRowOp::Insert);
    assert_eq!(row.entity_id, npc);
    assert_eq!(row.world, "Agnos");
    assert_eq!(row.template_id, 1);
    assert_eq!((row.x, row.y, row.z), (12.0, 0.0, 12.0));
    assert!(change.sql.starts_with("INSERT INTO resources.spawnlist"));

    let (live, _) = run(&mut mgr, gm, ".seedconfirm").await;
    assert_eq!(live, 1, "seedconfirm writes each queued change live");
    assert_eq!(queued(&mgr, gm), 0);
}

#[tokio::test]
async fn spawn_authoring_seedcancel_writes_nothing() {
    let (mut mgr, gm, _npc) = setup();
    let (a, _) = run(&mut mgr, gm, ".savespawn").await;
    let (b, _) = run(&mut mgr, gm, ".seedcancel").await;
    let (c, _) = run(&mut mgr, gm, ".seedconfirm").await;
    assert_eq!(a + b + c, 0);
}

#[tokio::test]
async fn spawn_authoring_resave_before_confirm_replaces_the_queued_change() {
    let (mut mgr, gm, npc) = setup();
    run(&mut mgr, gm, ".savespawn").await;
    if let Some(e) = mgr.get_entity_mut(npc) {
        e.position = Vector3::new(20.0, 0.0, 20.0);
    }
    let (_, feedback) = run(&mut mgr, gm, ".savespawn").await;

    assert_eq!(queued(&mgr, gm), 1, "one NPC, one queued row");
    assert_eq!(mgr.authoring_changes[&gm][0].spawn.clone().unwrap().x, 20.0);
    assert!(
        feedback.iter().any(|f| f.contains("replaced")),
        "{feedback:?}"
    );
}

/// A confirmed new spawn has a live row but no `spawn_id` in memory, so a
/// second save would insert a duplicate. It must be refused.
#[tokio::test]
async fn spawn_authoring_resave_after_confirm_of_a_new_spawn_is_refused() {
    let (mut mgr, gm, _npc) = setup();
    run(&mut mgr, gm, ".savespawn").await;
    run(&mut mgr, gm, ".seedconfirm").await;
    let (_, feedback) = run(&mut mgr, gm, ".savespawn").await;

    assert_eq!(queued(&mgr, gm), 0);
    assert!(
        feedback.iter().any(|f| f.contains("already confirmed")),
        "{feedback:?}"
    );
}

/// A seeded NPC (with a `spawn_id`) re-saves as an UPDATE any number of times.
#[tokio::test]
async fn spawn_authoring_seeded_npc_can_be_saved_again_after_confirm() {
    let (mut mgr, gm, npc) = setup();
    if let Some(e) = mgr.get_entity_mut(npc) {
        e.spawn_id = Some(42);
    }
    run(&mut mgr, gm, ".savespawn").await;
    run(&mut mgr, gm, ".seedconfirm").await;
    run(&mut mgr, gm, ".savespawn").await;

    let change = &mgr.authoring_changes[&gm][0];
    assert_eq!(change.spawn.clone().unwrap().op, SpawnRowOp::Update);
    assert!(change.sql.contains("WHERE spawn_id = 42"));
}

/// The SigNoz contract: every column a developer needs to rebuild the row,
/// plus the batch id and the GM's identity.
#[tokio::test]
async fn spawn_authoring_seedconfirm_emits_the_full_row_to_telemetry() {
    let (mut mgr, gm, _npc) = setup();
    if let Some(e) = mgr.get_entity_mut(gm) {
        e.account_id = Some(900);
        e.player_id = Some(901);
    }
    run(&mut mgr, gm, ".savespawn").await;

    let capture = LogCapture::install();
    run(&mut mgr, gm, ".seedconfirm").await;
    let all = capture.all();

    let spawn = all
        .iter()
        .find(|e| e.target == "authoring" && e.message_contains("seed spawn confirmed"))
        .expect("one spawn event per confirmed spawn");
    assert_eq!(spawn.level, Level::INFO);
    for key in [
        "batch",
        "seed_file",
        "op",
        "world",
        "template_id",
        "x",
        "y",
        "z",
        "heading",
        "heading_deg",
        "sql",
        "account_id",
        "player_id",
    ] {
        assert!(spawn.fields.contains_key(key), "missing {key}: {spawn:?}");
    }
    assert!(spawn.fields["op"].contains("insert"));
    assert!(spawn.fields["world"].contains("Agnos"));
    assert!(spawn.has_field("template_id", "1"));
    assert!(spawn.has_field("account_id", "900"));

    let block = all
        .iter()
        .find(|e| e.target == "authoring" && e.message_contains("seed authoring confirmed"))
        .expect("one block event per seed file");
    assert_eq!(block.fields.get("batch"), spawn.fields.get("batch"));
}

/// Rule 6, resolve before teardown: the NPC a `.delspawn` removed is named
/// on its `seed spawn confirmed` row by the name it had when the GM queued
/// the change. By `.seedconfirm` the NPC is gone and its slot can hold
/// another entity, so a lookup at confirm time would name the wrong one.
#[tokio::test]
async fn spawn_authoring_seedconfirm_names_the_npc_as_queued() {
    use cimmeria_names::{NameBook, Table};
    const QUEUED_NAME_ID: i32 = 27_001;
    const REUSED_NAME_ID: i32 = 27_002;
    let mut book = NameBook::empty();
    book.insert(Table::Texts, QUEUED_NAME_ID.into(), "Unas Hunter");
    book.insert(Table::Texts, REUSED_NAME_ID.into(), "Jaffa Patrol");
    cimmeria_names::global().store(book);

    let (mut mgr, gm, npc) = setup();
    if let Some(e) = mgr.get_entity_mut(npc) {
        e.spawn_id = Some(42);
        e.name_id = Some(QUEUED_NAME_ID);
    }
    run(&mut mgr, gm, ".delspawn").await;
    assert!(mgr.get_entity(npc).is_none(), ".delspawn despawns the NPC");

    // Recycle the slot with a different NPC before the GM confirms.
    mgr.spawn_npc(npc, "Agnos", [12.0, 0.0, 12.0], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(npc) {
        e.name_id = Some(REUSED_NAME_ID);
    }

    let capture = LogCapture::install();
    run(&mut mgr, gm, ".seedconfirm").await;
    let row = capture
        .all()
        .into_iter()
        .find(|e| e.target == "authoring" && e.message_contains("seed spawn confirmed"))
        .expect("the confirmed delete emits its spawn row");
    cimmeria_names::global().store(NameBook::empty());

    assert!(
        row.has_field("npc_entity_name", "Unas Hunter"),
        "the row must name the NPC the GM deleted, not the slot's new \
         occupant; got {row:#?}"
    );
}
