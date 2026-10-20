//! Mission 1564 escort on step 4631 (US-16, chains 3073-3078): Carter follows
//! the player after her radio report, stands at P05 beside the Failsafe device
//! once the player enters the room, and a relog puts her back per step.
//! Human classes only.

use sqlx::PgPool;

use crate::cell::spawner::load_step_objectives;

use super::rig::{Rig, Sent, NON_HUMANS, PLAYER, SOLDIER, STORY_ACTIVE, UNSTAMPED};
use crate::test_support::require_db_or_skip;

const M1564: i32 = 1564;
const M1565: i32 = 1565;
const S4631: i32 = 4631;
const S4632: i32 = 4632;
const S4633: i32 = 4633;
const CARTER: &str = "SGC_W1_SamCarter";
const ROOM: &str = "SGC_W1.FailsafeRoom";
/// Carter's radio report (no buttons).
const D5375: i32 = 5375;
/// Carter's escort line while she walks with the player.
const D5376: i32 = 5376;
/// P05, Carter at the Failsafe (sealed in `placement.md`).
const P05: [f32; 3] = [147.0, 1.30, -197.0];

fn near(a: [f32; 3], b: [f32; 3], tol: f32) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() <= tol)
}

/// Carter's follow target: `follow_target_id` on her entity, which
/// `set_follow_target` writes.
fn follow(rig: &Rig) -> Option<u32> {
    let id = rig
        .mgr
        .find_entity_by_tag(PLAYER, CARTER)
        .expect("Carter is spawned");
    rig.mgr
        .get_entity(id)
        .expect("Carter exists")
        .follow_target_id
}

/// A player on 1564 step 4630 with the cell-to-base channel drained.
///
/// `Rig::new` leaves `step_objectives` empty (production loads it at startup),
/// so it is loaded here; `Rig` itself is not edited.
async fn on_4630(pool: &PgPool, archetype: Option<i32>) -> Rig {
    let mut rig = Rig::new(pool, archetype).await;
    rig.mgr.step_objectives = load_step_objectives(pool)
        .await
        .expect("load the step objectives");
    rig.accept(M1564).await;
    rig.sent();
    rig
}

/// **Revert proof:** delete chain 3073 and closing 5375 leaves Carter standing.
#[tokio::test]
async fn live_db_m1564_carter_follows_after_the_radio_report() {
    let pool = require_db_or_skip!();
    let mut rig = on_4630(&pool, Some(SOLDIER)).await;
    assert_eq!(follow(&rig), None, "Carter stands before the report");
    rig.close(D5375).await;
    assert_eq!(rig.step(M1564), Some(S4631), "closing 5375 moves to 4631");
    assert_eq!(follow(&rig), Some(PLAYER), "Carter follows the player");
}

/// **Revert proof:** delete chain 3074 and the room leaves her following with
/// no goal at P05.
#[tokio::test]
async fn live_db_m1564_carter_stops_at_the_failsafe() {
    let pool = require_db_or_skip!();
    let mut rig = on_4630(&pool, Some(SOLDIER)).await;
    rig.close(D5375).await;
    assert_eq!(follow(&rig), Some(PLAYER));
    rig.enter(ROOM).await;
    assert_eq!(rig.step(M1564), Some(S4632), "the room moves to 4632");
    assert_eq!(follow(&rig), None, "the follow is cleared");
    assert!(
        near(rig.pos(CARTER), P05, 0.1),
        "Carter walks to P05, got {:?}",
        rig.pos(CARTER)
    );
}

/// **Revert proof:** delete chain 3076 (first assertion), or drop the 1565
/// `neq` row of chain 3076 (second assertion shows 5376 over her tactics line).
#[tokio::test]
async fn live_db_m1564_carter_escort_line_shows_only_without_a_tactics_mission() {
    let pool = require_db_or_skip!();
    let mut rig = on_4630(&pool, Some(SOLDIER)).await;
    rig.advance(M1564, S4631).await;
    rig.sent();
    rig.press(CARTER).await;
    assert!(
        rig.sent().contains(&Sent::Dialog(D5376)),
        "5376 shows on 4631"
    );
    rig.accept(M1565).await;
    rig.sent();
    rig.press(CARTER).await;
    assert!(
        !rig.sent().contains(&Sent::Dialog(D5376)),
        "5376 stands aside while 1565 is active"
    );
}

/// **Revert proof:** delete chain 3075 (follow), 3077 or 3078 (position).
#[tokio::test]
async fn live_db_m1564_relog_restores_carter_per_step() {
    let pool = require_db_or_skip!();

    let mut rig = on_4630(&pool, Some(SOLDIER)).await;
    rig.advance(M1564, S4631).await;
    let mut rig = rig.relog(&pool).await;
    rig.mgr.step_objectives = load_step_objectives(&pool)
        .await
        .expect("load the step objectives");
    assert_eq!(
        follow(&rig),
        Some(PLAYER),
        "4631: Carter follows after a relog"
    );
    assert!(
        (rig.flags(CARTER) & STORY_ACTIVE) != 0,
        "4631: Carter keeps her marker"
    );

    for step in [S4632, S4633] {
        rig.advance(M1564, step).await;
        let mut r = rig.relog(&pool).await;
        r.mgr.step_objectives = load_step_objectives(&pool)
            .await
            .expect("load the step objectives");
        assert!(
            near(r.pos(CARTER), P05, 0.5),
            "{step}: Carter is at P05 after a relog, got {:?}",
            r.pos(CARTER)
        );
        rig = r;
    }
}

/// **Revert proof:** drop the `archetype gte 1` row of chain 3073 and the
/// unstamped player (reads -1... fails `gte`; with the row gone it passes) sets
/// Carter following; drop the `lte 4` row and a non-Human does.
#[tokio::test]
async fn live_db_m1564_escort_never_starts_for_a_non_human() {
    let pool = require_db_or_skip!();
    let cases: Vec<Option<i32>> = NON_HUMANS
        .iter()
        .map(|&a| Some(a))
        .chain([UNSTAMPED])
        .collect();
    for archetype in cases {
        let mut rig = on_4630(&pool, archetype).await;
        rig.close(D5375).await;
        assert_eq!(follow(&rig), None, "{archetype:?}: Carter does not follow");
    }
}
