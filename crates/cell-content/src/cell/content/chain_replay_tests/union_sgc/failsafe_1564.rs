//! Mission 1564 Self-Destruct (US-15, chains 3060-3071). The offer from Carter,
//! her radio report, the Failsafe room and device, and her final report.
//! Human classes only: the Union Human gate is `archetype gte 1` and `lte 4`.

use sqlx::PgPool;

use crate::cell::spawner::load_step_objectives;

use super::rig::{
    Rig, Sent, ACTIVE, COMPLETED, HUMANS, MISSION_OBJECT, NON_HUMANS, OFFER, SOLDIER, STORY_ACTIVE,
    UNSTAMPED,
};
use crate::test_support::require_db_or_skip;

const M1563: i32 = 1563;
const M1564: i32 = 1564;
const S4630: i32 = 4630;
const S4631: i32 = 4631;
const S4632: i32 = 4632;
const S4633: i32 = 4633;
const O5373: i32 = 5373;
const CARTER: &str = "SGC_W1_SamCarter";
const DEVICE: &str = "SGC_W1_FailsafeDevice";
const ROOM: &str = "SGC_W1.FailsafeRoom";
/// Carter's offer (accept on every screen).
const D5374: i32 = 5374;
/// Carter's radio report (no buttons).
const D5375: i32 = 5375;
/// Carter at the Failsafe (no buttons).
const D5377: i32 = 5377;
/// Carter's final report (no buttons).
const D5378: i32 = 5378;

/// Whether the rig has sent a `display_dialog` for `dialog_id` since the last drain.
fn showed(rig: &mut Rig, dialog_id: i32) -> bool {
    rig.sent().contains(&Sent::Dialog(dialog_id))
}

/// A player who has finished 1563, with the cell-to-base channel drained.
///
/// `Rig::new` leaves the `step_objectives` cache empty (production loads it
/// at startup), so this loads it: without it, advancing to 4633 gives the
/// step an empty roster and `complete_objective(5373)` is a no-op.
async fn after_1563(pool: &PgPool, archetype: Option<i32>) -> Rig {
    let mut rig = Rig::new(pool, archetype).await;
    rig.mgr.step_objectives = load_step_objectives(pool)
        .await
        .expect("load the step objectives");
    rig.accept(M1563).await;
    rig.finish(M1563).await;
    rig.sent();
    rig
}

/// **Revert proof:** delete chain 3064, 3065, 3067 or 3069 and the route stops
/// at the step that chain moves. Each Human walks the whole route: Carter's
/// offer and accept, the radio report, the Failsafe room, the device, the
/// final report.
#[tokio::test]
async fn live_db_m1564_human_runs_the_failsafe_route() {
    let pool = require_db_or_skip!();
    for archetype in HUMANS {
        let mut rig = after_1563(&pool, Some(archetype)).await;
        assert!(
            (rig.flags(CARTER) & OFFER) != 0,
            "archetype {archetype}: Carter offers 1564 after 1563"
        );
        rig.press(CARTER).await;
        assert!(
            showed(&mut rig, D5374),
            "archetype {archetype}: 5374 offered"
        );
        rig.choose(D5374, 1).await;
        assert_eq!(rig.status(M1564), ACTIVE, "archetype {archetype}: accepted");
        assert_eq!(rig.step(M1564), Some(S4630));
        assert!((rig.flags(CARTER) & STORY_ACTIVE) != 0);
        assert_eq!(rig.flags(CARTER) & OFFER, 0, "the offer goes on accept");

        rig.press(CARTER).await;
        assert!(showed(&mut rig, D5375), "archetype {archetype}: 5375 shown");
        rig.close(D5375).await;
        assert_eq!(rig.step(M1564), Some(S4631), "closing 5375 moves to 4631");

        rig.enter(ROOM).await;
        assert_eq!(rig.step(M1564), Some(S4632), "the room moves to 4632");
        assert!(
            (rig.flags(DEVICE) & MISSION_OBJECT) != 0,
            "the device glows"
        );

        rig.press(CARTER).await;
        assert!(showed(&mut rig, D5377), "archetype {archetype}: 5377 shown");
        rig.press(DEVICE).await;
        assert_eq!(rig.step(M1564), Some(S4633), "the device moves to 4633");
        assert_eq!(rig.flags(DEVICE) & MISSION_OBJECT, 0, "the glow clears");

        rig.press(CARTER).await;
        assert!(showed(&mut rig, D5378), "archetype {archetype}: 5378 shown");
        rig.close(D5378).await;
        assert!(rig.objective_done(M1564, O5373), "objective 5373 is done");
        assert_eq!(rig.status(M1564), COMPLETED, "5378 completes 1564");
        assert_eq!(
            rig.flags(CARTER) & STORY_ACTIVE,
            0,
            "Carter drops the marker"
        );
    }
}

/// **Revert proof:** drop the `archetype gte 1` and `lte 4` rows of chains
/// 3060-3062 and the `UNSTAMPED` case is offered 1564 (it reads -1). A
/// non-Human is never offered the mission and never gets 5374.
#[tokio::test]
async fn live_db_m1564_is_never_offered_to_a_non_human() {
    let pool = require_db_or_skip!();
    let cases: Vec<Option<i32>> = NON_HUMANS
        .iter()
        .map(|&a| Some(a))
        .chain([UNSTAMPED])
        .collect();
    for archetype in cases {
        let mut rig = after_1563(&pool, archetype).await;
        assert_eq!(
            rig.flags(CARTER) & OFFER,
            0,
            "{archetype:?}: Carter shows no offer"
        );
        rig.press(CARTER).await;
        rig.choose(D5374, 1).await;
        assert!(!showed(&mut rig, D5374), "{archetype:?}: 5374 is not shown");
        assert_eq!(rig.status(M1564), None, "{archetype:?}: 1564 is not given");
    }
}

/// **Revert proof:** drop the `step_status` row of chain 3065 or 3067 and the
/// player skips a step: entering the room on 4630 or pressing the device on
/// 4631 moves nothing and lights nothing.
#[tokio::test]
async fn live_db_m1564_steps_do_not_skip() {
    let pool = require_db_or_skip!();
    let mut rig = after_1563(&pool, Some(SOLDIER)).await;
    rig.choose(D5374, 1).await;
    rig.enter(ROOM).await;
    assert_eq!(rig.step(M1564), Some(S4630), "the room does not skip 4630");
    rig.close(D5375).await;
    assert_eq!(rig.step(M1564), Some(S4631), "5375 moves to 4631");
    rig.press(DEVICE).await;
    assert_eq!(
        rig.step(M1564),
        Some(S4631),
        "the device does not skip 4631"
    );
    assert_eq!(rig.flags(DEVICE) & MISSION_OBJECT, 0, "no glow on 4631");
}

/// **Revert proof:** delete chain 3070 or 3071 and the marker or the glow is
/// gone after a relog on the step that owns it.
#[tokio::test]
async fn live_db_m1564_markers_come_back_on_relog() {
    let pool = require_db_or_skip!();

    let mut rig = after_1563(&pool, Some(SOLDIER)).await;
    rig.choose(D5374, 1).await;
    let rig = rig.relog(&pool).await;
    assert_eq!(rig.step(M1564), Some(S4630), "a relog keeps step 4630");
    assert!(
        (rig.flags(CARTER) & STORY_ACTIVE) != 0,
        "Carter's marker comes back on 4630"
    );

    let mut rig = after_1563(&pool, Some(SOLDIER)).await;
    rig.choose(D5374, 1).await;
    rig.advance(M1564, S4632).await;
    let rig = rig.relog(&pool).await;
    assert_eq!(rig.step(M1564), Some(S4632), "a relog keeps step 4632");
    assert!(
        (rig.flags(CARTER) & STORY_ACTIVE) != 0,
        "Carter's marker comes back on 4632"
    );
    assert!(
        (rig.flags(DEVICE) & MISSION_OBJECT) != 0,
        "the device glow comes back on 4632"
    );
}

/// **Revert proof:** drop any `archetype` or `world` row of chains 3060-3071
/// and this reads it from the seed. Both Human bounds are on every chain; the
/// world 58 guard is on 3060, 3065, 3070 and 3071.
#[tokio::test]
async fn live_db_m1564_chains_carry_their_guards() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, String, Option<i32>, String, Option<String>)> = sqlx::query_as(
        "SELECT chain_id, condition_type, target_id, operator, value \
         FROM resources.content_conditions WHERE chain_id BETWEEN 3060 AND 3071",
    )
    .fetch_all(&pool)
    .await
    .expect("read the 1564 chain conditions");
    for chain in 3060..=3071 {
        for (op, val) in [("gte", "1"), ("lte", "4")] {
            assert!(
                rows.iter().any(|(c, t, _, o, v)| {
                    *c == chain && t == "archetype" && o == op && v.as_deref() == Some(val)
                }),
                "chain {chain} keeps the Human gate {op} {val}"
            );
        }
    }
    for chain in [3060, 3065, 3070, 3071] {
        assert!(
            rows.iter().any(|(c, t, id, o, _)| {
                *c == chain && t == "world" && *id == Some(58) && o == "eq"
            }),
            "chain {chain} is guarded to world 58"
        );
    }
}
