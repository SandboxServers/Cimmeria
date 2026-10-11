//! Cell-loop request/reply tests for `BaseToCellMsg::LabConsoleExec`.
//!
//! Mirrors the `create_entity_instance` reply-shape tests: drive the message
//! through `handle_base_message`, `.await` the `reply_tx`, and assert on the
//! captured result. These guard the live-research-lab MCP `server_console_exec`
//! path (issue #687): output must be *captured* (not sent to the player as
//! chat), and the GM access-level gate must still reject a non-GM entity.

use super::*;
use tokio::sync::oneshot;
use tracing::Level;

use crate::cell::messages::LabConsoleResult;
use crate::mercury::method_idx::ON_PLAYER_COMMUNICATION;
use crate::test_support::LogCapture;

/// Build a one-space manager with a single entity, at the given access level.
fn make_manager_with_entity(entity_id: u32, access_level: u32) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(entity_id, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(entity_id) {
        e.is_player = true;
        e.access_level = access_level;
    }
    mgr.connect_entity(entity_id);
    mgr
}

async fn exec(
    mgr: &mut SpaceManager,
    entity_id: u32,
    line: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> LabConsoleResult {
    let (reply_tx, reply_rx) = oneshot::channel();
    handle_base_message(
        BaseToCellMsg::LabConsoleExec {
            entity_id,
            line: line.to_string(),
            reply_tx,
        },
        tx,
        mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;
    reply_rx.await.expect("LabConsoleExec must always reply")
}

/// The point of the message: a GM's console command runs and its feedback is
/// *captured into the reply*, not forwarded to the player's client as chat.
///
/// Regression shape: if the handler forwarded feedback to the real cell→base
/// channel (the in-world behavior) instead of teeing it, the reply would be
/// empty and the caller would never see the output — the exact failure the
/// MCP `server_console_exec` tool exists to avoid.
#[tokio::test]
async fn lab_console_exec_captures_gm_command_output() {
    let mut mgr = make_manager_with_entity(1, 2); // access_level 2 = GameMaster
    let (tx, mut rx) = mpsc::channel(32);

    let result = exec(&mut mgr, 1, ".help", &tx).await;

    let lines = result.expect("a GM command must execute");
    assert!(
        !lines.is_empty(),
        ".help must produce at least one captured feedback line"
    );

    // The captured feedback must NOT also have been sent to the player as chat:
    // no `onPlayerCommunication` addressed to the caller may reach the real
    // cell→base channel.
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            ..
        } = msg
        {
            assert!(
                !(entity_id == 1 && method_index == ON_PLAYER_COMMUNICATION),
                "GM feedback must be captured into the reply, not forwarded as chat"
            );
        }
    }
}

/// The GM access-level gate that `crate::cell::chat` enforces on in-world
/// `.`-console input must still apply here: a non-GM acting entity is rejected,
/// never executed.
///
/// Regression shape: drop the `is_gm` check and a level-0 player could drive
/// every dev/authoring command through the lab endpoint — a privilege
/// escalation. The guard fails (reply becomes `Ok`) if the gate is removed.
#[tokio::test]
async fn lab_console_exec_rejects_non_gm_entity() {
    let capture = LogCapture::install();
    let mut mgr = make_manager_with_entity(2, 0); // access_level 0 = Player
    if let Some(e) = mgr.get_entity_mut(2) {
        e.player_id = Some(12);
    }
    let (tx, mut rx) = mpsc::channel(32);

    let result = exec(&mut mgr, 2, ".help", &tx).await;

    let err = result.expect_err("a non-GM entity must be rejected");
    assert!(
        err.contains("not authorized"),
        "rejection reason must name the authorization failure, got: {err}"
    );

    // The WARN row must name the reason and the acting entity's identity.
    let event = capture
        .find_event(Level::WARN, "LabConsoleExec rejected", "not_gm")
        .expect("a non-GM rejection must log a not_gm WARN");
    assert!(
        event.has_field("player_id", "12"),
        "the rejection must name the player; got {event:#?}"
    );
    assert!(
        event.has_field("entity_id", "2"),
        "the rejection must name the entity; got {event:#?}"
    );

    // A rejected command must not run — nothing at all may reach base.
    assert!(
        rx.try_recv().is_err(),
        "a rejected LabConsoleExec must not emit any cell→base message"
    );
}

/// An unknown entity id (never created) is treated as access level 0 and
/// rejected — the endpoint must not be able to drive commands as a
/// non-existent entity.
#[tokio::test]
async fn lab_console_exec_rejects_unknown_entity() {
    let capture = LogCapture::install();
    let mut mgr = make_manager_with_entity(1, 2);
    let (tx, _rx) = mpsc::channel(32);

    let result = exec(&mut mgr, 999, ".help", &tx).await;
    assert!(
        result.is_err(),
        "an unknown acting entity must be rejected, not executed"
    );

    // A missing entity is a distinct reason from a present non-GM entity.
    assert!(
        capture
            .find_event(Level::WARN, "LabConsoleExec rejected", "entity_missing")
            .is_some(),
        "an unknown acting entity must log an entity_missing WARN"
    );
}
