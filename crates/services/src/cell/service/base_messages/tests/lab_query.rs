//! Cell-loop request/reply tests for `BaseToCellMsg::LabQuery`.
//!
//! Same shape as the `LabConsoleExec` / `create_entity_instance` reply tests:
//! drive the message through `handle_base_message`, `.await` the `reply_tx`, and
//! assert on the captured [`LabQueryResult`]. These guard the live-research-lab
//! MCP `server_entity_get` / `server_entity_query` / `server_witnesses` paths
//! (issue #688): snapshots must copy out the right entity, the filter must
//! match, the query must stay under the size cap, and the witness report must
//! resolve both directions.

use super::*;
use tokio::sync::oneshot;

use crate::cell::messages::{
    LabEntityFilter, LabQuery, LabQueryReply, LabRadius, LabRadiusCenter, LAB_ENTITY_QUERY_CAP,
};

/// One non-instanced space, empty.
fn make_manager() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#,
    )
    .unwrap();
    mgr
}

/// Drive a `LabQuery` through the dispatch and return its reply.
async fn query(mgr: &mut SpaceManager, q: LabQuery) -> crate::cell::messages::LabQueryResult {
    let (tx, _rx) = mpsc::channel::<CellToBaseMsg>(8);
    let (reply_tx, reply_rx) = oneshot::channel();
    handle_base_message(
        BaseToCellMsg::LabQuery { query: q, reply_tx },
        &tx,
        mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;
    reply_rx.await.expect("LabQuery must always reply")
}

/// `EntityGet` returns a snapshot of the named entity; an unknown id yields a
/// `null` entity (never an error).
#[tokio::test]
async fn entity_get_snapshots_the_entity() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 1.0, 20.0], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(100) {
        e.is_player = true;
        e.character_name = Some("Tealc".to_string());
    }
    mgr.connect_entity(100);

    let reply = query(&mut mgr, LabQuery::EntityGet { entity_id: 100 })
        .await
        .expect("entity_get never errors");
    let LabQueryReply::Entity { entity } = reply else {
        panic!("expected Entity reply");
    };
    let snap = entity.expect("entity 100 exists");
    assert_eq!(snap.entity_id, 100);
    assert!(snap.is_player);
    assert_eq!(snap.name.as_deref(), Some("Tealc"));
    assert_eq!(snap.position, [10.0, 1.0, 20.0]);
    assert_eq!(snap.world_name, "Agnos");

    // Unknown id → Some(reply) with a null entity, not an Err.
    let reply = query(&mut mgr, LabQuery::EntityGet { entity_id: 999 })
        .await
        .expect("unknown id is not an error");
    let LabQueryReply::Entity { entity } = reply else {
        panic!("expected Entity reply");
    };
    assert!(entity.is_none(), "unknown entity snapshots to null");
}

/// `EntityQuery` honours the class-id filter and the radius filter.
#[tokio::test]
async fn entity_query_filters_by_class_and_radius() {
    let mut mgr = make_manager();
    // Player at origin.
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
    }
    mgr.connect_entity(1);
    // NPC near the player.
    mgr.create_entity(200, "Agnos", [5.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(200) {
        e.class_id = 0x04;
    }
    // NPC far away.
    mgr.create_entity(201, "Agnos", [500.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(201) {
        e.class_id = 0x04;
    }

    // class_id filter: both NPCs, not the player.
    let reply = query(
        &mut mgr,
        LabQuery::EntityQuery {
            filter: LabEntityFilter {
                class_id: Some(0x04),
                ..Default::default()
            },
        },
    )
    .await
    .unwrap();
    let LabQueryReply::Entities {
        entities,
        total_matched,
        capped,
    } = reply
    else {
        panic!("expected Entities reply");
    };
    assert_eq!(total_matched, 2, "two NPCs match class 0x04");
    assert!(!capped);
    let mut ids: Vec<u32> = entities.iter().map(|e| e.entity_id).collect();
    ids.sort_unstable();
    assert_eq!(ids, vec![200, 201]);

    // radius filter around the player: only the near NPC (5 units) is within 50.
    let reply = query(
        &mut mgr,
        LabQuery::EntityQuery {
            filter: LabEntityFilter {
                class_id: Some(0x04),
                radius: Some(LabRadius {
                    center: LabRadiusCenter::Entity(1),
                    radius: 50.0,
                }),
                ..Default::default()
            },
        },
    )
    .await
    .unwrap();
    let LabQueryReply::Entities { entities, .. } = reply else {
        panic!("expected Entities reply");
    };
    let ids: Vec<u32> = entities.iter().map(|e| e.entity_id).collect();
    assert_eq!(ids, vec![200], "only the near NPC is within radius 50");
}

/// A radius centered on a non-existent anchor fails the query rather than
/// silently matching nothing.
#[tokio::test]
async fn entity_query_bad_radius_anchor_errors() {
    let mut mgr = make_manager();
    let result = query(
        &mut mgr,
        LabQuery::EntityQuery {
            filter: LabEntityFilter {
                radius: Some(LabRadius {
                    center: LabRadiusCenter::Entity(424242),
                    radius: 10.0,
                }),
                ..Default::default()
            },
        },
    )
    .await;
    assert!(result.is_err(), "missing radius anchor must be an error");
}

/// The query result is capped at `LAB_ENTITY_QUERY_CAP` regardless of how many
/// entities match, and reports the true `total_matched` + `capped = true`.
///
/// Regression shape: an uncapped snapshot builder would materialise a snapshot
/// per entity on the cell loop thread and stall the tick on a dense space —
/// this asserts the bound holds and the caller is told the answer is truncated.
#[tokio::test]
async fn entity_query_enforces_size_cap() {
    let mut mgr = make_manager();
    let over = LAB_ENTITY_QUERY_CAP + 25;
    for i in 0..over as u32 {
        let id = 10_000 + i;
        mgr.create_entity(id, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(id) {
            e.class_id = 0x04;
        }
    }

    let reply = query(
        &mut mgr,
        LabQuery::EntityQuery {
            filter: LabEntityFilter {
                class_id: Some(0x04),
                ..Default::default()
            },
        },
    )
    .await
    .unwrap();
    let LabQueryReply::Entities {
        entities,
        total_matched,
        capped,
    } = reply
    else {
        panic!("expected Entities reply");
    };
    assert_eq!(
        entities.len(),
        LAB_ENTITY_QUERY_CAP,
        "returned snapshots never exceed the cap"
    );
    assert_eq!(
        total_matched, over,
        "total_matched reports the true pre-cap count"
    );
    assert!(capped, "capped flag set when the answer is truncated");
}

/// `Witnesses` resolves both directions: an NPC reports the players who see it
/// (`witnessed_by`), and a player reports whom it sees (`witnesses`).
#[tokio::test]
async fn witnesses_report_resolves_both_directions() {
    let mut mgr = make_manager();
    // Player + nearby NPC in the same space.
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(100) {
        e.is_player = true;
    }
    mgr.create_entity(900, "Agnos", [12.0, 0.0, 12.0], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(900) {
        e.class_id = 0x04;
    }
    mgr.connect_entity(100);
    // Populate witness sets.
    let _ = mgr.compute_aoi_changes();

    // NPC 900: who sees it → player 100.
    let reply = query(&mut mgr, LabQuery::Witnesses { entity_id: 900 })
        .await
        .unwrap();
    let LabQueryReply::Witnesses { report } = reply else {
        panic!("expected Witnesses reply");
    };
    assert_eq!(
        report.witnessed_by,
        vec![100],
        "player 100 witnesses the NPC"
    );

    // Player 100: whom it sees → includes NPC 900.
    let reply = query(&mut mgr, LabQuery::Witnesses { entity_id: 100 })
        .await
        .unwrap();
    let LabQueryReply::Witnesses { report } = reply else {
        panic!("expected Witnesses reply");
    };
    assert!(
        report.witnesses.contains(&900),
        "player's witness set includes the nearby NPC, got {:?}",
        report.witnesses
    );

    // Unknown entity → Err.
    let result = query(&mut mgr, LabQuery::Witnesses { entity_id: 55555 }).await;
    assert!(result.is_err(), "unknown entity witness query errors");
}
