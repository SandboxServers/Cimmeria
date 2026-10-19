//! Missions 1565 and 1566 Tactics (US-20, chains 3085-3093). Carter offers
//! 1565 on step 4630 with her radio report, the static Jaffa pair of rows
//! 57/58 is the fight, and 1566 follows with a pair the server spawns on the
//! completion. Flank objective O5375 is left uncredited (D-US8).

use sqlx::PgPool;

use super::rig::*;
use crate::test_support::require_db_or_skip;

const CARTER: &str = "SGC_W1_SamCarter";
const JAFFA1: &str = "SGC_W1_TacticsJaffa1";
const JAFFA2: &str = "SGC_W1_TacticsJaffa2";
const JAFFA_2ND_1: &str = "SGC_W1_Tactics2Jaffa1";
const JAFFA_2ND_2: &str = "SGC_W1_Tactics2Jaffa2";

/// A rig on step 4630 of 1564, Carter's radio report still to come. The
/// assert pins the step the rest of the module relies on.
async fn on_4630(pool: &PgPool, archetype: Option<i32>) -> Rig {
    let mut rig = Rig::new(pool, archetype).await;
    rig.accept(1564).await;
    assert_eq!(
        rig.step(1564),
        Some(4630),
        "accept(1564) lands on step 4630"
    );
    rig.sent();
    rig
}

/// Kill both static 1565 Jaffa, in order, through the live engine.
async fn finish_1565(rig: &mut Rig) {
    rig.kill(JAFFA1).await;
    rig.kill(JAFFA2).await;
}

/// **Guard: Carter's radio report (dialog 5375) offers 1565 to a Human on
/// step 4630, and to no one else.** Revert proof: delete chain 3085 and the
/// status assert fails (`None`, not `ACTIVE`).
#[tokio::test]
async fn live_db_m1565_offered_with_the_radio_report() {
    let pool = require_db_or_skip!();
    for archetype in HUMANS {
        let mut rig = on_4630(&pool, Some(archetype)).await;
        rig.close(5375).await;
        assert_eq!(
            rig.status(1565),
            ACTIVE,
            "archetype {archetype} is offered 1565"
        );
    }
}

/// **Guard: 1565 completes only when both static Jaffa are dead.** Revert
/// proof: delete chain 3086, or drop the second `entity_tag_state` row of
/// it: the first kill then completes 5374 and the first assert fails
/// (`status(1565)` is COMPLETED, not ACTIVE).
#[tokio::test]
async fn live_db_m1565_completes_when_both_jaffa_die() {
    let pool = require_db_or_skip!();
    let mut rig = on_4630(&pool, Some(SOLDIER)).await;
    rig.close(5375).await;
    assert_eq!(rig.status(1565), ACTIVE);

    rig.kill(JAFFA1).await;
    assert_eq!(
        rig.status(1565),
        ACTIVE,
        "one Jaffa dead does not complete 1565"
    );
    assert!(!rig.objective_done(1565, 5374));

    rig.kill(JAFFA2).await;
    assert!(rig.objective_done(1565, 5374), "both dead complete 5374");
    assert_eq!(rig.status(1565), COMPLETED);
    assert!(
        !rig.objective_done(1565, 5375),
        "the optional flank O5375 is never credited (D-US8)"
    );
}

/// **Guard: a static pair already dead when 1565 is offered completes it on
/// accept (the 3087 backstop).** Revert proof: delete chain 3087 and 1565 stays
/// ACTIVE after the radio report closes, so the `COMPLETED` assert fails.
#[tokio::test]
async fn live_db_m1565_backstop_credits_a_pair_that_is_already_dead() {
    let pool = require_db_or_skip!();
    let mut rig = on_4630(&pool, Some(SOLDIER)).await;
    finish_1565(&mut rig).await;
    assert_eq!(rig.status(1565), None, "kills alone do not give 1565");

    rig.close(5375).await;
    assert!(rig.objective_done(1565, 5374), "accept credits 5374");
    assert_eq!(rig.status(1565), COMPLETED);
    assert_eq!(rig.status(1566), ACTIVE, "the completion still offers 1566");
}

/// **Guard: 1565's completion offers 1566 and spawns its second pair; the
/// pair completes 1566.** Revert proof: delete chain 3088 or 3089 and the
/// first assert (or the final COMPLETED) fails.
#[tokio::test]
async fn live_db_m1566_follows_with_a_spawned_pair() {
    let pool = require_db_or_skip!();
    let mut rig = on_4630(&pool, Some(SOLDIER)).await;
    rig.close(5375).await;
    finish_1565(&mut rig).await;
    assert_eq!(rig.status(1565), COMPLETED);

    assert_eq!(rig.status(1566), ACTIVE, "1565 completion offers 1566");
    assert!(rig.live(JAFFA_2ND_1), "the second pair spawns with 1566");
    assert!(rig.live(JAFFA_2ND_2));

    rig.kill(JAFFA_2ND_1).await;
    rig.kill(JAFFA_2ND_2).await;
    assert_eq!(rig.status(1566), COMPLETED);
}

/// **Guard: Carter's lines follow the active mission.** 5379 while 1565 is
/// active, 5380 while 1566 is active, and never both. Revert proof: delete
/// chain 3090 or 3091 and the matching press sends nothing.
#[tokio::test]
async fn live_db_m1565_carter_lines_follow_the_active_mission() {
    let pool = require_db_or_skip!();
    let mut rig = on_4630(&pool, Some(SOLDIER)).await;
    rig.close(5375).await;
    rig.advance(1564, 4631).await;
    rig.sent();

    assert!(rig.press(CARTER).await);
    assert_eq!(
        rig.sent(),
        vec![Sent::Dialog(5379)],
        "1565 active: line 5379"
    );

    finish_1565(&mut rig).await;
    rig.sent();
    assert!(rig.press(CARTER).await);
    assert_eq!(
        rig.sent(),
        vec![Sent::Dialog(5380)],
        "1566 active: line 5380"
    );
}

/// **Guard: a relog during 1566 respawns the second pair and restores
/// Carter's "!" marker on step 4631.** Revert proof: delete chain 3092 and
/// the pair is not live after `loaded()`; delete 3093 and the flag assert
/// fails.
#[tokio::test]
async fn live_db_m1566_load_into_a_fresh_instance_respawns_the_pair() {
    let pool = require_db_or_skip!();
    let mut rig = Rig::new(&pool, Some(SOLDIER)).await;
    rig.accept(1564).await;
    rig.advance(1564, 4631).await;
    rig.accept(1565).await;
    rig.complete(1565).await;
    rig.accept(1566).await;
    assert_eq!(rig.step(1566), Some(4635), "1566 starts on step 4635");
    assert!(!rig.live(JAFFA_2ND_1), "quiet setup spawns no second pair");
    assert!(!rig.live(JAFFA_2ND_2));

    rig.loaded().await;
    assert!(rig.live(JAFFA_2ND_1), "the load respawns the second pair");
    assert!(rig.live(JAFFA_2ND_2));
    assert!(
        rig.flags(CARTER) & STORY_ACTIVE != 0,
        "Carter is marked on the load"
    );
}

/// **Guard: only a Human is offered 1565.** A non-Human (or an unstamped
/// player) stays without the mission. Revert proof: drop the
/// `archetype gte 1` row of chain 3085 and the GM-created player (archetype
/// -1) is offered 1565, so the `None` assert fails for `UNSTAMPED`.
#[tokio::test]
async fn live_db_m1565_never_offered_to_a_non_human() {
    let pool = require_db_or_skip!();
    for archetype in NON_HUMANS.map(Some).into_iter().chain([UNSTAMPED]) {
        let mut rig = on_4630(&pool, archetype).await;
        rig.close(5375).await;
        assert_eq!(
            rig.status(1565),
            None,
            "archetype {archetype:?} is not offered 1565"
        );
    }
}
