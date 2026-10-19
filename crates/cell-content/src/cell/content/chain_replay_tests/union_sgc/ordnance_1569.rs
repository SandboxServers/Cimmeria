//! Mission 1569 Ordinance (US-23, chains 3110-3122). The offer after 1564,
//! the six storage crates (one objective each), the relog relight. The class
//! kit is chains 3041-3044 and is checked at the end of the route. Human
//! classes only: the Union Human gate is `archetype gte 1` and `lte 4`.

use sqlx::PgPool;

use crate::cell::spawner::load_step_objectives;

use super::rig::{
    Rig, Sent, ACTIVE, ARCHAEOLOGIST, COMPLETED, HUMANS, MISSION_OBJECT, NON_HUMANS, SOLDIER,
    UNSTAMPED,
};
use crate::test_support::require_db_or_skip;

const M1564: i32 = 1564;
const M1569: i32 = 1569;

/// Each storage crate, its required objective, and the dialog its open shows.
const CRATES: [(&str, i32, Option<i32>); 6] = [
    ("SGC_W1_Crate1", 5381, None),
    ("SGC_W1_Crate2", 5385, Some(5385)),
    ("SGC_W1_Crate3", 5386, Some(5386)),
    ("SGC_W1_Crate4", 5387, Some(5387)),
    ("SGC_W1_Crate5", 5388, Some(5388)),
    ("SGC_W1_Crate6", 5389, Some(5389)),
];

/// The dialog ids a drained channel carries, in send order.
fn dialogs(sent: &[Sent]) -> Vec<i32> {
    sent.iter()
        .filter_map(|s| match s {
            Sent::Dialog(id) => Some(*id),
            _ => None,
        })
        .collect()
}

/// Whether a drained channel carries an `add_item` for `item`.
fn granted(sent: &[Sent], item: i32) -> bool {
    sent.iter()
        .any(|s| matches!(s, Sent::Item(id, _, _) if *id == item))
}

/// A Human-or-other player who has finished 1564, with the channel drained.
///
/// `Rig::new` leaves the `step_objectives` cache empty (production loads it
/// at startup), so this loads it: without it, `complete_objective` on 4640
/// finds no roster and does nothing.
async fn after_1564(pool: &PgPool, archetype: Option<i32>) -> Rig {
    let mut rig = Rig::new(pool, archetype).await;
    rig.mgr.step_objectives = load_step_objectives(pool)
        .await
        .expect("load the step objectives");
    rig.accept(M1564).await;
    rig.finish(M1564).await;
    rig.sent();
    rig
}

/// **Revert proof:** delete chain 3110, or drop one of its six glow rows, and
/// a crate is dark on offer. Each Human is offered 1569 on the 1564 completion
/// with all six crates lit.
#[tokio::test]
async fn live_db_m1569_offered_after_1564_with_six_lit_crates() {
    let pool = require_db_or_skip!();
    for archetype in HUMANS {
        let rig = after_1564(&pool, Some(archetype)).await;
        assert_eq!(
            rig.status(M1569),
            ACTIVE,
            "archetype {archetype}: 1569 is offered after 1564"
        );
        for (tag, _, _) in CRATES {
            assert!(
                (rig.flags(tag) & MISSION_OBJECT) != 0,
                "archetype {archetype}: {tag} is lit on offer"
            );
        }
    }
}

/// **Revert proof:** delete any of chains 3111-3116 and that crate never
/// credits its objective; drop the `objective_status` row of chain 3112 and the
/// second press of Crate2 shows 5385 again. Each crate, opened in order,
/// credits its own objective, clears its glow and shows its dialog once.
#[tokio::test]
async fn live_db_m1569_each_crate_credits_its_objective_once() {
    let pool = require_db_or_skip!();
    let mut rig = after_1564(&pool, Some(SOLDIER)).await;
    for (tag, objective, dialog) in CRATES {
        rig.press(tag).await;
        let shown = dialogs(&rig.sent());
        assert_eq!(
            shown,
            dialog.into_iter().collect::<Vec<i32>>(),
            "{tag}: its open shows its dialog once"
        );
        assert!(
            rig.objective_done(M1569, objective),
            "{tag}: objective {objective} is done"
        );
        assert_eq!(rig.flags(tag) & MISSION_OBJECT, 0, "{tag}: the glow clears");
    }

    rig.press("SGC_W1_Crate2").await;
    assert!(
        dialogs(&rig.sent()).is_empty(),
        "a repeat press on Crate2 shows no second 5385"
    );
    assert!(rig.objective_done(M1569, 5385), "5385 stays done");
}

/// **Revert proof:** delete chain 3116 and the sixth crate never completes
/// 1569. With it, the route ends: Soldiers get the 3041 items (3260, 7373),
/// Archaeologists the 3044 items (6843, 7373), and 3260 is never granted to an
/// Archaeologist.
#[tokio::test]
async fn live_db_m1569_sixth_crate_completes_and_pays_the_class_kit() {
    let pool = require_db_or_skip!();

    let mut rig = after_1564(&pool, Some(SOLDIER)).await;
    let mut all = Vec::new();
    for (tag, _, _) in CRATES {
        rig.press(tag).await;
        all.extend(rig.sent());
    }
    assert_eq!(
        rig.status(M1569),
        COMPLETED,
        "the sixth crate completes 1569"
    );
    assert!(granted(&all, 3260), "Soldier: the SK37 LMG (3041)");
    assert!(granted(&all, 7373), "Soldier: the armored jacket (3041)");

    let mut rig = after_1564(&pool, Some(ARCHAEOLOGIST)).await;
    let mut all = Vec::new();
    for (tag, _, _) in CRATES {
        rig.press(tag).await;
        all.extend(rig.sent());
    }
    assert_eq!(
        rig.status(M1569),
        COMPLETED,
        "the sixth crate completes 1569"
    );
    assert!(granted(&all, 6843), "Archaeologist: the emitter (3044)");
    assert!(granted(&all, 7373), "Archaeologist: the jacket (3044)");
    assert!(!granted(&all, 3260), "the LMG is the Soldier's (3041 only)");
}

/// **Revert proof:** drop the `step_status` row of chain 3112 and a press on
/// Crate2 before the offer shows 5385 and credits the objective. With the row,
/// nothing happens and 1569 is not held.
#[tokio::test]
async fn live_db_m1569_crates_do_nothing_before_the_offer() {
    let pool = require_db_or_skip!();
    let mut rig = Rig::new(&pool, Some(SOLDIER)).await;
    rig.mgr.step_objectives = load_step_objectives(&pool)
        .await
        .expect("load the step objectives");
    rig.press("SGC_W1_Crate2").await;
    assert!(
        dialogs(&rig.sent()).is_empty(),
        "no 5385 before 1564 is done"
    );
    assert!(!rig.objective_done(M1569, 5385), "5385 is not credited");
    assert_eq!(rig.status(M1569), None, "1569 is not held before the offer");
}

/// **Revert proof:** delete any of chains 3120-3122, or drop an
/// `objective_status` row of chains 3117-3119, and the relog lights the wrong
/// crates. Press Crate1-3, relog: Crate1-3 stay dark, Crate4-6 are lit.
#[tokio::test]
async fn live_db_m1569_relog_relights_only_unopened_crates() {
    let pool = require_db_or_skip!();
    let mut rig = after_1564(&pool, Some(SOLDIER)).await;
    for (tag, _, _) in &CRATES[..3] {
        rig.press(tag).await;
    }
    let mut rig = rig.relog(&pool).await;
    rig.mgr.step_objectives = load_step_objectives(&pool)
        .await
        .expect("load the step objectives");
    assert_eq!(rig.step(M1569), Some(4640), "a relog keeps step 4640");
    for (tag, _, _) in &CRATES[..3] {
        assert_eq!(
            rig.flags(tag) & MISSION_OBJECT,
            0,
            "{tag}: an opened crate stays dark after a relog"
        );
    }
    for (tag, _, _) in &CRATES[3..] {
        assert!(
            (rig.flags(tag) & MISSION_OBJECT) != 0,
            "{tag}: an unopened crate is lit after a relog"
        );
    }
}

/// **Revert proof:** drop the `archetype gte 1` row of chain 3110 and the
/// non-Human cases are offered 1569 (the `UNSTAMPED` case reads -1). A
/// non-Human, or a player with no archetype, is never offered the mission and
/// no crate is lit for them.
#[tokio::test]
async fn live_db_m1569_never_offered_to_a_non_human() {
    let pool = require_db_or_skip!();
    let cases: Vec<Option<i32>> = NON_HUMANS
        .iter()
        .map(|&a| Some(a))
        .chain([UNSTAMPED])
        .collect();
    for archetype in cases {
        let mut rig = after_1564(&pool, archetype).await;
        rig.loaded().await;
        assert_eq!(
            rig.status(M1569),
            None,
            "{archetype:?}: 1569 is not offered"
        );
        for (tag, _, _) in CRATES {
            assert_eq!(
                rig.flags(tag) & MISSION_OBJECT,
                0,
                "{archetype:?}: {tag} is not lit"
            );
        }
    }
}
