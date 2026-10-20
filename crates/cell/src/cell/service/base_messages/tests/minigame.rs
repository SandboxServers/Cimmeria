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
        once: false,
    });

    let (tx, mut rx) = mpsc::channel(8);
    handle_base_message(
        BaseToCellMsg::MinigameResult {
            entity_id: 1,
            player_id: 100,
            result_code: 1, // victory
            on_victory_chains: vec![9999],
            on_failure_chains: vec![],
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

/// MG-06 / D-MG15: every result code releases the minigame hold, including
/// the launch-failure and expiry code 3. Revert: drop the
/// `release_minigame_hold` call from `handle_minigame_result` and the bits
/// stay set for every code.
#[tokio::test]
async fn every_minigame_result_code_releases_the_hold() {
    use cimmeria_wire::state_field::{BSF_MOVEMENT_LOCK, BSF_PLAYING_MINIGAME};
    const BOTH: u32 = BSF_PLAYING_MINIGAME | BSF_MOVEMENT_LOCK;

    for code in 0u8..=4 {
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        mgr.get_entity_mut(1).unwrap().is_player = true;
        mgr.connect_entity(1);
        let (tx, mut rx) = mpsc::channel(8);
        cimmeria_cell_world::cell::minigame_hold::hold_for_minigame(1, &tx, &mut mgr).await;
        assert_eq!(mgr.get_entity(1).unwrap().state_field & BOTH, BOTH);
        while rx.try_recv().is_ok() {}

        handle_base_message(
            BaseToCellMsg::MinigameResult {
                entity_id: 1,
                player_id: 0,
                result_code: code,
                on_victory_chains: vec![],
                on_failure_chains: vec![],
            },
            &tx,
            &mut mgr,
            &ChainEngine::new(),
            &[],
        )
        .await;

        assert_eq!(
            mgr.get_entity(1).unwrap().state_field & BOTH,
            0,
            "result code {code} must release the hold"
        );
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
        once: false,
    });

    let (tx, mut rx) = mpsc::channel(8);
    handle_base_message(
        BaseToCellMsg::MinigameResult {
            entity_id: 1,
            player_id: 100,
            result_code,
            on_victory_chains: vec![9999],
            on_failure_chains: vec![],
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

/// TG-MG-09 (CS-MGF2): a victory for an entity the cell does not hold fires
/// nothing (it used to fire with player 0) and the WARN row says why.
#[tokio::test]
async fn minigame_victory_for_unknown_entity_warns() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = SpaceManager::new(1);
    let (tx, mut rx) = mpsc::channel(8);
    let mut engine = ChainEngine::new();
    engine.register_chain(counter_chain(9999, vec![]));
    handle_base_message(
        BaseToCellMsg::MinigameResult {
            entity_id: 77,
            player_id: 100,
            result_code: 1, // victory, but entity 77 was never created
            on_victory_chains: vec![9999, 9998],
            on_failure_chains: vec![],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    assert!(
        rx.try_recv().is_err(),
        "no chain may fire for a missing entity"
    );
    let warn = capture
        .find_message(Level::WARN, "no chain fires")
        .expect("a victory for a missing entity must log a WARN row");
    for (key, want) in [
        ("reason", "entity_missing"),
        ("entity_id", "77"),
        ("expected_player_id", "100"),
    ] {
        assert!(
            warn.has_field(key, want),
            "the missing-entity row must carry {key}={want}; got {warn:#?}"
        );
    }
}

/// A chain that bumps counter `callback` on the entity. With `conditions` it
/// is guarded, and carries the custom-event trigger the guarded path resolves.
fn counter_chain(
    id: i64,
    conditions: Vec<cimmeria_content_engine::conditions::Condition>,
) -> cimmeria_content_engine::chain::Chain {
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;
    Chain {
        action_delays: Vec::new(),
        id,
        name: "mgf2-victory-chain".into(),
        enabled: true,
        trigger: Trigger::OnCustomEvent {
            event_name: format!("__direct_invoke_{id}"),
        },
        conditions,
        actions: vec![Action::IncrementCounter {
            counter_name: "callback".into(),
            amount: 1,
        }],
        priority: 1,
        once: false,
    }
}

/// Entity 1 (player_id 100) in `world` ("Castle" = world id 8, "Elsewhere" = 9)
/// with mission 1520 active at step 4453.
fn mgf2_mgr(world: &str) -> SpaceManager {
    use crate::cell::spawner::WorldRow;
    use cimmeria_entity::missions::MissionInstance;
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
        <Space WorldName="Elsewhere" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
        </Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /><Space WorldName="Elsewhere" /></Spaces>"#,
    )
    .unwrap();
    mgr.stamp_world_rows(&std::collections::HashMap::from([
        ("Castle".to_string(), WorldRow::enforcing(8)),
        ("Elsewhere".to_string(), WorldRow::enforcing(9)),
    ]));
    mgr.create_entity(1, world, [0.0; 3], [0.0; 3]).unwrap();
    let e = mgr.get_entity_mut(1).unwrap();
    e.is_player = true;
    e.player_id = Some(100);
    e.missions
        .add_mission(MissionInstance::new(1520, 4453, vec![]));
    mgr.connect_entity(1);
    mgr
}

fn callback_count(mgr: &SpaceManager) -> i32 {
    mgr.get_entity(1)
        .and_then(|e| e.counters.get("callback"))
        .copied()
        .unwrap_or(0)
}

fn set_step(mgr: &mut SpaceManager, step: i32) {
    mgr.get_entity_mut(1)
        .unwrap()
        .missions
        .get_mission_mut(1520)
        .unwrap()
        .current_step_id = Some(step);
}

async fn send_victory(
    mgr: &mut SpaceManager,
    engine: &ChainEngine,
    receipt_player_id: i32,
    chains: Vec<i64>,
) {
    let (tx, _rx) = mpsc::channel(8);
    handle_base_message(
        BaseToCellMsg::MinigameResult {
            entity_id: 1,
            player_id: receipt_player_id,
            result_code: 1,
            on_victory_chains: chains,
            on_failure_chains: vec![],
        },
        &tx,
        mgr,
        engine,
        &[],
    )
    .await;
}

/// The receipt names player 200 but the slot holds player 100 (a reused
/// slot): nothing fires, and the WARN names both ids.
#[tokio::test]
async fn minigame_victory_for_another_player_fires_nothing() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = mgf2_mgr("Castle");
    let mut engine = ChainEngine::new();
    engine.register_chain(counter_chain(9999, vec![]));
    send_victory(&mut mgr, &engine, 200, vec![9999]).await;

    assert_eq!(callback_count(&mgr), 0, "another player's receipt fired");
    let warn = capture
        .find_message(Level::WARN, "does not match the player")
        .expect("a mismatched receipt must log a WARN row");
    for (key, want) in [
        ("reason", "player_mismatch"),
        ("entity_id", "1"),
        ("expected_player_id", "200"),
        ("actual_player_id", "100"),
    ] {
        assert!(
            warn.has_field(key, want),
            "missing {key}={want}; got {warn:#?}"
        );
    }
}

/// A receipt carrying player 0 never matches, though the entity has a player.
#[tokio::test]
async fn minigame_victory_with_player_zero_fires_nothing() {
    let mut mgr = mgf2_mgr("Castle");
    let mut engine = ChainEngine::new();
    engine.register_chain(counter_chain(9999, vec![]));
    send_victory(&mut mgr, &engine, 0, vec![9999]).await;
    assert_eq!(callback_count(&mgr), 0, "player 0 receipt fired a chain");
}

/// A conditioned victory chain runs through the guarded path: it fires in
/// its world and not in another.
#[tokio::test]
async fn minigame_victory_conditioned_chain_is_guarded() {
    use cimmeria_content_engine::conditions::{ComparisonOp, Condition};
    let mut engine = ChainEngine::new();
    engine.register_chain(counter_chain(
        9999,
        vec![Condition::World {
            operator: ComparisonOp::Eq,
            world_id: 8,
        }],
    ));

    let mut elsewhere = mgf2_mgr("Elsewhere");
    send_victory(&mut elsewhere, &engine, 100, vec![9999]).await;
    assert_eq!(callback_count(&elsewhere), 0, "world condition ignored");

    let mut castle = mgf2_mgr("Castle");
    send_victory(&mut castle, &engine, 100, vec![9999]).await;
    assert_eq!(
        callback_count(&castle),
        1,
        "must fire when the condition holds"
    );
}

/// A step-gated chain grants once: a replayed victory finds the step moved on.
#[tokio::test]
async fn minigame_victory_step_gated_chain_grants_once() {
    use cimmeria_content_engine::conditions::{ComparisonOp, Condition, StepStatusValue};
    let mut engine = ChainEngine::new();
    engine.register_chain(counter_chain(
        9999,
        vec![Condition::StepStatus {
            mission_id: 1520,
            step_id: 4453,
            operator: ComparisonOp::Eq,
            expected_status: StepStatusValue::Active,
        }],
    ));

    let mut mgr = mgf2_mgr("Castle");
    set_step(&mut mgr, 4454);
    send_victory(&mut mgr, &engine, 100, vec![9999]).await;
    assert_eq!(callback_count(&mgr), 0, "inactive step fired");

    set_step(&mut mgr, 4453);
    send_victory(&mut mgr, &engine, 100, vec![9999]).await;
    assert_eq!(callback_count(&mgr), 1);

    // The chain's real effect moves the step on; the same message again is inert.
    set_step(&mut mgr, 4454);
    send_victory(&mut mgr, &engine, 100, vec![9999]).await;
    assert_eq!(callback_count(&mgr), 1, "a replayed victory granted twice");
}

/// Rule 5 + Rule 6: the cell's "Minigame result" row, the one a reader
/// checks first when a hack or bypass "did nothing", names the player, not
/// only the entity slot it happened to hold.
#[tokio::test]
async fn minigame_result_row_names_the_player() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
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
        e.account_id = Some(6);
        e.player_id = Some(100);
        e.stamp_log_names(Some("Daniel"), Some("sgc_daniel"));
    }

    let (tx, _rx) = mpsc::channel(8);
    handle_base_message(
        BaseToCellMsg::MinigameResult {
            entity_id: 1,
            // Deliberately != the entity's player_id (100): the receipt id
            // and the held id must be distinguishable in the row.
            player_id: 101,
            result_code: 2, // defeat: no chains to fire
            on_victory_chains: vec![],
            on_failure_chains: vec![],
        },
        &tx,
        &mut mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;

    let event = capture
        .find_message(Level::INFO, "Minigame result")
        .expect("every minigame outcome logs one result row");
    for (key, want) in [
        ("entity_name", "Daniel"),
        ("account_id", "6"),
        ("account_name", "sgc_daniel"),
        ("player_id", "100"),
        ("player_name", "Daniel"),
        ("receipt_player_id", "101"),
    ] {
        assert!(
            event.has_field(key, want),
            "the minigame result row must name its player: expected \
             {key}={want}; got {event:#?}"
        );
    }
}

/// Which of the two chains (victory 9999, failure 9998) a result code fires.
///
/// Both chains carry a `ChangeStat` HEALTH +10, so a fired chain shows up as
/// an `onStatUpdate` on the channel. The two chains use different amounts so
/// the test can tell them apart: victory +10, failure +5 on a 50/100 pool.
async fn fired_amounts(result_code: u8) -> Vec<i32> {
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
        if let Some(h) = e.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            h.update(0, 50, 100);
        }
        e.stats.clear_dirty();
    }
    mgr.connect_entity(1);

    let mut engine = ChainEngine::new();
    for (id, amount) in [(9999_i64, 10), (9998_i64, 5)] {
        engine.register_chain(Chain {
            action_delays: Vec::new(),
            id,
            name: format!("test-chain-{id}"),
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
                amount: Some(amount),
            }],
            priority: 1,
            once: false,
        });
    }

    let (tx, mut rx) = mpsc::channel(8);
    handle_base_message(
        BaseToCellMsg::MinigameResult {
            entity_id: 1,
            player_id: 100,
            result_code,
            on_victory_chains: vec![9999],
            on_failure_chains: vec![9998],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    // The ChangeStat executor emits one onStatUpdate per fired chain; count
    // them and read the resulting HEALTH to tell which chain ran.
    let mut updates = 0;
    while rx.try_recv().is_ok() {
        updates += 1;
    }
    let cur = mgr
        .get_entity(1)
        .and_then(|e| e.stats.get(cimmeria_entity::stats::HEALTH))
        .map(|s| s.cur)
        .unwrap();
    let mut out = Vec::new();
    if updates > 0 {
        out.push(cur - 50);
    }
    out
}

/// D-MG5: failure (2) and defeated (4) fire `on_failure_chains` and not the
/// victory chains. Revert: drop the failure branch in `handle_minigame_result`
/// and both rows yield an empty list.
#[tokio::test]
async fn minigame_failure_and_defeated_fire_only_failure_chains() {
    assert_eq!(
        fired_amounts(2).await,
        vec![5],
        "failure fires chain 9998 only"
    );
    assert_eq!(
        fired_amounts(4).await,
        vec![5],
        "defeated fires chain 9998 only"
    );
}

/// Victory fires only the victory chain, canceled (0) and interrupted (3)
/// fire neither.
#[tokio::test]
async fn minigame_victory_fires_only_victory_and_inert_codes_fire_nothing() {
    assert_eq!(
        fired_amounts(1).await,
        vec![10],
        "victory fires chain 9999 only"
    );
    assert!(fired_amounts(0).await.is_empty(), "canceled is inert");
    assert!(fired_amounts(3).await.is_empty(), "interrupted is inert");
}
