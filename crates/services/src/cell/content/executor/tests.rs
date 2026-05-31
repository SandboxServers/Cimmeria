//! Tests for `executor::execute_actions` — exercise the per-action-family
//! handlers via the public match dispatch.

use super::*;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};

fn make_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr
}

/// Build a connected player at id=1 with HEALTH at `cur/max`, dirty
/// flags cleared so a single `serialize_dirty` reads only what
/// `Action::ChangeStat` writes. Used by the heal-action tests below.
fn make_player_with_health(mgr: &mut SpaceManager, cur: i32, max: i32) {
    use cimmeria_entity::stats::HEALTH;
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(42);
        if let Some(h) = e.stats.get_mut(HEALTH) {
            h.update(0, cur, max);
        }
        e.stats.clear_dirty();
    }
    mgr.connect_entity(1);
}

/// `change_stat { amount: +500 }` is the canonical heal-on-use shape
/// (Health Slappack TC1, chain 4001). Three things must hold: HP
/// advances by exactly the delta when room is available, the change
/// is broadcast as a single onStatUpdate carrying the new HEALTH
/// value, and the entity's dirty state is drained so a follow-up
/// serialize doesn't re-emit the same stat.
#[tokio::test]
async fn change_stat_amount_advances_health_and_emits_on_stat_update() {
    use cimmeria_entity::stats::HEALTH;

    let mut mgr = make_space_mgr();
    make_player_with_health(&mut mgr, 200, 1000);

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            4001,
            Action::ChangeStat {
                stat_id: HEALTH,
                min: None,
                max: None,
                use_ammo_stat: None,
                set_to_max: None,
                amount: Some(500),
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    // Entity state.
    let entity = mgr.get_entity(1).unwrap();
    assert_eq!(
        entity.stats.get(HEALTH).unwrap().cur,
        700,
        "HEALTH.cur must advance by +500"
    );
    assert!(
        !entity.stats.has_dirty(),
        "executor must clear dirty after sending — otherwise the next \
         serialize would re-emit the same stat"
    );

    // Wire frame.
    let msg = rx.try_recv().expect("expected onStatUpdate");
    match msg {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(method_index, crate::mercury::method_idx::ON_STAT_UPDATE);
            let count = u32::from_le_bytes([args[0], args[1], args[2], args[3]]) as usize;
            let mut found = false;
            for i in 0..count {
                let off = 4 + i * 16;
                let stat_id =
                    i32::from_le_bytes([args[off], args[off + 1], args[off + 2], args[off + 3]]);
                if stat_id == HEALTH {
                    let cur = i32::from_le_bytes([
                        args[off + 8],
                        args[off + 9],
                        args[off + 10],
                        args[off + 11],
                    ]);
                    assert_eq!(cur, 700, "wire payload carries the post-heal HEALTH.cur");
                    found = true;
                }
            }
            assert!(found, "onStatUpdate payload must include HEALTH");
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }

    assert!(rx.try_recv().is_err(), "no further messages expected");
}

/// Heal must clamp at `max` — a slappack on a near-full bar can't
/// push the wire payload to `cur > max`. The clamp is `Stat::change`'s
/// responsibility; this guard pins that the executor doesn't bypass
/// it (e.g., by writing `cur` directly).
#[tokio::test]
async fn change_stat_amount_clamps_to_max() {
    use cimmeria_entity::stats::HEALTH;

    let mut mgr = make_space_mgr();
    make_player_with_health(&mut mgr, 950, 1000);

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            4001,
            Action::ChangeStat {
                stat_id: HEALTH,
                min: None,
                max: None,
                use_ammo_stat: None,
                set_to_max: None,
                amount: Some(500),
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let h = mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap();
    assert_eq!(
        h.cur, 1000,
        "heal clamps to max even on +500 over a 50-room bar"
    );
    assert!(h.cur <= h.max, "wire invariant cur <= max preserved");
}

/// Negative `amount` damages the stat. The same code path serves
/// debuff/poison-style chains; if the executor ever silently ignored
/// negative deltas (e.g., `cur += amount.max(0)`) this guard fails.
#[tokio::test]
async fn change_stat_negative_amount_damages_stat() {
    use cimmeria_entity::stats::HEALTH;

    let mut mgr = make_space_mgr();
    make_player_with_health(&mut mgr, 800, 1000);

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            4001,
            Action::ChangeStat {
                stat_id: HEALTH,
                min: None,
                max: None,
                use_ammo_stat: None,
                set_to_max: None,
                amount: Some(-300),
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    assert_eq!(
        mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap().cur,
        500,
        "negative amount must subtract from cur"
    );
}

/// `set_to_max: true` snaps `cur` to `max` regardless of the prior
/// value. Pairs with the legacy reload-effect chain (effect_id-driven,
/// `set_to_max=true` on the ammo stat) so the bounds-modifying path
/// stays exercised alongside the new `amount` path.
#[tokio::test]
async fn change_stat_set_to_max_snaps_current_to_max() {
    use cimmeria_entity::stats::HEALTH;

    let mut mgr = make_space_mgr();
    make_player_with_health(&mut mgr, 100, 1000);

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            4001,
            Action::ChangeStat {
                stat_id: HEALTH,
                min: None,
                max: None,
                use_ammo_stat: None,
                set_to_max: Some(true),
                amount: None,
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let h = mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap();
    assert_eq!(h.cur, 1000, "set_to_max snaps cur to max");
    assert_eq!(h.max, 1000, "max unchanged");
}

/// `use_ammo_stat=true` (and its legacy `stat_id=-1` sentinel form,
/// used by the seeded Reload effect chain 2011) must skip cleanly
/// rather than warn-and-no-op on a missing stat lookup. Pin the
/// no-side-effects shape: HP unchanged, no wire message sent. When
/// active-ammo-slot resolution lands, this test should be replaced
/// with one that asserts the resolved ammo stat actually mutates.
#[tokio::test]
async fn change_stat_with_use_ammo_stat_skips_cleanly() {
    use cimmeria_entity::stats::HEALTH;

    let mut mgr = make_space_mgr();
    make_player_with_health(&mut mgr, 500, 1000);

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![
            (
                2011,
                Action::ChangeStat {
                    stat_id: -1, // legacy sentinel for "ammo stat"
                    min: None,
                    max: None,
                    use_ammo_stat: Some(true),
                    set_to_max: Some(true),
                    amount: None,
                },
            ),
            (
                2011,
                Action::ChangeStat {
                    stat_id: -1, // negative-only path also skips
                    min: None,
                    max: None,
                    use_ammo_stat: None,
                    set_to_max: None,
                    amount: Some(50),
                },
            ),
        ],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let h = mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap();
    assert_eq!(
        h.cur, 500,
        "HEALTH untouched — ammo-stat path is unimplemented"
    );
    assert!(
        rx.try_recv().is_err(),
        "no onStatUpdate must fire when the ammo-stat path skips"
    );
}

/// Regression for #95: `Action::RemoveItem` must route through the new
/// `RemoveInventoryItemByType` cell→base RPC, not the silently-ignored
/// stub it used to be. Locks in the chain-driven removal path that
/// chain 1034 (FindAmbernol consume) depends on.
#[tokio::test]
async fn remove_item_action_emits_remove_inventory_by_type() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            1034,
            Action::RemoveItem {
                item_id: 19,
                count: 1,
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let msg = rx.try_recv().expect("expected RemoveInventoryItemByType");
    match msg {
        CellToBaseMsg::RemoveInventoryItemByType {
            entity_id,
            player_id,
            type_id,
            count,
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(player_id, 42);
            assert_eq!(type_id, 19);
            assert_eq!(count, 1);
        }
        other => panic!("expected RemoveInventoryItemByType, got {:?}", other),
    }
}

/// `Action::IncrementCounter` mutates `entity.counters`. Previously
/// a stub that only logged; now load-bearing for kill-counter
/// missions like Mess Hall (counter `messhall_kills`) and Hallway05
/// (`hallway05_kills`). Pin the new-key initialization path:
/// missing entry → 0, then add `amount`.
#[tokio::test]
async fn increment_counter_initializes_and_adds_amount() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            1085,
            Action::IncrementCounter {
                counter_name: "messhall_kills".to_string(),
                amount: 1,
            },
        )],
    };
    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let entity = mgr.get_entity(1).expect("entity must still exist");
    assert_eq!(
        entity.counters.get("messhall_kills"),
        Some(&1),
        "new counter must initialize at 0 and add `amount` (1)",
    );
}

/// `Action::IncrementCounter` on an existing counter adds to the
/// stored value rather than overwriting. The Mess Hall mission
/// design depends on this: each guard kill increments the same
/// counter; the second kill must read the first's stored value
/// for the completion chain's `gte (target - 1)` condition to
/// fire on the right kill.
#[tokio::test]
async fn increment_counter_adds_to_existing_value() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(1)
        .unwrap()
        .counters
        .insert("messhall_kills".to_string(), 1);

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            1086,
            Action::IncrementCounter {
                counter_name: "messhall_kills".to_string(),
                amount: 1,
            },
        )],
    };
    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    assert_eq!(
        mgr.get_entity(1).unwrap().counters.get("messhall_kills"),
        Some(&2),
        "second increment must add to the stored value, not overwrite",
    );
}

/// `Action::ResetCounter` removes the entry entirely. Subsequent
/// `Condition::Counter` reads see the missing-key default of 0.
/// Used by the Mess Hall completion chain (1087) so a re-accept
/// of mission 681 (e.g., the same player respawning into a fresh
/// instance) starts the counter clean.
#[tokio::test]
async fn reset_counter_clears_entry() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(1)
        .unwrap()
        .counters
        .insert("messhall_kills".to_string(), 2);

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            1087,
            Action::ResetCounter {
                counter_name: "messhall_kills".to_string(),
            },
        )],
    };
    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    assert!(
        !mgr.get_entity(1)
            .unwrap()
            .counters
            .contains_key("messhall_kills"),
        "reset must remove the entry — leaving a 0 entry would surface \
         via populate_counters_context as `counter_messhall_kills = 0` \
         rather than the missing-key default, masking a re-acceptance",
    );
}

/// `Action::CrossWorldTeleport` must produce exactly one
/// `CellToBaseMsg::GateTravel` send carrying the right world name and
/// position, with `destination_ring_id: None` (the discriminator that
/// tells the base side NOT to emit `BaseToCellMsg::AdvanceRingDestination`
/// — there's no destination ring FSM to advance for a chain-driven
/// hop). Pinning this guards against the action accidentally being
/// rerouted through the `Action::Teleport` (same-space) path.
#[tokio::test]
async fn cross_world_teleport_action_emits_gate_travel_with_no_ring_id() {
    use crate::cell::messages::CellToBaseMsg;

    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(42);
    }
    mgr.connect_entity(1);

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            1109,
            Action::CrossWorldTeleport {
                world_name: "Castle".to_string(),
                position: [466.365, 70.397, 991.466],
            },
        )],
    };
    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    // Drain the channel and find the GateTravel message. The action
    // also flushes dirty bandolier ammo before sending GateTravel —
    // we don't assert on that here (the player has no dirty ammo) but
    // it's why we drain rather than just `try_recv`.
    let mut gate_travel: Option<(u32, String, [f32; 3], [f32; 3], Option<i32>)> = None;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::GateTravel {
            entity_id,
            target_world_name,
            position,
            rotation,
            destination_ring_id,
        } = msg
        {
            gate_travel = Some((
                entity_id,
                target_world_name,
                position,
                rotation,
                destination_ring_id,
            ));
        }
    }
    let (eid, world, pos, rot, ring_id) =
        gate_travel.expect("CrossWorldTeleport action must produce a GateTravel send");
    assert_eq!(
        eid, 1,
        "GateTravel.entity_id must be the player's entity_id"
    );
    assert_eq!(world, "Castle", "GateTravel.target_world_name must match");
    assert!(
        (pos[0] - 466.365).abs() < 0.001
            && (pos[1] - 70.397).abs() < 0.001
            && (pos[2] - 991.466).abs() < 0.001,
        "GateTravel.position must be propagated verbatim; got {pos:?}"
    );
    assert_eq!(rot, [0.0, 0.0, 0.0], "rotation defaults to identity");
    assert_eq!(
        ring_id, None,
        "destination_ring_id must be None — chain-driven cross-world hop \
         skips the destination ring FSM, so base must not emit \
         AdvanceRingDestination"
    );

    // The cell-side entity must be torn down on this world before the
    // GateTravel send (matches the ring's TeleportCrossWorld arm
    // ordering). Without this, the player exists in two worlds at once
    // until base destroys via RESET_ENTITIES.
    assert!(
        mgr.get_entity(1).is_none(),
        "cell entity must be destroyed locally before cross-world hop",
    );
}

/// `Action::CompleteMission` against a mission already in MISSION_FAILED
/// must NOT fire `OnMissionCompleted` chains. The executor's gate looks
/// at the prior status — only a real active→completed transition fires
/// the dispatcher. Without this guard, "failing" a mission then later
/// "completing" it (e.g., from a manual seed action or chain authoring
/// mistake) would re-fire downstream chains like 1105's auto-accept of
/// the next mission, producing a spurious mission grant the player
/// didn't earn.
///
/// Detection shape: register a `OnMissionCompleted` chain whose only
/// action is `IncrementCounter`. The counter mutation is observable on
/// the entity without needing to wire up the full base-side mission
/// dispatch. The gate's positive (ACTIVE→COMPLETED fires) path is
/// already covered by `fire_mission_completed_runs_matched_chain_actions`
/// in event_dispatch::mission::tests.
#[tokio::test]
async fn complete_mission_action_against_failed_mission_does_not_fire_completion_event() {
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;
    use cimmeria_entity::missions::{MissionInstance, MISSION_FAILED};

    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(42);
        // Seed mission 687 in MISSION_FAILED state. Pre-fix the gate
        // read prior_status.is_some() and would treat this as
        // "transitioned" since it's not None.
        let mut m = MissionInstance::new(687, 2113, vec![]);
        m.fail();
        assert_eq!(m.status, MISSION_FAILED);
        e.missions.add_mission(m);
    }
    mgr.connect_entity(1);

    // Chain: mission_completed 687 → increment_counter (detectable
    // mutation on the entity we control). Same trigger shape as
    // production chain 1105's 687→688 auto-accept.
    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 1105,
        name: "test_complete_chain".to_string(),
        enabled: true,
        trigger: Trigger::OnMissionCompleted { mission_id: 687 },
        conditions: Vec::new(),
        actions: vec![Action::IncrementCounter {
            counter_name: "completion_fired".to_string(),
            amount: 1,
        }],
        priority: 0,
    });

    let (tx, _rx) = mpsc::channel(64);
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(9000, Action::CompleteMission { mission_id: 687 })],
    };
    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    // The counter must NOT have been touched — fire_mission_completed
    // should have been skipped because the prior status was MISSION_FAILED,
    // not MISSION_ACTIVE.
    let entity = mgr.get_entity(1).expect("entity must still exist");
    assert!(
        !entity.counters.contains_key("completion_fired"),
        "fire_mission_completed must NOT fire on FAILED→COMPLETED \
         transition; counter was incremented, meaning chain 1105 \
         spuriously ran",
    );
}

/// CrossWorldTeleport against an unknown local entity_id still
/// dispatches `GateTravel` — base may hold a connection record for the
/// player at this address that the cell hasn't synced yet, and the
/// ring's `Effect::TeleportCrossWorld` arm uses the same shape. The
/// load-bearing invariant is "no panic": the action runs inside
/// `execute_actions` iterating a resolved chain action list, and a
/// malformed chain or a desync between the resolved entity_id and
/// the cell's entity table must not crash the cell loop.
#[tokio::test]
async fn cross_world_teleport_action_with_unknown_entity_dispatches_gate_travel() {
    use crate::cell::messages::CellToBaseMsg;

    let mut mgr = make_space_mgr();
    // Note: no create_entity — eid 999 doesn't exist.

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            1109,
            Action::CrossWorldTeleport {
                world_name: "Castle".to_string(),
                position: [466.365, 70.397, 991.466],
            },
        )],
    };
    // Per-action handlers fail-soft on missing entities; execute_actions
    // returning Ok here is the regression guard.
    execute_actions(resolved, 999, 42, &tx, &mut mgr, &engine).await;

    // The current implementation still emits GateTravel even when the
    // entity is missing locally, because the base side may have an
    // entity record for the player at this addr that the cell hasn't
    // synced yet. That's the same shape the ring's
    // `Effect::TeleportCrossWorld` arm uses. Confirm the message is
    // sent with the requested target.
    let mut got_gate_travel = false;
    while let Ok(msg) = rx.try_recv() {
        if matches!(msg, CellToBaseMsg::GateTravel { .. }) {
            got_gate_travel = true;
        }
    }
    assert!(
        got_gate_travel,
        "CrossWorldTeleport must dispatch GateTravel even when local \
         cell entity is absent — base may still hold the connection"
    );
}

/// `Action::SetAggression { level: 1 }` writes the `aggression` field on
/// the tagged NPC's `CellEntity` so the AI idle tick can wake it up. Bug
/// shape: the previous implementation stored an unread `"aggression"`
/// property-bag entry; this guard pins that the canonical field is now
/// the source of truth.
#[tokio::test]
async fn set_aggression_level_one_writes_entity_field() {
    let mut mgr = make_space_mgr();
    // Tagged NPC the chain will target.
    mgr.spawn_npc(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        npc.tag = Some("Drone".to_string());
        assert_eq!(npc.aggression, 0, "fresh NPC must start passive");
    }
    // Player triggering the chain — must be co-located so
    // `find_entity_by_tag` resolves "Drone" against the same space.
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            1032,
            Action::SetAggression {
                entity_tag: "Drone".to_string(),
                level: 1,
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    assert_eq!(
        mgr.get_entity(101).unwrap().aggression,
        1,
        "SetAggression level=1 must write aggression=1 on the target — \
         the AI idle tick reads this field directly, no property-bag lookup",
    );
}

/// Chain-driven `Action::GenerateThreat` must broadcast `RefreshAppearance`
/// BEFORE `onStateFieldUpdate` when the player just entered combat.
/// `combat::generate_threat` flips `weapon_holstered = false` via
/// `enter_player_combat`; without the appearance refresh the client's
/// cached `ComponentList` keeps rendering the holstered/empty-hand mesh
/// while the server thinks the weapon is drawn — every subsequent fire
/// animation plays against empty hands and the in-combat pose shows no
/// weapon.
///
/// Bug shape from the play-session report on chain 1032 (Ambernol vial
/// pickup triggers drone aggro): "the players fists go into the combat
/// position even though that's not the active bandolier slot, the
/// player then shoots without having a weapon in their hands, and the
/// fists holsters when aggro drops when the drone dies." Three callers
/// of `combat::generate_threat` — `npc_ai_idle_auto_aggro` (NPC sees
/// player), `damage_apply::apply_damage_to_target` (player hits NPC),
/// and this content-action path — must all refresh appearance on
/// first-add. Pre-fix, only the first two did.
///
/// Order matters: appearance BEFORE state-field per the documented
/// "splinch" guard (the client's draw animation socket-attaches the
/// weapon mesh from the current `BeingAppearance`; flipping
/// `BSF_InCombat` first starts the draw animation against an empty
/// socket and the mesh snaps in mid-frame).
#[tokio::test]
async fn generate_threat_action_refreshes_appearance_before_state_field_on_first_aggro() {
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        npc.tag = Some("Drone".to_string());
    }
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
        // Pre-stage: player is holstered + OOC. The bug shape is
        // specifically the first-add `enter_player_combat` transition
        // flipping `weapon_holstered=false` without broadcasting
        // appearance — fixture must start at the holstered state for
        // the flip to be observable.
        p.weapon_holstered = true;
        assert!(
            p.threatened_mobs.is_empty(),
            "fixture sanity: player starts with no aggroed mobs so \
             `enter_player_combat` takes the first-add transition path \
             (returns Some(state)) — without that, the appearance + \
             state-field broadcasts are both skipped and this test \
             passes for the wrong reason"
        );
    }
    mgr.connect_entity(1);

    let (tx, mut rx) = mpsc::channel(32);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            1032,
            Action::GenerateThreat {
                entity_tag: Some("Drone".to_string()),
                threat_level: 1000,
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    // Drain the cell→base channel and capture the order of the two
    // load-bearing messages.
    let mut refresh_at: Option<usize> = None;
    let mut state_field_at: Option<usize> = None;
    let mut idx: usize = 0;
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::RefreshAppearance {
                entity_id: 1,
                player_id: 42,
                holstered,
            } => {
                assert!(
                    !holstered,
                    "RefreshAppearance must carry holstered=false — \
                     enter_player_combat flipped the field; the wire \
                     must mirror the server-side value"
                );
                refresh_at.get_or_insert(idx);
            }
            CellToBaseMsg::EntityMethodCall {
                entity_id: 1,
                method_index,
                ..
            } if method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE => {
                state_field_at.get_or_insert(idx);
            }
            _ => {}
        }
        idx += 1;
    }

    let refresh_idx = refresh_at.expect(
        "generate_threat content action MUST broadcast RefreshAppearance \
         on the first-add `enter_player_combat` transition — pre-fix the \
         action handler only sent onStateFieldUpdate and the client kept \
         the holstered/empty-hand BeingAppearance cached, producing the \
         play-session 'fists in combat pose, shoots without a weapon' bug \
         specifically from chain 1032 (Ambernol pickup triggers drone aggro)",
    );
    let state_field_idx = state_field_at.expect(
        "generate_threat must STILL broadcast onStateFieldUpdate so the \
         in-combat HUD / targeting cursor / state-bit-derived UI flips. \
         A regression that drops this packet leaves the player visually \
         drawn but UI-OOC",
    );
    assert!(
        refresh_idx < state_field_idx,
        "ordering: RefreshAppearance (#{refresh_idx}) must precede \
         onStateFieldUpdate (#{state_field_idx}). Both flow through the \
         same client-side state-machine entry point, but only the \
         appearance path triggers the socket re-attach that writes the \
         weapon-category byte. If `BSF_InCombat` flips first, the \
         unholster animation starts before the mesh is attached — the \
         documented 'splinch' bug shape from PR #395"
    );

    // Sanity: the cell-side state did flip. If this fails, the
    // assertions above passed for the wrong reason.
    let player = mgr.get_entity(1).unwrap();
    assert!(
        !player.weapon_holstered,
        "enter_player_combat must flip weapon_holstered=false on the \
         first-add transition — broadcast or not, the server-side state \
         is the source of truth for subsequent fire-path gating"
    );
    assert!(
        player.threatened_mobs.contains(&101),
        "drone must be in the player's threatened_mobs set after \
         generate_threat — this is what `enter_player_combat`'s \
         was_empty → Some(state) gate keys on"
    );
}

// ──────────────────────────────────────────────────────────────────────
// Negative-logging regression guards — content executor.
//
// Each test drops the mpsc receiver before invoking the executor, so
// the cell→base channel returns SendError on every `.send()`. The
// guards assert the new WARN fires with the chain_id field — reverting
// the `if let Err` change back to `let _` would silence the log and
// trip the test.
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn play_sequence_warns_when_cell_to_base_channel_closed() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    mgr.create_entity(7, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(7) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, rx) = mpsc::channel(8);
    drop(rx); // close the cell→base channel
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(9001, Action::PlaySequence { sequence_id: 512 })],
    };

    execute_actions(resolved, 7, 42, &tx, &mut mgr, &engine).await;

    assert!(
        capture
            .find_message(Level::WARN, "PlaySequence: cell→base send failed")
            .is_some(),
        "negative-logging convention: PlaySequence must WARN when cell→base channel is closed; \
         reverting to `let _ = tx.send` breaks chain-stall diagnosability. \
         Captured events: {:#?}",
        capture.all()
    );
}

#[tokio::test]
async fn start_minigame_warns_when_cell_to_base_channel_closed() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    mgr.create_entity(8, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(8) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, rx) = mpsc::channel(8);
    drop(rx);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            9002,
            Action::StartMinigame {
                minigame_type: "livewire".to_string(),
                on_victory_chains: vec![],
            },
        )],
    };

    execute_actions(resolved, 8, 42, &tx, &mut mgr, &engine).await;

    assert!(
        capture
            .find_message(Level::WARN, "StartMinigame: cell→base send failed")
            .is_some(),
        "negative-logging convention: StartMinigame must WARN when cell→base channel is closed"
    );
}

#[tokio::test]
async fn set_active_slot_warns_when_cell_to_base_channel_closed() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    mgr.create_entity(9, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(9) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, rx) = mpsc::channel(8);
    drop(rx);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(9003, Action::SetActiveSlot { bag_id: 3, slot: 0 })],
    };

    execute_actions(resolved, 9, 42, &tx, &mut mgr, &engine).await;

    assert!(
        capture
            .find_message(Level::WARN, "SetActiveSlot: cell→base send failed")
            .is_some(),
        "negative-logging convention: SetActiveSlot must WARN when cell→base channel is closed"
    );
}

/// Stage a player + an NPC with matching template_id, with the NPC
/// already in the player's witness set, with one `available_interactions`
/// entry on the player keyed on the template slot. Returns the slot id
/// and the dialog_set_id that's been stashed there.
///
/// Setting both `witnesses` and `available_interactions` is what makes
/// `RemoveDialogSet` exercise the `send_interaction_update_if_visible`
/// branch where the WitnessEntityMethod is actually dispatched.
fn stage_dialog_set_witness(
    mgr: &mut SpaceManager,
    player_id: u32,
    npc_id: u32,
    template_id: i32,
    dialog_set_id: i32,
) {
    use cimmeria_common::EntityId;

    mgr.create_entity(player_id, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.create_entity(npc_id, "Agnos", [1.0; 3], [0.0; 3])
        .unwrap();

    if let Some(p) = mgr.get_entity_mut(player_id) {
        p.is_player = true;
        p.player_id = Some(42);
        p.witnesses.insert(EntityId(npc_id as i32));
        p.available_interactions
            .entry(template_id)
            .or_default()
            .push((dialog_set_id, /* dialog_id */ 7, /* flags */ 0x10));
    }
    if let Some(n) = mgr.get_entity_mut(npc_id) {
        n.template_id = Some(template_id);
        n.interaction_type_flags = 0x01;
    }
}

#[tokio::test]
async fn remove_dialog_set_warns_when_cell_to_base_channel_closed() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    stage_dialog_set_witness(
        &mut mgr, /* player */ 11, /* npc */ 111, /* template_id */ 555,
        /* dialog_set_id */ 88,
    );

    let (tx, rx) = mpsc::channel(8);
    drop(rx);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            9004,
            Action::RemoveDialogSet {
                dialog_set_id: 88,
                slot: 555,
            },
        )],
    };

    execute_actions(resolved, 11, 42, &tx, &mut mgr, &engine).await;

    assert!(
        capture
            .find_message(
                Level::WARN,
                "RemoveDialogSet: cell→base interaction-type send failed"
            )
            .is_some(),
        "negative-logging convention: RemoveDialogSet must WARN when its InteractionType push fails; \
         reverting to `let _ = tx.send` breaks stale-prompt diagnosability. \
         Captured: {:#?}",
        capture.all()
    );
}

#[tokio::test]
async fn add_dialog_set_warns_when_cell_to_base_channel_closed() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();

    // Pre-populate `dialog_set_maps` so `add_dialog_set` resolves the
    // entry and reaches the WitnessEntityMethod send. The cache key is
    // the dialog_set_id; value is a (dialog_id, interaction_flags).
    {
        use crate::cell::spawner::DialogSetMapEntry;
        mgr.dialog_set_maps.insert(
            88,
            DialogSetMapEntry {
                dialog_id: 7,
                interaction_flags: 0x10,
            },
        );
    }

    stage_dialog_set_witness(
        &mut mgr, /* player */ 12, /* npc */ 112, /* template_id */ 555,
        /* dialog_set_id */ 88,
    );

    let (tx, rx) = mpsc::channel(8);
    drop(rx);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            9005,
            Action::AddDialogSet {
                dialog_set_id: 88,
                slot: 555,
                mission_id: None,
            },
        )],
    };

    execute_actions(resolved, 12, 42, &tx, &mut mgr, &engine).await;

    assert!(
        capture
            .find_message(Level::WARN, "interaction-type send failed")
            .is_some(),
        "negative-logging convention: AddDialogSet's send_interaction_update_if_visible helper \
         must WARN when cell→base is closed (covers the :247 path). Captured: {:#?}",
        capture.all()
    );
}

// ──────────────────────────────────────────────────────────────────────
// Phase 4 / 6 / 7 content actions: SetNpcPoi, SetFollowTarget, SetNpcAiState
// ──────────────────────────────────────────────────────────────────────

/// `Action::SetNpcPoi` transitions the tagged NPC into Investigating
/// with `poi` set to the given coords and `nav_path` cleared so the
/// investigate handler can re-pathfind.
#[tokio::test]
async fn set_npc_poi_transitions_target_to_investigating() {
    use cimmeria_entity::cell_entity::AiState;
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        npc.tag = Some("Guard".to_string());
        npc.nav_path
            .push_back(cimmeria_common::Vector3::new(99.0, 0.0, 99.0));
    }
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            1234,
            Action::SetNpcPoi {
                entity_tag: "Guard".to_string(),
                x: 50.0,
                y: 0.0,
                z: 60.0,
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let npc = mgr.get_entity(101).unwrap();
    assert_eq!(npc.ai_state, AiState::Investigating);
    assert_eq!(
        npc.poi,
        Some(cimmeria_common::Vector3::new(50.0, 0.0, 60.0)),
    );
    assert!(npc.nav_path.is_empty(), "Action must clear stale nav_path");
}

/// `Action::SetFollowTarget` with a resolvable `target_tag` transitions
/// the follower into Follow with `follow_target_id` set.
#[tokio::test]
async fn set_follow_target_resolves_target_and_transitions_to_follow() {
    use cimmeria_entity::cell_entity::AiState;
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        npc.tag = Some("Pet".to_string());
    }
    mgr.spawn_npc(102, "Agnos", [20.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(102) {
        npc.tag = Some("Owner".to_string());
    }
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            5678,
            Action::SetFollowTarget {
                entity_tag: "Pet".to_string(),
                target_tag: Some("Owner".to_string()),
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let pet = mgr.get_entity(101).unwrap();
    assert_eq!(pet.ai_state, AiState::Follow);
    assert_eq!(pet.follow_target_id, Some(102));
}

/// `Action::SetFollowTarget` with `target_tag = None` clears the follow
/// state and returns the NPC to Idle.
#[tokio::test]
async fn set_follow_target_none_clears_and_returns_to_idle() {
    use cimmeria_entity::cell_entity::AiState;
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        npc.tag = Some("Pet".to_string());
        npc.ai_state = AiState::Follow;
        npc.follow_target_id = Some(999);
    }
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            5678,
            Action::SetFollowTarget {
                entity_tag: "Pet".to_string(),
                target_tag: None,
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let pet = mgr.get_entity(101).unwrap();
    assert_eq!(pet.ai_state, AiState::Idle);
    assert_eq!(pet.follow_target_id, None);
}

/// `Action::SetFollowTarget` with an unresolvable `target_tag` is
/// treated the same as `None` — the follow state clears and the NPC
/// returns to Idle. Pin: a typo or removed tag shouldn't leave the
/// follower in a half-state.
#[tokio::test]
async fn set_follow_target_unresolvable_tag_clears_follow() {
    use cimmeria_entity::cell_entity::AiState;
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        npc.tag = Some("Pet".to_string());
        npc.ai_state = AiState::Follow;
        npc.follow_target_id = Some(999);
    }
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            5678,
            Action::SetFollowTarget {
                entity_tag: "Pet".to_string(),
                target_tag: Some("Nonexistent".to_string()),
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let pet = mgr.get_entity(101).unwrap();
    assert_eq!(
        pet.ai_state,
        AiState::Idle,
        "Unresolvable target_tag must drop to Idle (treated as clear)",
    );
    assert_eq!(pet.follow_target_id, None);
}

/// `Action::SetNpcAiState { state: Despawning }` flips the state.
#[tokio::test]
async fn set_npc_ai_state_despawning_flips_state() {
    use cimmeria_content_engine::actions::NpcAiStateAction;
    use cimmeria_entity::cell_entity::AiState;
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        npc.tag = Some("Boss".to_string());
    }
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            999,
            Action::SetNpcAiState {
                entity_tag: "Boss".to_string(),
                state: NpcAiStateAction::Despawning,
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let npc = mgr.get_entity(101).unwrap();
    assert_eq!(npc.ai_state, AiState::Despawning);
}

/// `Action::SetNpcAiState { state: Idle }` on a patrolling NPC drops
/// it back to Idle. Pin: `patrol_next_index` persists so the AI tick's
/// Idle→Patrol promotion can resume the route from where it left off.
#[tokio::test]
async fn set_npc_ai_state_idle_on_patroller_preserves_patrol_index() {
    use cimmeria_common::Vector3;
    use cimmeria_content_engine::actions::NpcAiStateAction;
    use cimmeria_entity::cell_entity::AiState;
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        npc.tag = Some("Guard".to_string());
        npc.patrol_path = vec![
            Vector3::new(10.0, 0.0, 0.0),
            Vector3::new(10.0, 0.0, 10.0),
            Vector3::new(0.0, 0.0, 10.0),
        ];
        npc.ai_state = AiState::Patrol;
        npc.patrol_next_index = 2;
    }
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            999,
            Action::SetNpcAiState {
                entity_tag: "Guard".to_string(),
                state: NpcAiStateAction::Idle,
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let npc = mgr.get_entity(101).unwrap();
    assert_eq!(npc.ai_state, AiState::Idle);
    assert_eq!(
        npc.patrol_next_index, 2,
        "patrol_next_index must persist across SetNpcAiState(Idle) so the AI tick can resume the route",
    );
}
