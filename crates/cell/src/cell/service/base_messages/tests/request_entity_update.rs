//! Tests for `BaseToCellMsg::RequestEntityUpdate`.
//!
//! Bug shape these guards prevent: the client's `requestEntityUpdate` (0x07)
//! cache-stamp handshake fires once per non-player entity on *every* normal
//! AoI entry, not just when something was genuinely dropped (see
//! `docs/reverse-engineering/findings/request-entity-update-cache-stamp.md`).
//! A handler that still re-emits `CREATE_ENTITY` + cascade for every in-AoI
//! id would double that traffic on every single entry. These guards pin: (a)
//! an in-AoI id produces no cell→base traffic at all, (b) an out-of-AoI id is
//! still refused (anti-probe, unchanged from PR #390), and (c) neither path
//! panics on an unknown witness.

use super::*;
use crate::test_support::LogCapture;
use cimmeria_common::EntityId;
use tracing::Level;

/// Normal on-enter case: witness has the target entity in AoI (the base
/// already sent it a full CREATE_ENTITY + cascade). The handler must answer
/// with nothing -- no EnteredAoI, no cascade. This is the regression guard
/// for "fixing only the parser would double-create on every normal AoI
/// entry" (issue #838's acceptance criteria); it must fail if the handler is
/// reverted to PR #390's unconditional re-emit.
#[tokio::test]
async fn request_entity_update_answers_nothing_for_witnessed_entity() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    // Witness (player) at (0,0,0)
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(w) = mgr.get_entity_mut(1) {
        w.is_player = true;
        w.player_id = Some(100);
        // Simulate the AoI tick having already added entity 42 to this witness.
        w.witnesses.insert(EntityId(42));
    }
    mgr.create_entity(42, "Castle_CellBlock", [5.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::RequestEntityUpdate {
            witness_id: 1,
            entity_ids: vec![42],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let count = std::iter::from_fn(|| rx.try_recv().ok()).count();
    assert_eq!(
        count, 0,
        "an id already in the witness's AoI must produce no cell\u{2192}base traffic \
         (the client already has full state from its original CREATE_ENTITY)"
    );
}

/// Security guard: witness does NOT have target in AoI. The request MUST be
/// refused -- otherwise a malicious client can probe any entity id and
/// receive its full state. Unchanged from PR #390.
#[tokio::test]
async fn request_entity_update_refuses_when_entity_not_in_witness_aoi() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(w) = mgr.get_entity_mut(1) {
        w.is_player = true;
        w.player_id = Some(100);
        // witnesses is intentionally empty — entity 42 exists but is NOT in
        // this player's AoI.
    }
    mgr.create_entity(42, "Castle_CellBlock", [500.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::RequestEntityUpdate {
            witness_id: 1,
            entity_ids: vec![42],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let count = std::iter::from_fn(|| rx.try_recv().ok()).count();
    assert_eq!(
        count, 0,
        "out-of-AoI request must produce no cell\u{2192}base traffic (anti-probe)"
    );
}

/// Negative log: an out-of-AoI request is refused with a WARN carrying
/// `reason = "not_in_witness_aoi"` plus `entity_id`/`witness_id` -- per
/// `docs/architecture/negative-logging-convention.md`, this must be
/// greppable, not a silent drop.
#[tokio::test]
async fn request_entity_update_out_of_aoi_logs_a_negative_event() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(w) = mgr.get_entity_mut(1) {
        w.is_player = true;
        w.account_id = Some(10);
        w.player_id = Some(100);
    }
    mgr.create_entity(42, "Castle_CellBlock", [500.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let guard = LogCapture::install();

    handle_base_message(
        BaseToCellMsg::RequestEntityUpdate {
            witness_id: 1,
            entity_ids: vec![42],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let row = guard
        .find_event(
            Level::WARN,
            "id outside witness's AoI",
            "not_in_witness_aoi",
        )
        .unwrap_or_else(|| panic!("no not_in_witness_aoi row; saw {:#?}", guard.all()));
    assert!(row.has_field("witness_id", "1"));
    assert!(row.has_field("entity_id", "42"));
    assert!(row.has_field("account_id", "10"));
    assert!(row.has_field("player_id", "100"));
}

/// Mixed request: one entity in AoI, one out of AoI, one unknown id. None of
/// them should produce any cell→base traffic — the in-AoI id is acknowledged
/// silently, the other two are refused/skipped, all without panic.
#[tokio::test]
async fn request_entity_update_produces_no_traffic_for_a_mixed_request() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(w) = mgr.get_entity_mut(1) {
        w.is_player = true;
        w.player_id = Some(100);
        w.witnesses.insert(EntityId(42)); // 42 is in AoI
                                          // 43 is NOT in AoI; 99 doesn't exist at all.
    }
    mgr.create_entity(42, "Castle_CellBlock", [5.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(43, "Castle_CellBlock", [500.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::RequestEntityUpdate {
            witness_id: 1,
            entity_ids: vec![42, 43, 99],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let count = std::iter::from_fn(|| rx.try_recv().ok()).count();
    assert_eq!(
        count, 0,
        "no id in a mixed request should produce cell\u{2192}base traffic: the in-AoI id \
         is acknowledged silently, the others are refused"
    );
}

/// Unknown witness entity (e.g. a request from a stale connection) drops
/// the whole request without panic and without emitting anything.
#[tokio::test]
async fn request_entity_update_drops_when_witness_missing() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    // No entity 999 exists.

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::RequestEntityUpdate {
            witness_id: 999,
            entity_ids: vec![42],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let count = std::iter::from_fn(|| rx.try_recv().ok()).count();
    assert_eq!(
        count, 0,
        "unknown witness must produce no cell\u{2192}base traffic"
    );
}

/// DoS guard: a payload larger than `MAX_REQUEST_ENTITIES` (64) is truncated
/// to the cap and still produces no cell→base traffic (every id is in AoI,
/// which now means "acknowledged silently", not "re-created").
#[tokio::test]
async fn request_entity_update_truncates_request_above_cap() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    const SPAM_COUNT: usize = 200;
    if let Some(w) = mgr.get_entity_mut(1) {
        w.is_player = true;
        w.player_id = Some(100);
        for i in 0..SPAM_COUNT as u32 {
            w.witnesses.insert(EntityId(2000 + i as i32));
        }
    }
    for i in 0..SPAM_COUNT as u32 {
        mgr.create_entity(2000 + i, "Castle_CellBlock", [1.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
    }

    let (tx, mut rx) = mpsc::channel(SPAM_COUNT + 16);
    let engine = ChainEngine::new();
    let entity_ids: Vec<u32> = (0..SPAM_COUNT as u32).map(|i| 2000 + i).collect();

    handle_base_message(
        BaseToCellMsg::RequestEntityUpdate {
            witness_id: 1,
            entity_ids,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let count = std::iter::from_fn(|| rx.try_recv().ok()).count();
    assert_eq!(
        count, 0,
        "a truncated, all-in-AoI request must still produce no cell\u{2192}base traffic"
    );
}

/// The acknowledgement names the entity the client just created. That id is
/// the server-side proof that the client has an entity (the client sends the
/// handshake for every NPC it creates), which a "server sent it but the
/// client never showed it" investigation needs per entity, not as a count.
#[tokio::test]
async fn request_entity_update_acknowledgement_names_the_entity() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(w) = mgr.get_entity_mut(1) {
        w.is_player = true;
        w.witnesses.insert(EntityId(42));
    }
    mgr.create_entity(42, "Castle_CellBlock", [1.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let guard = LogCapture::install();

    handle_base_message(
        BaseToCellMsg::RequestEntityUpdate {
            witness_id: 1,
            entity_ids: vec![42],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let row = guard
        .all()
        .into_iter()
        .find(|r| {
            r.message
                .as_deref()
                .is_some_and(|m| m.contains("RequestEntityUpdate acknowledged"))
        })
        .unwrap_or_else(|| panic!("no acknowledgement row; saw {:#?}", guard.all()));
    assert!(row.has_field("entity_ids", "[42]"), "{row:?}");
    assert!(row.has_field("known", "1"));
}
