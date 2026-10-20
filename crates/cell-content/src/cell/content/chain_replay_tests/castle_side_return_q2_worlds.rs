//! Castle return_q2 side story (mission 10321), CS-04c2c2: what the seven
//! chains 10140-10146 must NOT do across chains and across worlds.
//!
//! * no event resolves two of the story's chains, or one of them and a 708
//!   sibling on the same key, in any reachable state
//!   (`live_db_castle_side_return_q2_siblings_disjoint`);
//! * G9: the Moh'katan and Marsh templates are also spawned in the Cellblock,
//!   the Harset Command Center and Dakara, so no bind may happen there
//!   (`live_db_castle_side_return_q2_no_bind_outside_world_8`) and every chain of the
//!   story carries a `world` condition (`live_db_castle_side_return_q2_every_chain_names_a_world`);
//! * a Harset to Castle crossing accepts nothing of 10321, and from a world
//!   other than 57 accepts nothing at all
//!   (`live_db_castle_side_return_q2_crossing_from_the_wrong_world_fires_nothing`).
//!
//! The harness (staged player, state enums) is in
//! `castle_side_return_q2_lifecycle.rs`. Chains 10130-10133 (return_q1) are
//! seeded by the return_q1 packet; where this tree does not carry them the
//! tests run on the chains that exist and note it.

use std::collections::BTreeSet;

use cimmeria_common::EntityId;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use cimmeria_entity::missions::{MissionInstance, MISSION_COMPLETED};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::event_dispatch::fire_mission_completed;
use super::super::fire_stargate_crossed;
use super::super::mission_context::{populate_mission_context, populate_world_context};
use super::castle_side_return_q2_lifecycle::{
    bound_at, drain, engine_with, log_in, mission_of, missions, staged, Own, Prereq, ALL_CHAINS,
    CASTLE, ENTITY, MISSION, MOHKATAN_TAG, OFFER_SET, OFFER_SLOT, PLAYER, PREREQ, WORLDS,
};
use crate::cell::content::fire_mission_abandoned;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::require_db_or_skip;

const HARSET: &str = "Harset";
/// Mission 708 (Report to the Castle): the prerequisite of the whole arc.
const MISSION_708: i32 = 708;
/// The 708 sibling chains on the Moh'katan tag (1354) and the Harset dial
/// (1358 Tau'ri, 1359 Jaffa) that the return_q2 chains share keys with.
const SIBLINGS: [i32; 3] = [1354, 1358, 1359];
/// The Marsh side of return_q1: bind slot 10, chains 10131 and 10133.
const MARSH_SLOT: i32 = 10;
const Q1_CHAINS: [i32; 2] = [10131, 10133];
/// The Harset to Castle crossing accept (return_q1).
const CROSSING_CHAIN: i32 = 10130;

/// 708 as the player holds it.
#[derive(Clone, Copy, Debug)]
enum Q708 {
    Absent,
    /// Active at this step (2417 Moh'katan report, 4462 dial the gate).
    At(i32),
    Completed,
}

fn q708(state: Q708) -> Option<MissionInstance> {
    let mut mission = match state {
        Q708::Absent => return None,
        Q708::At(step) => {
            let mut m = MissionInstance::new(MISSION_708, step, Vec::new());
            m.current_step_id = Some(step);
            return Some(m);
        }
        Q708::Completed => MissionInstance::new(MISSION_708, 0, Vec::new()),
    };
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    Some(mission)
}

/// The chains of `ids` the seed carries, registered in one engine, and which
/// ones those were.
async fn engine_with_present(pool: &PgPool, ids: &[i32]) -> (ChainEngine, Vec<i32>) {
    let mut engine = ChainEngine::new();
    let mut present = Vec::new();
    for &id in ids {
        let chain = load_single_chain_for_test(pool, id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {id} must succeed: {e}"));
        if let Some(chain) = chain {
            engine.register_chain(chain);
            present.push(id);
        }
    }
    (engine, present)
}

// ── Sibling disjointness ─────────────────────────────────────────────────

/// **At most one chain of the return_q2 story or of the 708 siblings resolves
/// for any event in any reachable state.** The events are the login
/// (`player_loaded 'Castle'`), the Moh'katan click and the Harset dial, over
/// 708 absent, at 2417, at 4462 or completed, every reachable (10320, 10321)
/// state (10320 and 10321 exist only after 708 completes, 10321 only after
/// 10320 completes) and archetype 1 or 8. The second half requires each of
/// 10141, 10142, 10144, 10145 and the three siblings to resolve somewhere, so
/// a dropped chain or a wrong event key cannot pass vacuously.
/// Revert proof: delete `mission_status 10321 not_active` from 10141; the
/// (10320 completed, 10321 at A) login then resolves both 10141 and 10145.
#[tokio::test]
async fn live_db_castle_side_return_q2_siblings_disjoint() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[&ALL_CHAINS[..], &SIBLINGS[..]].concat()).await;
    let mut seen: BTreeSet<i64> = BTreeSet::new();
    // (trigger, key param name and value)
    let events: [(TriggerType, Option<(&str, &str)>); 3] = [
        (TriggerType::PlayerLoaded, None),
        (TriggerType::InteractTag, Some(("entity_tag", MOHKATAN_TAG))),
        (
            TriggerType::StargateDialed,
            Some(("destination_world", HARSET)),
        ),
    ];
    let mut states: Vec<(Q708, Prereq, Own)> = vec![
        (Q708::Absent, Prereq::Absent, Own::Absent),
        (Q708::At(2417), Prereq::Absent, Own::Absent),
        (Q708::At(4462), Prereq::Absent, Own::Absent),
    ];
    for prereq in [Prereq::Absent, Prereq::Active, Prereq::Completed] {
        states.push((Q708::Completed, prereq, Own::Absent));
    }
    for own in [Own::AtA, Own::AtB, Own::Done] {
        states.push((Q708::Completed, Prereq::Completed, own));
    }
    for (state708, prereq, own) in states {
        for archetype in [1, 8] {
            let mut held = missions(prereq, own);
            held.extend(q708(state708));
            let mut mgr = staged(&pool, CASTLE, held).await;
            mgr.get_entity_mut(ENTITY).unwrap().archetype_id = Some(archetype);
            for (trigger_type, key) in &events {
                let mut ctx = ExecutionContext::new().with_source(EntityId(ENTITY as i32));
                if let Some((name, value)) = key {
                    ctx.set_param(name.to_string(), serde_json::json!(value));
                }
                populate_world_context(ENTITY, &mgr, &mut ctx);
                populate_mission_context(mgr.get_entity(ENTITY).unwrap(), &mut ctx);
                ctx.set_param("archetype".to_string(), serde_json::json!(archetype));
                let event = TriggerEvent {
                    trigger_type: trigger_type.clone(),
                    source_entity: Some(EntityId(ENTITY as i32)),
                    target_entity: None,
                    params: ctx.params.clone(),
                };
                let fired: BTreeSet<i64> = engine
                    .resolve_event(&event, &ctx)
                    .actions
                    .iter()
                    .map(|(id, _)| *id)
                    .collect();
                assert!(
                    fired.len() <= 1,
                    "{trigger_type:?} {key:?} with 708 {state708:?}, 10320 {prereq:?}, \
                     10321 {own:?}, archetype {archetype} resolved {fired:?}; \
                     the set must be disjoint"
                );
                seen.extend(fired);
            }
        }
    }
    for id in [10141, 10142, 10144, 10145, 1354, 1358, 1359] {
        assert!(
            seen.contains(&id),
            "chain {id} must resolve in some reachable state; resolved {seen:?}"
        );
    }
}

// ── G9: nothing binds outside world 8 ────────────────────────────────────

/// Bound ids at the two slots the story uses, in one value.
fn binds(mgr: &SpaceManager) -> (Vec<i32>, Vec<i32>) {
    (bound_at(mgr, MARSH_SLOT), bound_at(mgr, OFFER_SLOT))
}

/// Replay every trigger of 10131/10133/10140/10141/10146 with the player in
/// `world`, each in the state where every condition but the world holds.
/// Returns the binds and cell-to-base traffic after each replay.
async fn replay_binding_triggers(
    pool: &PgPool,
    engine: &ChainEngine,
    world: &str,
) -> Vec<(&'static str, (Vec<i32>, Vec<i32>), Vec<CellToBaseMsg>)> {
    let mut out = Vec::new();
    // `player_loaded` is keyed on the QUEST world's name: the replay keeps
    // the key 'Castle' and moves only the player, so the `world` condition is
    // the one thing standing between the event and the bind.
    let logins = [
        (
            "login, 10320 active (10131)",
            missions(Prereq::Active, Own::Absent),
        ),
        (
            "login, offer open (10141)",
            missions(Prereq::Completed, Own::Absent),
        ),
    ];
    for (label, held) in logins {
        let mut mgr = staged(pool, world, held).await;
        log_in(engine, &mut mgr, CASTLE).await;
        out.push((label, binds(&mgr), Vec::new()));
    }
    let (tx, mut rx) = mpsc::channel(256);
    let mut mgr = staged(pool, world, missions(Prereq::Completed, Own::Absent)).await;
    fire_mission_completed(ENTITY, PLAYER, PREREQ, engine, &tx, &mut mgr).await;
    out.push(("10320 completed (10140)", binds(&mgr), drain(&mut rx)));

    let mut mgr = staged(pool, world, missions(Prereq::Completed, Own::Absent)).await;
    fire_mission_abandoned(ENTITY, PLAYER, MISSION, engine, &tx, &mut mgr).await;
    out.push(("10321 abandoned (10146)", binds(&mgr), drain(&mut rx)));

    let mut mgr = staged(pool, world, missions(Prereq::Absent, Own::Absent)).await;
    fire_mission_abandoned(ENTITY, PLAYER, PREREQ, engine, &tx, &mut mgr).await;
    out.push(("10320 abandoned (10133)", binds(&mgr), drain(&mut rx)));
    out
}

/// **No chain of the story binds or unbinds a dialog set, or sends mission or
/// dialog traffic, outside world 8.** The Marsh and Moh'katan templates are
/// also spawned in the Cellblock, the Harset Command Center and Dakara, and
/// the dialog-set slots are per template. Each trigger of 10131, 10133,
/// 10140, 10141 and 10146 is replayed with the player in Castle_CellBlock,
/// Harset_CmdCenter, Dakara_E1_StoryRm, Harset and CombatSim, in the state
/// where every condition but the world holds: nothing binds, nothing is sent.
/// The Castle control binds the Moh'katan offer on each of 10140, 10141 and
/// 10146, so the sweep cannot pass because the chains are dead. 10131 and
/// 10133 are seeded by the return_q1 packet; they join the replay when the
/// seed carries them.
/// Revert proof: delete `world 8` from 10141; the Harset_CmdCenter login binds
/// the offer.
#[tokio::test]
async fn live_db_castle_side_return_q2_no_bind_outside_world_8() {
    let pool = require_db_or_skip!();
    let (engine, present) =
        engine_with_present(&pool, &[&ALL_CHAINS[..], &Q1_CHAINS[..]].concat()).await;
    for id in ALL_CHAINS {
        assert!(present.contains(&id), "chain {id} must be seeded");
    }

    let control = replay_binding_triggers(&pool, &engine, CASTLE).await;
    for label in [
        "login, offer open (10141)",
        "10320 completed (10140)",
        "10321 abandoned (10146)",
    ] {
        let (_, bound, _) = control.iter().find(|(l, _, _)| *l == label).unwrap();
        assert_eq!(
            bound.1,
            vec![OFFER_SET],
            "control: {label} must bind the offer in Castle"
        );
    }

    for world in WORLDS.into_iter().filter(|w| *w != CASTLE) {
        for (label, bound, sent) in replay_binding_triggers(&pool, &engine, world).await {
            assert_eq!(
                bound,
                (Vec::new(), Vec::new()),
                "{label} in {world} must bind nothing"
            );
            assert!(sent.is_empty(), "{label} in {world} sent {sent:?}");
        }
    }
}

/// **Every chain of the story names a world, and only the crossing accept
/// uses Harset (57).** A static scan of the seeded rows of chains 10130-10146:
/// each has a `world` condition; every `world` target is 8, except chain
/// 10130 (when seeded) which is 57 with exactly one `accept_mission` action;
/// no other chain targets 57; and chain 10130 binds nothing.
/// Revert proof: delete `world 8` from 10141; the scan finds a chain with no
/// world.
#[tokio::test]
async fn live_db_castle_side_return_q2_every_chain_names_a_world() {
    let pool = require_db_or_skip!();
    let in_range: Vec<i64> = sqlx::query_scalar(
        "SELECT chain_id::bigint FROM resources.content_chains \
         WHERE chain_id BETWEEN 10130 AND 10146 ORDER BY chain_id",
    )
    .fetch_all(&pool)
    .await
    .expect("chain query must succeed");
    for id in ALL_CHAINS {
        assert!(in_range.contains(&(id as i64)), "chain {id} must be seeded");
    }

    let no_world: Vec<i64> = sqlx::query_scalar(
        "SELECT c.chain_id::bigint FROM resources.content_chains c \
         WHERE c.chain_id BETWEEN 10130 AND 10146 \
           AND NOT EXISTS (SELECT 1 FROM resources.content_conditions k \
                           WHERE k.chain_id = c.chain_id AND k.condition_type = 'world') \
         ORDER BY c.chain_id",
    )
    .fetch_all(&pool)
    .await
    .expect("world scan must succeed");
    assert!(
        no_world.is_empty(),
        "chains with no world condition: {no_world:?}"
    );

    let off_world: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT chain_id::bigint, target_id::bigint FROM resources.content_conditions \
         WHERE condition_type = 'world' AND chain_id BETWEEN 10130 AND 10146 \
           AND target_id IS DISTINCT FROM 8 ORDER BY chain_id",
    )
    .fetch_all(&pool)
    .await
    .expect("world target scan must succeed");
    let allowed = |chain: &(i64, i64)| chain.0 == CROSSING_CHAIN as i64 && chain.1 == 57;
    assert!(
        off_world.iter().all(allowed),
        "only the crossing accept 10130 may use world 57; got {off_world:?}"
    );

    if in_range.contains(&(CROSSING_CHAIN as i64)) {
        let accepts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM resources.content_actions \
             WHERE chain_id = $1 AND action_type = 'accept_mission'",
        )
        .bind(CROSSING_CHAIN)
        .fetch_one(&pool)
        .await
        .expect("accept scan must succeed");
        let binds: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM resources.content_actions \
             WHERE chain_id = $1 AND action_type IN ('add_dialog_set', 'remove_dialog_set')",
        )
        .bind(CROSSING_CHAIN)
        .fetch_one(&pool)
        .await
        .expect("bind scan must succeed");
        assert_eq!(
            (accepts, binds),
            (1, 0),
            "10130 carries exactly one accept_mission and no bind"
        );
    }
}

// ── Wrong-origin crossing ────────────────────────────────────────────────

/// **A `stargate_crossed 'Castle'` from any world but Harset (57) fires
/// nothing, and from Harset it accepts nothing of 10321.** With 708
/// completed, the crossing is replayed from each world with 10320/10321 in
/// every state: no mission update, no bind and no change to 10321 anywhere,
/// and 10320 appears only when the crossing chain 10130 is seeded and the
/// origin is Harset. Revert proof: add a `stargate_crossed 'Castle'` trigger
/// to 10141 (which holds in world 8 with 10320 completed); the crossing from
/// Castle then rebinds the offer. When 10130 is seeded, flipping its
/// `world 57` to `world 8` also fails the Harset arm.
#[tokio::test]
async fn live_db_castle_side_return_q2_crossing_from_the_wrong_world_fires_nothing() {
    let pool = require_db_or_skip!();
    let (engine, present) =
        engine_with_present(&pool, &[&ALL_CHAINS[..], &[CROSSING_CHAIN][..]].concat()).await;
    let crossing_seeded = present.contains(&CROSSING_CHAIN);

    for world in WORLDS {
        for (prereq, own) in [
            (Prereq::Absent, Own::Absent),
            (Prereq::Active, Own::Absent),
            (Prereq::Completed, Own::Absent),
            (Prereq::Completed, Own::AtA),
            (Prereq::Completed, Own::AtB),
            (Prereq::Completed, Own::Done),
        ] {
            let mut held = missions(prereq, own);
            held.extend(q708(Q708::Completed));
            let mut mgr = staged(&pool, world, held).await;
            let before = (
                binds(&mgr),
                mission_of(&mgr, MISSION).map(|m| (m.status, m.current_step_id)),
                mission_of(&mgr, PREREQ).map(|m| m.status),
            );
            let (tx, mut rx) = mpsc::channel(256);
            fire_stargate_crossed(ENTITY, PLAYER, CASTLE, &engine, &tx, &mut mgr).await;
            let sent = drain(&mut rx);

            let accepts_prereq =
                crossing_seeded && world == HARSET && matches!(prereq, Prereq::Absent);
            if accepts_prereq {
                assert!(
                    mission_of(&mgr, PREREQ).is_some(),
                    "the Harset crossing must accept 10320 when 10130 is seeded"
                );
                continue;
            }
            assert!(
                sent.is_empty(),
                "crossing from {world} with 10320 {prereq:?}, 10321 {own:?} sent {sent:?}"
            );
            let after = (
                binds(&mgr),
                mission_of(&mgr, MISSION).map(|m| (m.status, m.current_step_id)),
                mission_of(&mgr, PREREQ).map(|m| m.status),
            );
            assert_eq!(
                after, before,
                "crossing from {world} with 10320 {prereq:?}, 10321 {own:?} moved state"
            );
        }
    }
}
