//! `lootItem` handler tests: display args, the racing-click demotion, the
//! looter checks and the range gate.

#[test]
fn loot_display_args_format() {
    let npc_id: i32 = 100_003;
    let mut args = Vec::new();
    args.extend_from_slice(&npc_id.to_le_bytes());
    args.extend_from_slice(&0u32.to_le_bytes()); // empty loot
    args.push(1); // initial

    assert_eq!(args.len(), 9);
    assert_eq!(u32::from_le_bytes([args[4], args[5], args[6], args[7]]), 0);
    assert_eq!(args[8], 1);
}

/// Pins the WARN→DEBUG demotion for the "player not looting anything"
/// path. Symptom shape: client-side "Loot All" fires `lootItem(i)`
/// per visible entry. The corpse-exhaustion path clears
/// `looting_entity` and sends an empty `onLootDisplay` to close the
/// window — but any clicks in flight before the close arrives land
/// here with `looting_entity = None`. There is no defensive action
/// to take (no NPC id to address a close at), and the event is a
/// known benign race, so it must not be WARN. Observed surface:
/// 2 false positives per "Loot All" sequence on lomiada's
/// 2026-06-04 session.
///
/// Reverting the demotion (DEBUG → WARN) trips this guard.
#[tokio::test]
async fn loot_item_with_no_looting_entity_logs_at_debug() {
    use super::super::super::space_manager::SpaceManager;
    use super::*;
    use crate::test_support::LogCapture;
    use tokio::sync::mpsc;

    let capture = LogCapture::install();

    let mut mgr = SpaceManager::new(1);
    let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
    let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(spaces_xml).unwrap();
    mgr.create_startup_spaces(cell_spaces_xml).unwrap();

    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    // Note: looting_entity is left as None (the post-exhaustion state
    // the racing click lands in).

    let (tx, _rx) = mpsc::channel(16);
    handle_loot_item(1, 1, &tx, &mut mgr).await;

    assert!(
        capture
            .find_message(
                tracing::Level::DEBUG,
                "lootItem: player not looting anything"
            )
            .is_some(),
        "racing-click branch must log at DEBUG; got: {:#?}",
        capture.all()
    );
    assert!(
        capture
            .find_message(
                tracing::Level::WARN,
                "lootItem: player not looting anything"
            )
            .is_none(),
        "racing-click branch must NOT log at WARN — promoting it back \
         will re-introduce the 'Loot All' noise observed pre-fix: {:#?}",
        capture.all()
    );
}

/// Regression for #106 + Copilot review on PR #108: validate looter
/// has a player_id BEFORE removing the loot from the corpse. If the
/// player_id check ever moves back below the mutation, the drop is
/// gone and no grant fires.
#[tokio::test]
async fn loot_item_with_no_player_id_preserves_corpse_loot() {
    use super::super::super::space_manager::SpaceManager;
    use super::*;
    use cimmeria_entity::cell_entity::LootItem;
    use tokio::sync::mpsc;

    let mut mgr = SpaceManager::new(1);
    let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
    let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(spaces_xml).unwrap();
    mgr.create_startup_spaces(cell_spaces_xml).unwrap();

    // Looter — leave player_id as the default (None).
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let npc_id = mgr.allocate_npc_id();
    mgr.spawn_npc(npc_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    // Seed loot on the corpse and mark the player as looting it.
    if let Some(npc) = mgr.get_entity_mut(npc_id) {
        npc.loot.push(LootItem {
            design_id: None,
            quantity: 50,
            index: 1,
        });
    }
    if let Some(p) = mgr.get_entity_mut(1) {
        p.looting_entity = Some(npc_id);
        assert!(
            p.player_id.is_none(),
            "default CellEntity must have no player_id for this test"
        );
    }

    let (tx, mut rx) = mpsc::channel(16);
    handle_loot_item(1, 1, &tx, &mut mgr).await;

    // Corpse loot must still be present (the bug removed it before the
    // player_id check, leaving the player with nothing AND the corpse
    // empty).
    let loot_after = mgr.get_entity(npc_id).map(|e| e.loot.len()).unwrap_or(0);
    assert_eq!(
        loot_after, 1,
        "corpse loot must be intact when looter has no player_id"
    );

    // No GrantCash / GrantItem must have been queued.
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::GrantCash { .. } | CellToBaseMsg::GrantItem { .. } => {
                panic!("no grant message should fire when looter has no player_id");
            }
            _ => {}
        }
    }
}

// ── #446 loot range re-validation ─────────────────────────────────

/// Build a manager with one player (id 1, has player_id) at the origin
/// looting an NPC corpse (id 2) seeded with one cash drop. The corpse
/// position is caller-chosen so the distance gate can be exercised.
fn make_loot_mgr(corpse_pos: [f32; 3]) -> super::super::super::space_manager::SpaceManager {
    use super::super::super::space_manager::SpaceManager;
    use cimmeria_entity::cell_entity::LootItem;

    let mut mgr = SpaceManager::new(1);
    let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
    let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(spaces_xml).unwrap();
    mgr.create_startup_spaces(cell_spaces_xml).unwrap();

    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let npc_id = mgr.allocate_npc_id();
    mgr.spawn_npc(npc_id, "Agnos", corpse_pos, [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(npc_id) {
        npc.loot.push(LootItem {
            design_id: None, // cash
            quantity: 50,
            index: 1,
        });
    }
    if let Some(p) = mgr.get_entity_mut(1) {
        p.player_id = Some(42);
        p.looting_entity = Some(npc_id);
    }
    mgr
}

/// **#446 in-range positive guard.** A looter standing on the corpse
/// (distance 2 < MAX_INTERACT_DISTANCE) loots normally — the gate must
/// not over-block. The drop is removed and a GrantCash is queued.
#[tokio::test]
async fn loot_item_in_range_succeeds() {
    use super::*;
    use tokio::sync::mpsc;

    let mut mgr = make_loot_mgr([2.0, 0.0, 0.0]);
    let npc_id = mgr.get_entity(1).and_then(|e| e.looting_entity).unwrap();

    let (tx, mut rx) = mpsc::channel(16);
    handle_loot_item(1, 1, &tx, &mut mgr).await;

    assert_eq!(
        mgr.get_entity(npc_id).map(|e| e.loot.len()).unwrap_or(99),
        0,
        "in-range loot must remove the drop from the corpse"
    );
    let granted = std::iter::from_fn(|| rx.try_recv().ok())
        .any(|m| matches!(m, CellToBaseMsg::GrantCash { .. }));
    assert!(granted, "in-range cash loot must queue a GrantCash");
}

/// **#446 out-of-range negative guard.** A looter far from the corpse
/// (distance 100 > MAX_INTERACT_DISTANCE) — the position-spoof / vacuum
/// case — must be denied: the drop stays on the corpse, nothing is
/// granted, and the rejection is logged. Pre-fix the handler trusted
/// the interact-time `looting_entity` pin and looted regardless of
/// live distance.
#[tokio::test]
async fn loot_item_out_of_range_is_denied() {
    use super::*;
    use crate::test_support::LogCapture;
    use tokio::sync::mpsc;

    let mut mgr = make_loot_mgr([100.0, 0.0, 0.0]);
    let npc_id = mgr.get_entity(1).and_then(|e| e.looting_entity).unwrap();

    let capture = LogCapture::install();
    let (tx, mut rx) = mpsc::channel(16);
    handle_loot_item(1, 1, &tx, &mut mgr).await;

    assert_eq!(
        mgr.get_entity(npc_id).map(|e| e.loot.len()).unwrap_or(0),
        1,
        "out-of-range loot must NOT remove the drop — the corpse keeps it"
    );
    let granted = std::iter::from_fn(|| rx.try_recv().ok()).any(|m| {
        matches!(
            m,
            CellToBaseMsg::GrantCash { .. } | CellToBaseMsg::GrantItem { .. }
        )
    });
    assert!(!granted, "out-of-range loot must not grant anything");
    assert!(
        capture
            .find_message(
                tracing::Level::WARN,
                "lootItem rejected -- looter is out of range"
            )
            .is_some(),
        "out-of-range loot must emit the documented rejection warn (#446)"
    );
}
