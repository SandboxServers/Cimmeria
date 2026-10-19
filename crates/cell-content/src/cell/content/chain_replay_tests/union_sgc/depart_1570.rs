//! Mission 1570 and crossing C1 (US-24, chains 3123-3131). The offer from
//! Hammond or Teal'c after 1568 and 1569, the two accepts, Harriman's dial
//! (F4: advance at 0, dial at 0 and 2500, arrive at 3000), and the relog and
//! recovery chains that cover an interrupted dial. Human classes only: the
//! Union Human gate is `archetype gte 1` and `lte 4`.

use sqlx::PgPool;

use crate::cell::spawner::load_step_objectives;

use super::rig::{
    Rig, Sent, ACTIVE, HUMANS, NON_HUMANS, OFFER, PLAYER, SOLDIER, STORY_ACTIVE, UNSTAMPED,
};
use crate::test_support::require_db_or_skip;

const M1568: i32 = 1568;
const M1569: i32 = 1569;
const M1570: i32 = 1570;
const S4641: i32 = 4641;
const S4642: i32 = 4642;
/// Hammond's offer, Teal'c's offer, Hammond's follow-up, Teal'c's follow-up.
const D5392: i32 = 5392;
const D5393: i32 = 5393;
const D5395: i32 = 5395;
const D5396: i32 = 5396;
const HAMMOND: &str = "SGCW1_GenHammond";
const TEALC: &str = "SGC_W1_Tealc";
const HARRIMAN: &str = "SGC_W1_Harriman";
/// The two dial sequences of C1 (`docs/analysis/union-sgc-continuation/meshes.md`).
const DIAL_OPEN: i32 = 10047;
const DIAL_CROSS: i32 = 10060;
/// The Dakara_E1 arrival pin (the Dakara campaign's point; do not change it).
const DAKARA_WORLD: &str = "Dakara_E1";
const DAKARA_PIN: [f32; 3] = [96.87, -16.75, 243.23];

/// A fresh SGC_W1 rig with the `step_objectives` cache loaded (production
/// loads it at startup; `Rig::new` leaves it empty).
async fn new_rig(pool: &PgPool, archetype: Option<i32>) -> Rig {
    let mut rig = Rig::new(pool, archetype).await;
    rig.mgr.step_objectives = load_step_objectives(pool)
        .await
        .expect("load the step objectives");
    rig
}

/// Log out and back in, with the `step_objectives` cache reloaded.
async fn relogged(rig: Rig, pool: &PgPool) -> Rig {
    let mut rig = rig.relog(pool).await;
    rig.mgr.step_objectives = load_step_objectives(pool)
        .await
        .expect("load the step objectives");
    rig
}

/// A player who has finished 1568 and 1569 and logged in, with the channel
/// drained.
async fn ready(pool: &PgPool, archetype: Option<i32>) -> Rig {
    let mut rig = new_rig(pool, archetype).await;
    rig.accept(M1568).await;
    rig.complete(M1568).await;
    rig.accept(M1569).await;
    rig.complete(M1569).await;
    rig.loaded().await;
    rig.sent();
    rig
}

/// Press an officer the way `handle_interact` does. `fire_interact_tag`
/// alone never pins `last_interaction_target`, so a `dialog_choice` chain
/// that shows that officer's line (5395, 5396) would find no speaker and drop
/// the display. The pin is the player's interaction target, set on press.
async fn press_pinned(rig: &mut Rig, tag: &str) {
    let target = rig
        .mgr
        .find_entity_by_tag(PLAYER, tag)
        .unwrap_or_else(|| panic!("{tag} is spawned"));
    rig.press(tag).await;
    if let Some(player) = rig.mgr.get_entity_mut(PLAYER) {
        player.last_interaction_target = Some(target);
    }
}

/// A Soldier who has taken 1570 from Hammond (4641 active), channel drained.
async fn accepted_from_hammond(pool: &PgPool) -> Rig {
    let mut rig = ready(pool, Some(SOLDIER)).await;
    press_pinned(&mut rig, HAMMOND).await;
    rig.choose(D5392, 0).await;
    rig.sent();
    rig
}

/// How many `cross_world_teleport` rows the rig has sent since the last drain.
fn travel_count(rig: &mut Rig) -> usize {
    rig.sent()
        .iter()
        .filter(|s| matches!(s, Sent::Travel(..)))
        .count()
}

/// **Revert proof:** delete chain 3123, or drop its 1569 row, and no Human
/// sees "?" on Hammond or Teal'c after 1568 and 1569. A player with only 1568
/// done sees neither.
#[tokio::test]
async fn live_db_m1570_both_officers_offer_after_1568_and_1569() {
    let pool = require_db_or_skip!();
    for archetype in HUMANS {
        let rig = ready(&pool, Some(archetype)).await;
        assert!(
            (rig.flags(HAMMOND) & OFFER) != 0,
            "archetype {archetype}: Hammond offers 1570"
        );
        assert!(
            (rig.flags(TEALC) & OFFER) != 0,
            "archetype {archetype}: Teal'c offers 1570"
        );
    }
    let mut rig = new_rig(&pool, Some(SOLDIER)).await;
    rig.accept(M1568).await;
    rig.complete(M1568).await;
    rig.loaded().await;
    assert_eq!(
        rig.flags(HAMMOND) & OFFER,
        0,
        "1568 alone: no offer from Hammond"
    );
    assert_eq!(
        rig.flags(TEALC) & OFFER,
        0,
        "1568 alone: no offer from Teal'c"
    );
}

/// **Revert proof:** delete chain 3124 (Hammond's press shows nothing) or
/// chain 3126 (the accept never runs, so 1570 stays not started and the
/// follow-up 5395 is never shown).
#[tokio::test]
async fn live_db_m1570_hammond_path_accepts_and_lights_harriman() {
    let pool = require_db_or_skip!();
    let mut rig = ready(&pool, Some(SOLDIER)).await;
    press_pinned(&mut rig, HAMMOND).await;
    // Observed: the press also shows 5354 from chain 3002 (`step_status 1559
    // active`), which passes for a player who never held 1559. That chain is
    // owned elsewhere and is not edited here; the assertion is only that 5392
    // is among the shown dialogs.
    assert!(
        rig.sent().contains(&Sent::Dialog(D5392)),
        "Hammond shows 5392"
    );
    rig.choose(D5392, 0).await;
    assert_eq!(
        rig.sent(),
        vec![Sent::Dialog(D5395)],
        "accepting shows the follow-up 5395"
    );
    assert_eq!(rig.status(M1570), ACTIVE, "1570 is accepted");
    assert_eq!(rig.step(M1570), Some(S4641), "1570 starts at 4641");
    assert_eq!(rig.flags(HAMMOND) & OFFER, 0, "Hammond's '?' clears");
    assert_eq!(rig.flags(TEALC) & OFFER, 0, "Teal'c's '?' clears");
    assert!(
        (rig.flags(HARRIMAN) & STORY_ACTIVE) != 0,
        "Harriman shows '!'"
    );
}

/// **Revert proof:** delete chain 3125 (Teal'c's press shows nothing) or
/// chain 3127 (the accept from Teal'c never runs).
#[tokio::test]
async fn live_db_m1570_tealc_path_accepts_too() {
    let pool = require_db_or_skip!();
    let mut rig = ready(&pool, Some(SOLDIER)).await;
    press_pinned(&mut rig, TEALC).await;
    assert_eq!(rig.sent(), vec![Sent::Dialog(D5393)], "Teal'c shows 5393");
    rig.choose(D5393, 0).await;
    assert_eq!(
        rig.sent(),
        vec![Sent::Dialog(D5396)],
        "accepting shows the follow-up 5396"
    );
    assert_eq!(rig.status(M1570), ACTIVE, "1570 is accepted");
    assert_eq!(rig.step(M1570), Some(S4641), "1570 starts at 4641");
    assert_eq!(rig.flags(HAMMOND) & OFFER, 0, "Hammond's '?' clears");
    assert_eq!(rig.flags(TEALC) & OFFER, 0, "Teal'c's '?' clears");
}

/// **Revert proof:** delete chain 3128, or move its first `advance_step` (to
/// 4642) to delay 3000. The first assertion then fails, because 4641 stays
/// active until the window closes.
#[tokio::test]
async fn live_db_m1570_harriman_dials_then_crosses_after_three_seconds() {
    let pool = require_db_or_skip!();
    let mut rig = accepted_from_hammond(&pool).await;
    rig.press(HARRIMAN).await;
    assert_eq!(
        rig.step(M1570),
        Some(S4642),
        "the dial advances to 4642 at once"
    );
    assert_eq!(
        rig.sent(),
        vec![Sent::Sequence(DIAL_OPEN)],
        "10047 plays at once, no crossing yet"
    );
    rig.run_due(2500).await;
    assert_eq!(
        rig.sent(),
        vec![Sent::Sequence(DIAL_CROSS)],
        "10060 plays at 2500 ms"
    );
    assert_eq!(
        rig.step(M1570),
        Some(S4642),
        "the arrival step waits for 3000 ms"
    );
    rig.run_due(3000).await;
    assert_eq!(
        rig.sent(),
        vec![Sent::Travel(DAKARA_WORLD.to_string(), DAKARA_PIN)],
        "one crossing to the Dakara_E1 pin"
    );
    // Engine fact: the crossing sends GateTravel and destroys the cell entity,
    // so the 4906 advance that runs just before it cannot be read back here.
    assert_eq!(rig.step(M1570), None, "the crossing removes the player");
}

/// **Revert proof:** delete chain 3130 (no "!" after a relog on 4642) or
/// chain 3131 (the recovery press neither advances nor crosses). The relog
/// drops the deferred dial, as the engine does on disconnect.
#[tokio::test]
async fn live_db_m1570_interrupted_dial_recovers() {
    let pool = require_db_or_skip!();
    let mut rig = accepted_from_hammond(&pool).await;
    rig.press(HARRIMAN).await;
    let mut rig = relogged(rig, &pool).await;
    assert_eq!(rig.pending(), 0, "the relog drops the deferred dial");
    assert_eq!(
        rig.step(M1570),
        Some(S4642),
        "the dial stays at 4642 after the relog"
    );
    assert!(
        (rig.flags(HARRIMAN) & STORY_ACTIVE) != 0,
        "3130 relights Harriman on the relog"
    );
    rig.sent();
    rig.press(HARRIMAN).await;
    assert_eq!(
        rig.sent(),
        vec![Sent::Travel(DAKARA_WORLD.to_string(), DAKARA_PIN)],
        "one crossing to the Dakara_E1 pin"
    );
    // Engine fact: the recovery press advances and crosses at 0 ms, so the
    // player leaves the zone and the 4906 advance cannot be read back here.
    assert_eq!(rig.step(M1570), None, "the crossing removes the player");
}

/// **Characterisation of the F4 re-press risk (H-REPRESS, held: this test
/// records it and does not fix it).** The F4 design expected a second press
/// of Harriman inside the three-second window to fire recovery chain 3131 at
/// once (the step is already 4642) and then cross again at 3000 ms, so 2
/// Travel rows. The observed count is 1: the second press's immediate
/// crossing removes the player, and the first press's queued 3000 ms actions
/// do not run afterwards. The mechanism is inferred from the count and the
/// engine's disconnect behaviour, not traced. If the count moves, the design
/// or the engine changed: move this assertion with it. No revert proof: the
/// count is the observation.
#[tokio::test]
async fn live_db_m1570_repress_inside_the_window() {
    let pool = require_db_or_skip!();
    let mut rig = accepted_from_hammond(&pool).await;
    rig.press(HARRIMAN).await;
    rig.press(HARRIMAN).await;
    rig.run_due(3000).await;
    assert_eq!(
        travel_count(&mut rig),
        1,
        "observed: one crossing for two presses (the F4 design expected two)"
    );
}

/// **Revert proof:** drop the `archetype gte 1` row of chain 3123 and a
/// non-Human sees "?" after 1568 and 1569. Drop it from chain 3128 and a
/// non-Human who has taken 1570 dials and crosses on Harriman's press.
#[tokio::test]
async fn live_db_m1570_never_offered_to_a_non_human() {
    let pool = require_db_or_skip!();
    for archetype in NON_HUMANS.into_iter().map(Some).chain([UNSTAMPED]) {
        let mut rig = ready(&pool, archetype).await;
        assert_eq!(
            rig.flags(HAMMOND) & OFFER,
            0,
            "archetype {archetype:?}: no '?' on Hammond"
        );
        assert_eq!(
            rig.flags(TEALC) & OFFER,
            0,
            "archetype {archetype:?}: no '?' on Teal'c"
        );
        rig.accept(M1570).await;
        assert_eq!(rig.step(M1570), Some(S4641), "1570 starts at 4641");
        rig.press(HARRIMAN).await;
        assert_eq!(
            rig.step(M1570),
            Some(S4641),
            "archetype {archetype:?}: no dial"
        );
        assert_eq!(
            travel_count(&mut rig),
            0,
            "archetype {archetype:?}: no crossing"
        );
    }
}
