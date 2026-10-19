//! Mission 1568 Gate Room (US-22, chains 3100-3107). Offer after 1564, the
//! Gate Control region step, the terminal press, the three Gate Room Jaffa,
//! the terminal backstop, the departure staging and the two relog chains.
//! Every test drives the real dispatcher through [`Rig`].

use sqlx::PgPool;

use super::rig::*;
use crate::cell::spawner::{load_monologue_dialog_ids, load_step_objectives};
use crate::test_support::require_db_or_skip;

/// Gate Room Jaffa spawn tags (world 58).
const GROOM_JAFFA: [&str; 3] = [
    "SGC_W1_GroomJaffa1",
    "SGC_W1_GroomJaffa2",
    "SGC_W1_GroomJaffa3",
];
/// The Gate Control terminal spawn tag.
const TERMINAL: &str = "SGC_W1_GateTerminal1";
/// Gate Control region key (point set 15801).
const GATE_CONTROL: &str = "SGC_W1.GateControl";
/// P14: Hammond, P15: the player, P16: Carter (placement.md).
const P14: [f32; 3] = [199.989, 1.311, 43.608];
const P15: [f32; 3] = [182.0, 1.311, 43.5];
const P16: [f32; 3] = [179.5, 1.311, 45.5];

/// Whether `actual` is within 0.5 on every axis of `expected`.
fn within(actual: [f32; 3], expected: [f32; 3]) -> bool {
    actual
        .iter()
        .zip(expected)
        .all(|(a, e)| (a - e).abs() <= 0.5)
}

/// A rig for `archetype` with the real step objectives and monologue ids. The
/// rig loads neither: `advance_step` would open 4638 and 4639 with nothing to
/// complete, and the player-spoken 5382 and 5384 would be dropped as NPC lines.
async fn rig_for(pool: &PgPool, archetype: Option<i32>) -> Rig {
    let mut rig = Rig::new(pool, archetype).await;
    rig.mgr.step_objectives = load_step_objectives(pool)
        .await
        .expect("step objectives load");
    rig.mgr.monologue_dialog_ids = load_monologue_dialog_ids(pool)
        .await
        .expect("monologue ids load");
    rig
}

/// A rig for `archetype` with 1564 finished, so 1568 is offered.
async fn after_1564(pool: &PgPool, archetype: Option<i32>) -> Rig {
    let mut rig = rig_for(pool, archetype).await;
    rig.accept(1564).await;
    rig.finish(1564).await;
    let _ = rig.sent();
    rig
}

/// Walk a Soldier to 4639: enter Gate Control and press the terminal.
async fn at_4639(pool: &PgPool) -> Rig {
    let mut rig = after_1564(pool, Some(SOLDIER)).await;
    rig.enter(GATE_CONTROL).await;
    rig.press(TERMINAL).await;
    rig
}

/// **Offered on 1564 and on load after it, Human only.** Chain 3100 (its
/// `mission_completed` and `player_loaded` rows) offers 1568 to every Human.
/// Revert proof: delete 3100, or drop its `player_loaded` trigger row (b).
#[tokio::test]
async fn live_db_m1568_offered_when_1564_completes_and_on_load() {
    let pool = require_db_or_skip!();
    for archetype in HUMANS {
        let rig = after_1564(&pool, Some(archetype)).await;
        assert_eq!(rig.status(1568), ACTIVE, "archetype {archetype} is offered");
        assert_eq!(rig.step(1568), Some(4637));
    }
    let mut rig = rig_for(&pool, Some(SOLDIER)).await;
    rig.accept(1564).await;
    rig.complete(1564).await;
    rig.loaded().await;
    assert_eq!(rig.status(1568), ACTIVE, "offered on load after 1564");
    assert_eq!(rig.step(1568), Some(4637));
}

/// **Gate Control advances to 4638 and lights the terminal.** Chain 3101.
/// Revert proof: delete 3101.
#[tokio::test]
async fn live_db_m1568_gate_control_lights_the_terminal() {
    let pool = require_db_or_skip!();
    let mut rig = after_1564(&pool, Some(SOLDIER)).await;
    rig.enter(GATE_CONTROL).await;
    assert_eq!(rig.step(1568), Some(4638));
    assert!(rig.sent().contains(&Sent::Dialog(5382)), "monologue 5382");
    assert!(rig.flags(TERMINAL) & MISSION_OBJECT != 0, "terminal glows");
}

/// **The terminal press shuts the gate, then the three Jaffa finish 1568.**
/// Chains 3102 and 3103. Revert proof: delete 3102 or 3103.
#[tokio::test]
async fn live_db_m1568_terminal_then_jaffa_complete_the_mission() {
    let pool = require_db_or_skip!();
    let mut rig = after_1564(&pool, Some(SOLDIER)).await;
    rig.enter(GATE_CONTROL).await;
    assert!(
        rig.press(TERMINAL).await,
        "the lit terminal answers a press"
    );
    assert!(rig.sent().contains(&Sent::Dialog(5383)), "shut-off 5383");
    assert_eq!(rig.flags(TERMINAL) & MISSION_OBJECT, 0, "glow cleared");
    assert_eq!(rig.step(1568), Some(4639));
    rig.kill(GROOM_JAFFA[0]).await;
    rig.kill(GROOM_JAFFA[1]).await;
    assert_eq!(rig.status(1568), ACTIVE, "two Jaffa are not enough");
    rig.kill(GROOM_JAFFA[2]).await;
    assert_eq!(
        rig.status(1568),
        COMPLETED,
        "the third Jaffa completes 1568"
    );
}

/// **Terminal backstop: the Gate Room is already clear.** Chain 3104 runs
/// after 3102 in the same event (chain-id order at equal priority) and
/// completes 5380. Revert proof: delete 3104, and this test fails.
#[tokio::test]
async fn live_db_m1568_terminal_backstop_when_the_room_is_already_clear() {
    let pool = require_db_or_skip!();
    let mut rig = after_1564(&pool, Some(SOLDIER)).await;
    for tag in GROOM_JAFFA {
        rig.kill(tag).await;
    }
    rig.enter(GATE_CONTROL).await;
    rig.press(TERMINAL).await;
    assert_eq!(rig.status(1568), COMPLETED, "the backstop completes 1568");
}

/// **Completion stages the departure.** Chain 3105. Revert proof: delete 3105.
#[tokio::test]
async fn live_db_m1568_completion_stages_the_departure() {
    let pool = require_db_or_skip!();
    let mut rig = at_4639(&pool).await;
    for tag in GROOM_JAFFA {
        rig.kill(tag).await;
    }
    assert_eq!(rig.status(1568), COMPLETED);
    assert!(within(rig.player_pos(), P15), "player at P15");
    assert!(within(rig.pos("SGC_W1_SamCarter"), P16), "Carter at P16");
    assert!(within(rig.pos("SGCW1_GenHammond"), P14), "Hammond at P14");
    assert!(rig.sent().contains(&Sent::Dialog(5384)), "departure 5384");
}

/// **Relog restores the glow and Carter's place.** Chains 3106 and 3107.
/// Revert proof: delete 3106 or 3107.
#[tokio::test]
async fn live_db_m1568_relog_restores_glow_and_carter() {
    let pool = require_db_or_skip!();
    let mut rig = after_1564(&pool, Some(SOLDIER)).await;
    rig.enter(GATE_CONTROL).await;
    let mut rig = rig.relog(&pool).await;
    rig.loaded().await;
    assert!(
        rig.flags(TERMINAL) & MISSION_OBJECT != 0,
        "glow restored on 4638"
    );

    let mut rig = rig_for(&pool, Some(SOLDIER)).await;
    rig.accept(1568).await;
    rig.complete(1568).await;
    let mut rig = rig.relog(&pool).await;
    rig.loaded().await;
    assert!(
        within(rig.pos("SGC_W1_SamCarter"), P16),
        "Carter back at P16"
    );
}

/// **Never offered to a non-Human.** Chain 3100 gates on `archetype gte 1`
/// and `lte 4`; the Free Jaffa, Asgard, Goa'uld, Loyalist Jaffa and an
/// unstamped player (archetype -1) never get 1568. Revert proof: drop the
/// `archetype gte '1'` row of 3100.
#[tokio::test]
async fn live_db_m1568_never_offered_to_a_non_human() {
    let pool = require_db_or_skip!();
    let archetypes = NON_HUMANS
        .iter()
        .map(|archetype| Some(*archetype))
        .chain(std::iter::once(UNSTAMPED));
    for archetype in archetypes {
        let mut rig = after_1564(&pool, archetype).await;
        rig.loaded().await;
        assert_eq!(
            rig.status(1568),
            None,
            "archetype {archetype:?} is not offered"
        );
    }
}
