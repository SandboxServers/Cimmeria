//! return_q1 (mission 10320) against its siblings: no event that one of
//! chains 10130-10133 handles is also handled by another chain on the same
//! key, and the seed carries the world gates the seal freezes (G9). The
//! return_q2 chains are not seeded in this worktree; the 10141/10145 pairs of
//! the seal's list belong to CS-04c2.

use cimmeria_content_engine::triggers::TriggerType;
use cimmeria_entity::missions::MissionInstance;

use super::harness::*;
use crate::test_support::require_db_or_skip;

/// **No event matches one of 10130-10133 and any other chain on the same
/// key.** Every enabled chain that listens on one of the four keys is loaded
/// from the seed (so Marsh's 708 chain 1352 and every `player_loaded Castle`
/// relog chain are in), and each event is resolved over every reachable
/// state: 708 absent, at step 2417 or completed with 10320 not started, and
/// 708 completed with 10320 at A or completed; archetype 1 and 8; origin
/// Castle and Harset. When one of mine resolves it resolves alone. Each of
/// my chains must also resolve somewhere, so the matrix is not vacuous.
/// Revert proof: delete `mission_status 10320 active` and `step_status` from
/// 10132; with 708 at 2417 the Marsh click then resolves 10132 and 1352.
#[tokio::test]
async fn live_db_castle_side_return_c2_siblings_disjoint() {
    let pool = require_db_or_skip!();
    let ids: Vec<i32> = sqlx::query_scalar(
        "SELECT DISTINCT t.chain_id FROM resources.content_triggers t \
         JOIN resources.content_chains c ON c.chain_id = t.chain_id \
         WHERE c.enabled AND ((t.event_type = 'stargate_crossed' AND t.event_key = 'Castle') \
            OR (t.event_type = 'player_loaded' AND t.event_key = 'Castle') \
            OR (t.event_type = 'interact_tag' AND t.event_key = 'Castle_ColMarsh') \
            OR (t.event_type = 'mission_abandoned' AND t.event_key = '10320')) \
         ORDER BY 1",
    )
    .fetch_all(&pool)
    .await
    .expect("sibling chain query must succeed");
    for mine in RETURN_Q1_CHAINS {
        assert!(
            ids.contains(&mine),
            "chain {mine} must listen on a shared key"
        );
    }
    assert!(
        ids.contains(&1352),
        "708's Marsh chain 1352 must be a sibling"
    );
    // `player_loaded Castle` fans out to every Castle chain (701 binds slot 149
    // on the same login), so only the chains that touch Marsh's slot 10 are
    // siblings of 10131 there.
    let slot_10: Vec<i32> = sqlx::query_scalar(
        "SELECT DISTINCT chain_id FROM resources.content_actions \
         WHERE action_type IN ('add_dialog_set', 'remove_dialog_set') \
           AND params->>'slot' = '10' ORDER BY 1",
    )
    .fetch_all(&pool)
    .await
    .expect("slot 10 chain query must succeed");
    let mut all_ids = ids.clone();
    all_ids.extend(slot_10.iter().copied().filter(|id| !ids.contains(id)));
    let engine = engine_with(&pool, &all_ids).await;

    let states: [(&str, Vec<MissionInstance>); 5] = [
        ("708 absent", Vec::new()),
        ("708 at 2417", vec![open_708()]),
        ("708 done, 10320 absent", vec![completed_708()]),
        (
            "708 done, 10320 at A",
            vec![completed_708(), return_mission()],
        ),
        (
            "708 done, 10320 done",
            vec![completed_708(), finished_return()],
        ),
    ];
    let mut seen = std::collections::BTreeSet::new();
    for (label, missions) in &states {
        for archetype in [1, 8] {
            for world in [CASTLE, HARSET] {
                let mut mgr = stage(&pool, world, missions.clone()).await;
                mgr.get_entity_mut(ENTITY).unwrap().archetype_id = Some(archetype);
                let events: [(TriggerType, Option<(&str, serde_json::Value)>); 4] = [
                    (
                        TriggerType::StargateCrossed,
                        Some(("destination_world", CASTLE.into())),
                    ),
                    (TriggerType::PlayerLoaded, None),
                    (
                        TriggerType::InteractTag,
                        Some(("entity_tag", MARSH_TAG.into())),
                    ),
                    (
                        TriggerType::MissionAbandoned,
                        Some(("mission_id", MISSION.into())),
                    ),
                ];
                for (trigger_type, key) in events {
                    let event = format!("{trigger_type:?}");
                    let on_login = trigger_type == TriggerType::PlayerLoaded;
                    let mut fired = resolve(&engine, &mgr, trigger_type, key);
                    if on_login {
                        fired.retain(|id| {
                            slot_10.contains(&(*id as i32))
                                || RETURN_Q1_CHAINS.contains(&(*id as i32))
                        });
                    }
                    let mine: Vec<i32> = RETURN_Q1_CHAINS
                        .iter()
                        .copied()
                        .filter(|id| fired.contains(&(*id as i64)))
                        .collect();
                    seen.extend(mine.iter().copied());
                    if !mine.is_empty() {
                        assert_eq!(
                            fired.len(),
                            1,
                            "{event} in {world}, archetype {archetype}, {label}: \
                             my chain {mine:?} resolved with {fired:?}; siblings must be disjoint"
                        );
                    }
                }
            }
        }
    }
    for mine in RETURN_Q1_CHAINS {
        assert!(
            seen.contains(&mine),
            "chain {mine} never resolved in the matrix; the probe is broken (saw {seen:?})"
        );
    }
}

/// **Every chain of 10130-10133 is world-gated, and only the crossing accept
/// uses world 57.** 10130 carries `world 57` and exactly one action, an
/// `accept_mission` (no bind, no dialog); 10131-10133 carry `world 8` and no
/// chain carries two world conditions. Revert proof: delete the `world 8` row
/// of 10133 (or change it to 57); the count or value assertion fails.
#[tokio::test]
async fn live_db_castle_side_return_c2_world_gates_frozen() {
    let pool = require_db_or_skip!();
    let ids: Vec<i32> = RETURN_Q1_CHAINS.to_vec();
    for id in &ids {
        let worlds: Vec<Option<i32>> = sqlx::query_scalar(
            "SELECT target_id FROM resources.content_conditions \
             WHERE chain_id = $1 AND condition_type = 'world' ORDER BY sort_order",
        )
        .bind(id)
        .fetch_all(&pool)
        .await
        .expect("world condition query must succeed");
        let want = if *id == 10130 { 57 } else { 8 };
        assert_eq!(
            worlds,
            vec![Some(want)],
            "chain {id} must carry exactly one world condition, world {want}"
        );
    }
    let actions: Vec<String> = sqlx::query_scalar(
        "SELECT action_type FROM resources.content_actions WHERE chain_id = 10130",
    )
    .fetch_all(&pool)
    .await
    .expect("action query must succeed");
    assert_eq!(
        actions,
        vec!["accept_mission".to_string()],
        "the crossing chain 10130 only accepts"
    );
}
