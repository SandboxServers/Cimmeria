//! Click-path guards for **interaction-only** dialog-set binds — rows whose
//! `dialog_set_maps.dialog_id IS NULL` (CA02 / defect B3).
//!
//! Such a row exists to raise an indicator bit over an NPC (`!`, `?`, quest
//! glow) with no dialog behind the click; `Castle.py` binds seven of them.
//! The bind itself is wire-legal because the client-visible push is
//! `SGWSpawnableEntity.InteractionType(UINT64 TypeId)`
//! (`entities/defs/SGWSpawnableEntity.def:114-116`), a lone flags bitfield.
//! What these tests pin is the *other* half: when the player then clicks, the
//! server must not invent a dialog id to display.
//!
//! Bug shape guarded against: an `unwrap_or(0)` (or any other substituted id)
//! on the now-`Option<i32>` dialog field. That would send `onDialogDisplay`
//! with dialog 0 and open an empty dialog box on the client — worse than the
//! pre-CA02 no-op, because it also swallows the click that the `interact_tag`
//! chain was supposed to answer.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;

use super::super::{handle_initial_response, handle_interact};

/// `INT_AStoryMissionActive` — bit 24, the `!` indicator. The flag on Castle
/// dialog_set_map row 3062.
const INT_A_STORY_MISSION_ACTIVE: i64 = 16_777_216;

/// Player at the origin with an NPC two units away (inside
/// `MAX_INTERACT_DISTANCE`), the NPC carrying `template_id`.
fn stage_player_and_npc(template_id: i32) -> (crate::cell::space_manager::SpaceManager, u32) {
    let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
    let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
    let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(spaces_xml).unwrap();
    mgr.create_startup_spaces(cell_spaces_xml).unwrap();

    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let npc_id = mgr.allocate_npc_id();
    mgr.spawn_npc(npc_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(n) = mgr.get_entity_mut(npc_id) {
        n.template_id = Some(template_id);
    }
    (mgr, npc_id)
}

/// Clicking an NPC whose only per-player bind is interaction-only must not
/// display a dialog, and must report "no dialog opened" to the caller so the
/// static-interaction fall-through runs.
///
/// This is Sgt. Gerschon before mission 701 is accepted: row 3062 is bound to
/// put the `!` over his head, and the dialog comes from the
/// `interact_tag Castle_SgtGerschon` chain instead.
#[tokio::test]
async fn interact_with_only_null_dialog_bind_displays_nothing() {
    const TEMPLATE_GERSCHON: i32 = 149;
    const SET_MAP_3062: i32 = 3062;

    let (mut mgr, npc_id) = stage_player_and_npc(TEMPLATE_GERSCHON);
    if let Some(p) = mgr.get_entity_mut(1) {
        p.available_interactions.insert(
            TEMPLATE_GERSCHON,
            vec![(SET_MAP_3062, None, INT_A_STORY_MISSION_ACTIVE)],
        );
    }

    let (tx, mut rx) = mpsc::channel(16);
    let opened = handle_interact(1, npc_id, &tx, &mut mgr).await;

    assert_eq!(
        opened, None,
        "an interaction-only bind opens no dialog -- returning Some(0) here \
         would also fire a bogus dialog_open content event"
    );
    assert!(
        rx.try_recv().is_err(),
        "no wire frame may be emitted -- substituting a dialog id for the NULL \
         row opens an empty dialog box on the client"
    );
}

/// An interaction-only bind sitting *ahead* of a real one in the list must be
/// skipped, not treated as the answer.
///
/// This is the ordering that actually occurs in Castle: the `!` bind is
/// installed at `player_loaded`, a topic bind can be added later, and the list
/// is append-ordered. A `first()`-based lookup would return the flag-only
/// entry and silently drop the real dialog.
#[tokio::test]
async fn interact_skips_null_dialog_bind_and_uses_the_real_one() {
    const TEMPLATE_GERSCHON: i32 = 149;
    const SET_MAP_3062: i32 = 3062;
    const SET_MAP_3060: i32 = 3060;
    const DIALOG_2573: i32 = 2573;

    let (mut mgr, npc_id) = stage_player_and_npc(TEMPLATE_GERSCHON);
    if let Some(p) = mgr.get_entity_mut(1) {
        p.available_interactions.insert(
            TEMPLATE_GERSCHON,
            vec![
                (SET_MAP_3062, None, INT_A_STORY_MISSION_ACTIVE),
                (SET_MAP_3060, Some(DIALOG_2573), 0),
            ],
        );
    }

    let (tx, mut rx) = mpsc::channel(16);
    let opened = handle_interact(1, npc_id, &tx, &mut mgr).await;

    assert_eq!(
        opened,
        Some(DIALOG_2573),
        "the flag-only entry must be skipped over, not answer the click"
    );
    match rx.try_recv().expect("must emit onDialogDisplay") {
        CellToBaseMsg::EntityMethodCall { args, .. } => {
            let dialog_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
            assert_eq!(dialog_id, DIALOG_2573);
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }
}

/// `initialResponse` naming an interaction-only set map must bail rather than
/// display. The client can only send a `DialogSetMapID` it was offered, but
/// the server must not answer one that has no dialog behind it.
#[tokio::test]
async fn initial_response_on_null_dialog_bind_displays_nothing() {
    use cimmeria_content_engine::chain::ChainEngine;

    const TEMPLATE_GERSCHON: i32 = 149;
    const SET_MAP_3062: i32 = 3062;

    let (mut mgr, npc_id) = stage_player_and_npc(TEMPLATE_GERSCHON);
    if let Some(p) = mgr.get_entity_mut(1) {
        // Pinned by a preceding interact — so the only thing that can stop
        // the display is the NULL dialog itself, not a missing NPC id.
        p.last_interaction_target = Some(npc_id);
        p.available_interactions.insert(
            TEMPLATE_GERSCHON,
            vec![(SET_MAP_3062, None, INT_A_STORY_MISSION_ACTIVE)],
        );
    }

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();
    handle_initial_response(1, SET_MAP_3062, &engine, &tx, &mut mgr).await;

    assert!(
        rx.try_recv().is_err(),
        "initial_response must not emit onDialogDisplay for an interaction-only \
         bind -- a substituted dialog id opens an empty dialog"
    );
}

/// **Ordering pin (PR #661 review, item 5):** whichever order the two binds
/// were installed in, the click opens the dialog.
///
/// `handle_interact` scans with `find_map`, so a NULL row can never shadow a
/// row that carries a dialog. The companion above already covers NULL-first
/// (the order Castle actually produces, since the indicator is bound at
/// `player_loaded`); this pins the other order too, so a future change back to
/// "first entry wins" cannot pass by accident just because the seed happens to
/// order its binds favourably.
#[tokio::test]
async fn interact_finds_the_dialog_bind_in_either_order() {
    const TEMPLATE_GERSCHON: i32 = 149;
    const SET_MAP_3062: i32 = 3062;
    const SET_MAP_3060: i32 = 3060;
    const DIALOG_2573: i32 = 2573;

    let flag_only = (SET_MAP_3062, None, INT_A_STORY_MISSION_ACTIVE);
    let with_dialog = (SET_MAP_3060, Some(DIALOG_2573), 0i64);

    for (label, binds) in [
        ("null row first", vec![flag_only, with_dialog]),
        ("dialog row first", vec![with_dialog, flag_only]),
    ] {
        let (mut mgr, npc_id) = stage_player_and_npc(TEMPLATE_GERSCHON);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.available_interactions.insert(TEMPLATE_GERSCHON, binds);
        }

        let (tx, mut rx) = mpsc::channel(16);
        let opened = handle_interact(1, npc_id, &tx, &mut mgr).await;

        assert_eq!(
            opened,
            Some(DIALOG_2573),
            "{label}: the dialog-carrying bind must answer the click"
        );
        match rx.try_recv().expect("must emit onDialogDisplay") {
            CellToBaseMsg::EntityMethodCall { args, .. } => {
                let dialog_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                assert_eq!(dialog_id, DIALOG_2573, "{label}: wrong dialog on the wire");
            }
            other => panic!("{label}: expected EntityMethodCall, got {other:?}"),
        }
    }
}
