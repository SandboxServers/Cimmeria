use super::*;

#[tokio::test]
async fn minigame_result_victory_fires_on_victory_chains() {
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        // Set HP below max so ChangeStat +10 actually advances
        if let Some(h) = e.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            h.update(0, 50, 100);
        }
        e.stats.clear_dirty();
    }
    mgr.connect_entity(1);

    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        action_delays: Vec::new(),
        id: 9999,
        name: "test-victory-chain".into(),
        enabled: true,
        trigger: Trigger::OnInteractTag {
            entity_tag: "__unused__".into(),
        },
        conditions: vec![],
        actions: vec![Action::ChangeStat {
            stat_id: cimmeria_entity::stats::HEALTH,
            min: None,
            max: None,
            use_ammo_stat: None,
            set_to_max: None,
            amount: Some(10),
        }],
        priority: 1,
    });

    let (tx, mut rx) = mpsc::channel(8);
    handle_base_message(
        BaseToCellMsg::MinigameResult {
            entity_id: 1,
            result_code: 1, // victory
            on_victory_chains: vec![9999],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    // ChangeStat emits onStatUpdate via the executor
    let msg = rx
        .try_recv()
        .expect("victory chain must fire and produce onStatUpdate");
    match msg {
        crate::cell::messages::CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            ..
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(method_index, crate::mercury::method_idx::ON_STAT_UPDATE);
        }
        other => panic!("expected EntityMethodCall(onStatUpdate), got {other:?}"),
    }
}

/// Non-victory results must not fire `on_victory_chains`.
///
/// Parameterised over the two real non-victory codes. The fixture used to
/// pass 0 while calling it "defeat", which left the CA04 distinction
/// unpinned: 0 is `MinigameCanceled` (the session ended with no outcome) and
/// 2 is `MinigameDefeat` (the player lost). The handler treats both as inert
/// today, but they are different events upstream, and a test that conflates
/// them would not notice if one started firing chains.
#[tokio::test]
async fn minigame_result_canceled_does_not_fire_chains() {
    assert_non_victory_result_fires_no_chains(0).await;
}

#[tokio::test]
async fn minigame_result_defeat_does_not_fire_chains() {
    assert_non_victory_result_fires_no_chains(2).await;
}

async fn assert_non_victory_result_fires_no_chains(result_code: u8) {
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
    }
    mgr.connect_entity(1);

    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        action_delays: Vec::new(),
        id: 9999,
        name: "test-victory-chain".into(),
        enabled: true,
        trigger: Trigger::OnInteractTag {
            entity_tag: "__unused__".into(),
        },
        conditions: vec![],
        actions: vec![Action::ChangeStat {
            stat_id: cimmeria_entity::stats::HEALTH,
            min: None,
            max: None,
            use_ammo_stat: None,
            set_to_max: None,
            amount: Some(10),
        }],
        priority: 1,
    });

    let (tx, mut rx) = mpsc::channel(8);
    handle_base_message(
        BaseToCellMsg::MinigameResult {
            entity_id: 1,
            result_code,
            on_victory_chains: vec![9999],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    assert!(
        rx.try_recv().is_err(),
        "result_code {result_code} is not victory (1), so it must not fire victory chains"
    );
}
