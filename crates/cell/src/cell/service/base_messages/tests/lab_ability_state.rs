//! AB-L1: `LabQuery::AbilityState` answers with AB-T5's snapshot, and
//! `EntityGet` carries focus and every stat (an `EntityQuery` carries the
//! pools only, so a 256-entity reply stays small).

use super::*;
use std::time::Duration;
use tokio::sync::oneshot;

use cimmeria_entity::stats::{ACCURACY, DEFENSE, FOCUS};

use crate::cell::messages::{LabEntityFilter, LabQuery, LabQueryReply, LabQueryResult};

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
    mgr.create_entity(100, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    let e = mgr.get_entity_mut(100).unwrap();
    e.is_player = true;
    e.abilities
        .start_ability_cooldown(592, Duration::from_secs(30));
    e.stats.get_mut(FOCUS).unwrap().update(0, 120, 300);
    e.stats.get_mut(DEFENSE).unwrap().update(0, 15, 15);
    mgr
}

async fn query(mgr: &mut SpaceManager, q: LabQuery) -> LabQueryResult {
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

#[tokio::test]
async fn ab_l1_ability_state_returns_the_snapshot_or_null() {
    let mut mgr = make_manager();
    let reply = query(&mut mgr, LabQuery::AbilityState { entity_id: 100 })
        .await
        .expect("never errors");
    let LabQueryReply::AbilityState { state } = reply else {
        panic!("expected AbilityState reply, got {reply:?}");
    };
    let s = state.expect("entity 100 exists");
    assert_eq!(s.entity_id, 100);
    assert_eq!(s.cooldowns.len(), 1);
    assert_eq!(s.cooldowns[0].ability_id, 592);
    assert_eq!(s.stat(DEFENSE).map(|d| d.cur), Some(15));

    let reply = query(&mut mgr, LabQuery::AbilityState { entity_id: 999 })
        .await
        .expect("an unknown id is not an error");
    let LabQueryReply::AbilityState { state } = reply else {
        panic!("expected AbilityState reply");
    };
    assert!(state.is_none());
}

#[tokio::test]
async fn ab_l1_entity_get_carries_focus_and_every_stat() {
    let mut mgr = make_manager();
    let reply = query(&mut mgr, LabQuery::EntityGet { entity_id: 100 })
        .await
        .unwrap();
    let LabQueryReply::Entity { entity } = reply else {
        panic!("expected Entity reply");
    };
    let snap = entity.unwrap();
    assert_eq!((snap.focus_cur, snap.focus_max), (Some(120), Some(300)));
    let defense = snap.stats.iter().find(|s| s.stat_id == DEFENSE).unwrap();
    assert_eq!((defense.cur, defense.max), (15, 15));
    assert!(snap.stats.iter().any(|s| s.stat_id == ACCURACY));
    assert!(
        snap.stats.windows(2).all(|w| w[0].stat_id < w[1].stat_id),
        "sorted by stat id"
    );

    let reply = query(
        &mut mgr,
        LabQuery::EntityQuery {
            filter: LabEntityFilter::default(),
        },
    )
    .await
    .unwrap();
    let LabQueryReply::Entities { entities, .. } = reply else {
        panic!("expected Entities reply");
    };
    assert_eq!(entities[0].focus_cur, Some(120), "pools on a query too");
    assert!(
        entities[0].stats.is_empty(),
        "no full stat block per entity"
    );
}
