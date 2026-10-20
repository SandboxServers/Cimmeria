//! defense_q1 siblings and the Q1 hand-off (CS-04b1c2): no event resolves two
//! story chains, the sibling chains never act on 10310, and completing Q1 in
//! the courtyard offers the sentry.

use std::collections::BTreeSet;

use cimmeria_common::EntityId;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::super::mission_context::{populate_mission_context, populate_world_context};
use super::*;

/// Q1's chains 10000-10006, loaded beside the story for the Q1 completion.
const Q1_CHAINS: [i32; 7] = [10000, 10001, 10002, 10003, 10004, 10005, 10006];
/// Sibling chains on the same event keys as the story (`castle_side_stories`
/// and the bark seeds): they must never act on 10310.
const SIBLING_CHAINS: [i32; 8] = [1367, 1368, 1369, 1381, 1382, 1383, 10200, 10201];

/// The event keys the story's chains and their siblings listen on, as the
/// dispatchers build them: the trigger type and the one param that carries
/// the key.
fn event_keys() -> Vec<(TriggerType, Option<(&'static str, serde_json::Value)>)> {
    vec![
        (
            TriggerType::MissionCompleted,
            Some(("mission_id", PREREQ.into())),
        ),
        (
            TriggerType::MissionCompleted,
            Some(("mission_id", 704.into())),
        ),
        (TriggerType::PlayerLoaded, None),
        (
            TriggerType::MissionAbandoned,
            Some(("mission_id", PREREQ.into())),
        ),
        (
            TriggerType::MissionAccepted,
            Some(("mission_id", PREREQ.into())),
        ),
        (
            TriggerType::InteractTag,
            Some(("entity_tag", "Castle_Pop_Infirmary_Medic1".into())),
        ),
        (
            TriggerType::MissionAbandoned,
            Some(("mission_id", MISSION.into())),
        ),
        (
            TriggerType::InteractTag,
            Some(("entity_tag", SENTRY_TAG.into())),
        ),
        (
            TriggerType::MissionAccepted,
            Some(("mission_id", MISSION.into())),
        ),
        (
            TriggerType::RegionEnter,
            Some(("region_key", INFIRMARY.into())),
        ),
        (
            TriggerType::RegionEnter,
            Some(("region_key", COURTYARD.into())),
        ),
    ]
}

/// Resolve one event against `mgr` as the dispatchers build its context;
/// returns each resolved chain id with its action, as debug text.
fn resolve(
    engine: &ChainEngine,
    mgr: &SpaceManager,
    trigger_type: TriggerType,
    key: Option<(&'static str, serde_json::Value)>,
) -> Vec<(i64, String)> {
    let mut ctx = ExecutionContext::new().with_source(EntityId(ENTITY as i32));
    if let Some((name, value)) = key {
        ctx.set_param(name.to_string(), value);
    }
    populate_world_context(ENTITY, mgr, &mut ctx);
    let entity = mgr.get_entity(ENTITY).unwrap();
    populate_mission_context(entity, &mut ctx);
    ctx.set_param("archetype".to_string(), serde_json::json!(1));
    let event = TriggerEvent {
        trigger_type,
        source_entity: Some(EntityId(ENTITY as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine
        .resolve_event(&event, &ctx)
        .actions
        .iter()
        .map(|(id, action)| (*id, format!("{action:?}")))
        .collect()
}

fn is_story_chain(id: i64) -> bool {
    (10100..=10106).contains(&id)
}

/// **For each event key and every Q1 and 10310 state in the Castle, at most
/// one story chain resolves, and the Q1 chains resolve somewhere.** The
/// siblings (`SIBLING_CHAINS`) fire exactly as they do with 10310 absent, and
/// their actions never name 10310, so the story does not change what they do.
/// Revert proof: change 10101's `mission_status 10310` from `not_active` to
/// `active` and 10101 and 10106 both resolve on `player_loaded` at step A.
#[tokio::test]
async fn live_db_castle_side_defense_q1_siblings_disjoint() {
    let pool = require_db_or_skip!();
    let ids: Vec<i32> = Q1_CHAINS
        .iter()
        .chain(&ALL_CHAINS)
        .chain(&SIBLING_CHAINS)
        .copied()
        .collect();
    let engine = engine_with(&pool, &ids).await;
    let states = [Stage::Absent, Stage::AtA, Stage::AtB, Stage::Done];
    let mut seen: BTreeSet<i64> = BTreeSet::new();
    for q1 in states {
        for sentry in states {
            let mgr = staged(&pool, CASTLE, stages(q1, sentry)).await;
            let bare = staged(&pool, CASTLE, stages(q1, Stage::Absent)).await;
            for (trigger_type, key) in event_keys() {
                let fired = resolve(&engine, &mgr, trigger_type.clone(), key.clone());
                // One row per action, so a chain with three actions repeats.
                let story: BTreeSet<i64> = fired
                    .iter()
                    .map(|(id, _)| *id)
                    .filter(|id| is_story_chain(*id))
                    .collect();
                assert!(
                    story.len() <= 1,
                    "{trigger_type:?} with Q1 {q1:?}, 10310 {sentry:?} resolved story chains {story:?}"
                );
                seen.extend(fired.iter().map(|(id, _)| *id));

                let siblings_now: Vec<(i64, String)> = fired
                    .into_iter()
                    .filter(|(id, _)| !is_story_chain(*id))
                    .collect();
                let siblings_bare: Vec<(i64, String)> = resolve(&engine, &bare, trigger_type, key)
                    .into_iter()
                    .filter(|(id, _)| !is_story_chain(*id))
                    .collect();
                assert_eq!(
                    siblings_now.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
                    siblings_bare.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
                    "Q1 {q1:?}, 10310 {sentry:?}: the sibling chains must not depend on 10310"
                );
                for (id, action) in &siblings_now {
                    assert!(
                        !action.contains("10310"),
                        "sibling chain {id} must not act on 10310; got {action}"
                    );
                }
            }
        }
    }
    for id in Q1_CHAINS.iter().chain(&ALL_CHAINS) {
        assert!(
            seen.contains(&i64::from(*id)),
            "chain {id} must resolve in some Q1 and 10310 state"
        );
    }
}

/// **Completing Q1 in the courtyard offers the sentry (10100), and a player
/// who finished Q1 earlier is bound at login (10101).** Entering the courtyard
/// at Q1's B runs 10005, which completes 10300 and fires `mission_completed
/// 10300`; 10100 binds the sentry on slot 419 in the same run. Revert proof:
/// drop 10100 and the courtyard completion leaves slot 419 unbound.
#[tokio::test]
async fn live_db_castle_side_defense_q1_q1_completion_offers_sentry() {
    let pool = require_db_or_skip!();
    let ids: Vec<i32> = Q1_CHAINS.iter().chain(&ALL_CHAINS).copied().collect();
    let engine = engine_with(&pool, &ids).await;

    let mut mgr = staged(&pool, CASTLE, stages(Stage::AtB, Stage::Absent)).await;
    assert!(
        bound_at(&mgr, OFFER_SLOT).is_empty(),
        "the sentry must not be offered before Q1 completes"
    );
    enter(&engine, &mut mgr, COURTYARD).await;
    assert_eq!(
        status_of(&mgr, PREREQ),
        Some(MISSION_COMPLETED),
        "the courtyard at Q1's B must complete 10300"
    );
    assert_eq!(
        bound_at(&mgr, OFFER_SLOT),
        vec![OFFER_SET],
        "Q1's completion must bind the sentry (10100) on slot 419"
    );

    let mut earlier = staged(&pool, CASTLE, stages(Stage::Done, Stage::Absent)).await;
    log_in(&engine, &mut earlier).await;
    assert_eq!(
        bound_at(&earlier, OFFER_SLOT),
        vec![OFFER_SET],
        "a player who finished Q1 earlier must be bound at login (10101)"
    );
}
