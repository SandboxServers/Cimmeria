//! Return state and the relog matrix (US-34, chains 3171-3175). SGC_W1 is
//! instanced per player, so every return rebuilds the story Jaffa from the
//! spawnlist; `player_loaded` despawns the ones whose missions are spent.
//! Default D-US5: no Gate Room Jaffa once 1568 is done, no story Jaffa for a
//! Free Jaffa, and a route pair (Tactics, Clear group) stays only while a
//! mission still needs it. The state is set quietly, then `relog` runs the
//! real `player_loaded` chains on a fresh instance.

use std::collections::HashMap;

use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};

use super::rig::*;
use crate::cell::content::execute_actions;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::spawner::load_step_objectives;
use crate::test_support::require_db_or_skip;

const M1562: i32 = 1562;
const M1564: i32 = 1564;
const M1565: i32 = 1565;
const M1566: i32 = 1566;
const M1567: i32 = 1567;
const M1568: i32 = 1568;
const M1569: i32 = 1569;
const M1570: i32 = 1570;
const M1651: i32 = 1651;
const S4639: i32 = 4639;
const S4931: i32 = 4931;
const CARTER: &str = "SGC_W1_SamCarter";
/// Carter's place in the Gate Room on 4931 (US-30 chain 3139), sealed in
/// `docs/analysis/union-sgc-continuation/placement.md` row P08.
const P08: [f32; 3] = [164.50, -10.80, 18.00];
/// The Tactics pair (spawnlist rows 57/58), spent by 1565.
const TACTICS: [&str; 2] = ["SGC_W1_TacticsJaffa1", "SGC_W1_TacticsJaffa2"];
/// The Clear group (spawnlist rows 59-63), needed while 1565-1567 run.
const CLEAR: [&str; 5] = [
    "SGC_W1_ClearJaffa1",
    "SGC_W1_ClearJaffa2",
    "SGC_W1_ClearJaffa3",
    "SGC_W1_ClearJaffa4",
    "SGC_W1_ClearJaffa5",
];
/// The Gate Room Jaffa (spawnlist rows 65-67), spent once 1568 is completed.
const GROOM: [&str; 3] = [
    "SGC_W1_GroomJaffa1",
    "SGC_W1_GroomJaffa2",
    "SGC_W1_GroomJaffa3",
];
/// The 1566 second pair, spawned by the server, not by the spawnlist.
const SECOND_PAIR_1: &str = "SGC_W1_Tactics2Jaffa1";
/// Spawnlist template of the story Jaffa (the Tactics rows use 34).
const JAFFA_TEMPLATE: i32 = 34;

/// Every story Jaffa tag: the ten rows US-11 tagged.
fn story_tags() -> Vec<&'static str> {
    TACTICS
        .iter()
        .chain(CLEAR.iter())
        .chain(GROOM.iter())
        .copied()
        .collect()
}

/// A rig with the step-objective roster loaded, as production has it at
/// startup (`Rig::new` leaves the roster empty).
async fn fresh(pool: &PgPool, archetype: Option<i32>) -> Rig {
    let mut rig = Rig::new(pool, archetype).await;
    rig.mgr.step_objectives = load_step_objectives(pool)
        .await
        .expect("load the step objectives");
    rig
}

/// Log out and back in. `relog` builds a fresh instance from the spawnlist and
/// fires `player_loaded` itself, before the roster is back: no return chain
/// reads the roster, so the reload below only serves later steps.
async fn relogged(rig: Rig, pool: &PgPool) -> Rig {
    let mut rig = rig.relog(pool).await;
    rig.mgr.step_objectives = load_step_objectives(pool)
        .await
        .expect("load the step objectives");
    rig
}

/// Give each mission quietly and complete it: no chain fires.
async fn complete_all(rig: &mut Rig, missions: &[i32]) {
    for &mission_id in missions {
        rig.accept(mission_id).await;
        rig.complete(mission_id).await;
    }
}

/// Every tag in `tags` is live (`want` true) or gone (`want` false) on return.
fn assert_live(rig: &Rig, tags: &[&str], want: bool) {
    for tag in tags {
        assert_eq!(
            rig.live(tag),
            want,
            "{tag} should be live == {want} after the return"
        );
    }
}

/// Whether `p` is within `metres` of `q` on the three axes.
fn within(p: [f32; 3], q: [f32; 3], metres: f32) -> bool {
    p.iter().zip(q.iter()).all(|(a, b)| (a - b).abs() <= metres)
}

/// Spawn `tag` into the player's space with the same executor path a chain
/// takes, so the same-tag guard runs as it does in production.
async fn spawn_tagged(rig: &mut Rig, tag: &str, at: [f32; 3]) {
    let (tx, mut rx) = mpsc::channel::<CellToBaseMsg>(64);
    let resolved = ResolvedActions {
        actions: vec![(
            0,
            Action::SpawnEntity {
                template_id: JAFFA_TEMPLATE,
                position: at,
                heading: 0.0,
                tag: tag.to_string(),
                is_stationary: None,
                aggression: None,
                allow_shared: None,
            },
        )],
        action_delays: vec![0],
        params: HashMap::new(),
    };
    execute_actions(
        resolved,
        PLAYER,
        PLAYER_ID,
        &tx,
        &mut rig.mgr,
        &ChainEngine::new(),
    )
    .await;
    while rx.try_recv().is_ok() {}
}

/// **Guard: a Human who has not started the 1564 route keeps every story Jaffa.**
/// Revert proof: drop chain 3174's `1564 completed` row and 3174 fires for a
/// player who never took the route, so the Tactics pair and Clear group are
/// despawned and this fails.
#[tokio::test]
async fn live_db_return_fresh_human_keeps_every_story_jaffa() {
    let pool = require_db_or_skip!();

    let mut rig = fresh(&pool, Some(SOLDIER)).await;
    rig.accept(M1562).await;
    let rig = relogged(rig, &pool).await;

    assert_live(&rig, &story_tags(), true);
}

/// **Revert proof:** delete chain 3171 and the Gate Room Jaffa stay on the
/// floor after 1568 completes.
#[tokio::test]
async fn live_db_return_after_1568_clears_the_gate_room() {
    let pool = require_db_or_skip!();

    let mut rig = fresh(&pool, Some(SOLDIER)).await;
    complete_all(&mut rig, &[M1564, M1565, M1566, M1567, M1568]).await;
    let rig = relogged(rig, &pool).await;

    assert_live(&rig, &GROOM, false);
    assert_live(&rig, &TACTICS, false);
    assert_live(&rig, &CLEAR, false);
}

/// **Revert proof:** change chain 3171's `mission_status 1568` operator from
/// `eq 'completed'` to `neq 'completed'` and the Gate Room is cleared while
/// 1568 is still open.
#[tokio::test]
async fn live_db_return_mid_1568_keeps_the_gate_room() {
    let pool = require_db_or_skip!();

    let mut rig = fresh(&pool, Some(SOLDIER)).await;
    rig.accept(M1564).await;
    rig.complete(M1564).await;
    rig.accept(M1568).await;
    rig.advance(M1568, S4639).await;
    let rig = relogged(rig, &pool).await;

    assert_live(&rig, &GROOM, true);
}

/// **Revert proof:** delete chain 3172 and the story Jaffa stay up for a Free
/// Jaffa, who has no use for them.
#[tokio::test]
async fn live_db_return_free_jaffa_sees_no_story_jaffa() {
    let pool = require_db_or_skip!();

    let rig = fresh(&pool, Some(FREE_JAFFA)).await;
    let rig = relogged(rig, &pool).await;

    assert_live(&rig, &story_tags(), false);
}

/// **Revert proof:** delete chain 3173 and the Tactics pair stays up after
/// 1565 is completed, while 1566 is still active.
#[tokio::test]
async fn live_db_return_tactics_go_once_1565_is_done() {
    let pool = require_db_or_skip!();

    let mut rig = fresh(&pool, Some(SOLDIER)).await;
    rig.accept(M1564).await;
    complete_all(&mut rig, &[M1565]).await;
    rig.accept(M1566).await;
    let rig = relogged(rig, &pool).await;

    assert_live(&rig, &TACTICS, false);
    assert_live(&rig, &CLEAR, true);
}

/// **Revert proof:** drop chain 3174's `1566 neq 'active'` row and the route
/// chain spends the Clear group while 1566 is still running.
#[tokio::test]
async fn live_db_return_clear_group_stays_while_1567_is_owed() {
    let pool = require_db_or_skip!();

    let mut rig = fresh(&pool, Some(SOLDIER)).await;
    complete_all(&mut rig, &[M1564, M1565]).await;
    rig.accept(M1566).await;
    let rig = relogged(rig, &pool).await;

    assert_live(&rig, &CLEAR, true);
}

/// **Revert proof:** delete chain 3174 and the Tactics pair and Clear group
/// both stay up after 1564 when 1565-1567 were never taken.
#[tokio::test]
async fn live_db_return_route_spent_after_1564_without_side_missions() {
    let pool = require_db_or_skip!();

    let mut rig = fresh(&pool, Some(SOLDIER)).await;
    complete_all(&mut rig, &[M1564]).await;
    let rig = relogged(rig, &pool).await;

    assert_live(&rig, &TACTICS, false);
    assert_live(&rig, &CLEAR, false);
}

/// **Revert proof:** delete chain 3175 and the Clear group stays up after
/// 1567 is completed, while 1564 is still active.
#[tokio::test]
async fn live_db_return_clear_group_goes_once_1567_is_done() {
    let pool = require_db_or_skip!();

    let mut rig = fresh(&pool, Some(SOLDIER)).await;
    rig.accept(M1564).await;
    complete_all(&mut rig, &[M1565, M1566, M1567]).await;
    let rig = relogged(rig, &pool).await;

    assert_live(&rig, &CLEAR, false);
    assert_live(&rig, &TACTICS, false);
}

/// **Revert proof:** delete chain 3171 and the Gate Room Jaffa are still up
/// when a player returns from Dakara through C2b (1651 on 4931). Carter is
/// US-30's chain 3139 and must stand at P08 in the same return.
#[tokio::test]
async fn live_db_return_c2b_from_dakara_finds_the_gate_room_clear() {
    let pool = require_db_or_skip!();

    let mut rig = fresh(&pool, Some(SOLDIER)).await;
    complete_all(
        &mut rig,
        &[M1562, M1564, M1565, M1566, M1567, M1568, M1569, M1570],
    )
    .await;
    rig.accept(M1651).await;
    rig.advance(M1651, S4931).await;
    let rig = relogged(rig, &pool).await;

    assert_live(&rig, &GROOM, false);
    assert!(
        within(rig.pos(CARTER), P08, 0.5),
        "Carter stands at P08 on 4931, got {:?}",
        rig.pos(CARTER)
    );
}

/// **Finding (US-20, not a fix):** `find_entity_by_tag` counts a corpse still
/// in the space, so a dead second-pair Jaffa blocks a same-space respawn
/// (`spawn_entity` refuses on `tag_already_live`) until a `despawn_entity`
/// removes the corpse. A relog is a fresh instance, so the corpse blocks only
/// a same-space respawn. This asserts the current behaviour; no engine code
/// changes here.
#[tokio::test]
async fn live_db_return_dead_second_pair_corpse_blocks_respawn() {
    let pool = require_db_or_skip!();

    let mut rig = fresh(&pool, Some(SOLDIER)).await;
    let at = rig.pos(TACTICS[0]);
    spawn_tagged(&mut rig, SECOND_PAIR_1, at).await;
    assert!(rig.live(SECOND_PAIR_1), "the second pair spawns");

    rig.kill(SECOND_PAIR_1).await;
    assert!(!rig.live(SECOND_PAIR_1), "the killed Jaffa is a corpse");

    spawn_tagged(&mut rig, SECOND_PAIR_1, at).await;
    assert!(
        rig.mgr.find_entity_by_tag(PLAYER, SECOND_PAIR_1).is_some(),
        "the corpse is still counted by find_entity_by_tag"
    );
    assert!(
        !rig.live(SECOND_PAIR_1),
        "the same-space respawn is refused: the corpse blocks it"
    );

    let despawn = ResolvedActions {
        actions: vec![(
            0,
            Action::DespawnEntity {
                entity_tag: SECOND_PAIR_1.to_string(),
            },
        )],
        action_delays: vec![0],
        params: HashMap::new(),
    };
    let (tx, mut rx) = mpsc::channel::<CellToBaseMsg>(64);
    execute_actions(
        despawn,
        PLAYER,
        PLAYER_ID,
        &tx,
        &mut rig.mgr,
        &ChainEngine::new(),
    )
    .await;
    while rx.try_recv().is_ok() {}
    assert!(
        rig.mgr.find_entity_by_tag(PLAYER, SECOND_PAIR_1).is_none(),
        "the despawn removes the corpse"
    );

    spawn_tagged(&mut rig, SECOND_PAIR_1, at).await;
    assert!(
        rig.live(SECOND_PAIR_1),
        "after the despawn the respawn lives"
    );
}
