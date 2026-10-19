//! Mission 1654 Hammond turn-in (US-31, chains 3148-3152). Arrival from
//! Dakara on step 4939 lights Hammond; his report (5852 for a Human, 5853 for a
//! Free Jaffa) completes O5962 and so 1654; Woolsey then appears with "?".

use sqlx::PgPool;

use crate::cell::spawner::load_step_objectives;
use crate::test_support::require_db_or_skip;

use super::rig::{
    Rig, Sent, ACTIVE, ASGARD, COMPLETED, FREE_JAFFA, GOAULD, HUMANS, OFFER, SOLDIER, TURN_IN,
};

const M1654: i32 = 1654;
const M1655: i32 = 1655;
const O5961: i32 = 5961;
const O5962: i32 = 5962;
const D5852: i32 = 5852;
const D5853: i32 = 5853;
const HAMMOND: &str = "SGCW1_GenHammond";
const WOOLSEY: &str = "SGC_W1_Woolsey";
/// Woolsey's spawn point, P02 (`docs/analysis/union-sgc-continuation/placement.md`).
const P02: [f32; 3] = [197.5, 1.311, 43.6];

/// The objective cache is empty after `Rig::new` (production loads it at
/// startup), so every fresh rig, and every relogged one, loads it here.
async fn with_objectives(pool: &PgPool, mut rig: Rig) -> Rig {
    rig.mgr.step_objectives = load_step_objectives(pool)
        .await
        .expect("load the step objectives");
    rig
}

/// The player arrives from Dakara: 1654 accepted, O5961 done, `loaded()`, the
/// cell-to-base channel drained. O5962 is open and Hammond is lit.
async fn arrived(pool: &PgPool, archetype: Option<i32>) -> Rig {
    let mut rig = with_objectives(pool, Rig::new(pool, archetype).await).await;
    rig.accept(M1654).await;
    rig.objective(M1654, O5961).await;
    rig.loaded().await;
    rig.sent();
    rig
}

/// Log out and back in, with the objective cache loaded again. `relog` fires
/// `loaded()` before this, so the cache is only needed for later actions.
async fn relogged(pool: &PgPool, rig: Rig) -> Rig {
    with_objectives(pool, rig.relog(pool).await).await
}

/// How many entities carry `tag` in the player's instance (all spaces, so a
/// second spawn anywhere shows up).
fn count_tag(rig: &Rig, tag: &str) -> usize {
    rig.mgr
        .spaces
        .values()
        .flat_map(|space| space.entities.values())
        .filter(|e| e.tag.as_deref() == Some(tag))
        .count()
}

/// Whether `a` and `b` are within half a metre on the ground plane.
fn near(a: [f32; 3], b: [f32; 3]) -> bool {
    let (dx, dz) = (a[0] - b[0], a[2] - b[2]);
    (dx * dx + dz * dz).sqrt() <= 0.5
}

/// **Revert proof:** delete chain 3148 and Hammond shows no turn-in marker on
/// arrival. The Human and the Free Jaffa both arrive on step 4939.
#[tokio::test]
async fn live_db_m1654_arrival_lights_hammond() {
    let pool = require_db_or_skip!();
    for archetype in [SOLDIER, FREE_JAFFA] {
        let rig = arrived(&pool, Some(archetype)).await;
        assert!(
            rig.flags(HAMMOND) & TURN_IN != 0,
            "archetype {archetype}: Hammond is the turn-in marker on arrival"
        );
    }
}

/// **Revert proof:** delete chain 3149 or 3151 and the Human report does not
/// complete 1654 (3149 is the dialog, 3151 is the close that completes O5962).
#[tokio::test]
async fn live_db_m1654_human_report_completes_the_mission() {
    let pool = require_db_or_skip!();
    for archetype in HUMANS {
        let mut rig = arrived(&pool, Some(archetype)).await;
        rig.press(HAMMOND).await;
        let shown = rig.sent();
        assert!(
            shown.contains(&Sent::Dialog(D5852)),
            "archetype {archetype}: 5852 is the Human report"
        );
        assert!(
            !shown.contains(&Sent::Dialog(D5853)),
            "archetype {archetype}: 5853 is not shown to a Human"
        );
        rig.close(D5852).await;
        assert!(
            rig.objective_done(M1654, O5962),
            "archetype {archetype}: the close completes O5962"
        );
        assert_eq!(
            rig.status(M1654),
            COMPLETED,
            "archetype {archetype}: O5962 is the last required objective"
        );
        assert_eq!(
            rig.flags(HAMMOND) & TURN_IN,
            0,
            "archetype {archetype}: the turn-in marker is cleared"
        );
    }
}

/// **Revert proof:** delete chain 3150, or drop the `dialog_choice '5853'`
/// trigger of 3151, and the Free Jaffa report does not complete 1654.
#[tokio::test]
async fn live_db_m1654_jaffa_report_completes_the_mission() {
    let pool = require_db_or_skip!();
    let mut rig = arrived(&pool, Some(FREE_JAFFA)).await;
    rig.press(HAMMOND).await;
    let shown = rig.sent();
    assert!(
        shown.contains(&Sent::Dialog(D5853)),
        "5853 is the Free Jaffa report"
    );
    assert!(
        !shown.contains(&Sent::Dialog(D5852)),
        "5852 is not shown to a Free Jaffa"
    );
    rig.close(D5853).await;
    assert_eq!(
        rig.status(M1654),
        COMPLETED,
        "the Free Jaffa report completes 1654"
    );
    assert!(
        rig.live(WOOLSEY) && rig.flags(WOOLSEY) & OFFER != 0,
        "Woolsey also appears with the offer for a Free Jaffa"
    );
}

/// **Revert proof:** delete chain 3152 and Woolsey never appears. Woolsey is
/// live with "?" after the report, one copy per load, and back after a relog.
/// (The `entity_tag_state` row of 3152 only avoids a WARN: a second spawn of a
/// live tag is a no-op, so the count cannot show it.)
#[tokio::test]
async fn live_db_m1654_woolsey_arrives_once() {
    let pool = require_db_or_skip!();
    let mut rig = arrived(&pool, Some(SOLDIER)).await;
    rig.press(HAMMOND).await;
    rig.close(D5852).await;
    assert!(rig.live(WOOLSEY), "Woolsey arrives after the report");
    assert!(
        rig.flags(WOOLSEY) & OFFER != 0,
        "Woolsey shows the \"?\" offer"
    );
    assert!(
        near(rig.pos(WOOLSEY), P02),
        "Woolsey stands at P02 beside Hammond"
    );
    assert_eq!(count_tag(&rig, WOOLSEY), 1, "one Woolsey after the report");

    rig.loaded().await;
    assert_eq!(count_tag(&rig, WOOLSEY), 1, "one Woolsey after a load");

    let mut rig = relogged(&pool, rig).await;
    assert!(rig.live(WOOLSEY), "Woolsey is back after a relog");
    assert_eq!(count_tag(&rig, WOOLSEY), 1, "one Woolsey after a relog");

    rig.loaded().await;
    assert_eq!(
        count_tag(&rig, WOOLSEY),
        1,
        "one Woolsey after the last load"
    );
}

/// **Revert proof:** drop the `objective_status` row of 3149 and the second
/// press shows the report again. The mission waits for Dakara's O5961, which is
/// the C3 contract: O5962 completes, and 1654 stays active until O5961 is done.
#[tokio::test]
async fn live_db_m1654_report_once_even_if_o5961_is_open() {
    let pool = require_db_or_skip!();
    let mut rig = with_objectives(&pool, Rig::new(&pool, Some(SOLDIER)).await).await;
    rig.accept(M1654).await;
    rig.loaded().await;
    rig.sent();

    rig.press(HAMMOND).await;
    rig.close(D5852).await;
    assert!(
        rig.objective_done(M1654, O5962),
        "the report completes O5962"
    );
    assert_eq!(
        rig.status(M1654),
        ACTIVE,
        "1654 waits for Dakara's O5961 (C3 contract)"
    );

    rig.sent();
    rig.press(HAMMOND).await;
    let again = rig.sent();
    assert!(
        !again.contains(&Sent::Dialog(D5852)),
        "the second press sends no second report"
    );
}

/// **Revert proof:** widen the Human guard of 3149 to `lt 5` (or drop the
/// `archetype eq '7'` row of 3150) and an Asgard or Goauld gets a report and
/// completes 1654. Archetypes 5 and 6 never reach the SGC on this mission.
#[tokio::test]
async fn live_db_m1654_asgard_and_goauld_get_no_report() {
    let pool = require_db_or_skip!();
    for archetype in [ASGARD, GOAULD] {
        let mut rig = arrived(&pool, Some(archetype)).await;
        rig.press(HAMMOND).await;
        let shown = rig.sent();
        assert!(
            !shown.contains(&Sent::Dialog(D5852)) && !shown.contains(&Sent::Dialog(D5853)),
            "archetype {archetype}: no report dialog"
        );
        assert_eq!(
            rig.status(M1654),
            ACTIVE,
            "archetype {archetype}: 1654 is not completed"
        );
    }
}

/// **Revert proof:** drop the `step_status` row of 3148 or 3149 and Hammond
/// does not light, or does not report, when the player is not on 4939.
#[tokio::test]
async fn live_db_m1654_hammond_silent_without_the_step() {
    let pool = require_db_or_skip!();
    let mut rig = with_objectives(&pool, Rig::new(&pool, Some(SOLDIER)).await).await;
    rig.press(HAMMOND).await;
    let shown = rig.sent();
    assert!(
        !shown.contains(&Sent::Dialog(D5852)),
        "no report without 1654 on 4939"
    );
    rig.loaded().await;
    assert_eq!(
        rig.flags(HAMMOND) & TURN_IN,
        0,
        "no turn-in marker without 1654 on 4939"
    );
    assert_eq!(rig.status(M1655), None, "no 1655 without the turn-in");
}
