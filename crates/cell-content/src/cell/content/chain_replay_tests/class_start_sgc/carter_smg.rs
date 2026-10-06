//! M1562 "Locate Col. Carter" after CS-05: chains 3030-3035 take a Human
//! from the elevator (step 4625) through Carter's dialog 5367 (4626) to the
//! desk step (4627), where the SGHC 6 SMG (item 21) is granted once and the
//! mission completes. Carter is the click target for the desk step; see the
//! seed's own note on why.
//!
//! For every other archetype M1562 stops at step 4625 with Carter unmarked,
//! as it did before CS-05 (the Asgard holding state, OD-CS09).

use tokio::sync::mpsc;

use super::super::super::event_dispatch::{
    fire_dialog_choice, fire_interact_tag, fire_player_loaded,
};
use super::{
    drain, engine_with, give_mission, interaction_flags, label, mission_status, mission_step,
    sgc_mgr, spawn_tagged, Sent, ACTIVE, ASGARD, COMPLETED, HUMANS, NON_HUMANS, PLAYER_EID,
    PLAYER_ID, STORY_ACTIVE,
};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::require_db_or_skip;

const CARTER_EID: u32 = 7102;
const BUTTON_EID: u32 = 7103;
const CARTER: &str = "SGC_W1_SamCarter";
const BUTTON: &str = "SGC_W1_ElevatorButton2";

/// Every chain that listens on the M1562 route, old (3028) and new.
const ROUTE: [i32; 6] = [3028, 3030, 3032, 3033, 3034, 3035];

/// A player of `archetype` holding M1562 on `step`, with Carter and the
/// second elevator button in the space.
fn on_step(archetype: Option<i32>, step: i32) -> SpaceManager {
    let mut mgr = sgc_mgr(archetype);
    spawn_tagged(&mut mgr, CARTER_EID, CARTER);
    spawn_tagged(&mut mgr, BUTTON_EID, BUTTON);
    give_mission(&mut mgr, 1562, step);
    mgr
}

fn carter_marked(mgr: &SpaceManager) -> bool {
    interaction_flags(mgr, CARTER_EID) & STORY_ACTIVE != 0
}

/// **Guard: a Human finishes M1562 and receives SMG 21 exactly once.**
/// Walks the whole route with the real dispatchers. Each stage names the
/// chain that owns it; remove that chain, or its step gate, and the stage
/// fails.
#[tokio::test]
async fn live_db_m1562_human_reaches_carter_and_takes_the_smg_once() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ROUTE).await;

    for archetype in HUMANS.iter().map(|&a| Some(a)).chain([None]) {
        let who = label(archetype);
        let mut mgr = on_step(archetype, 4624);
        let (tx, mut rx) = mpsc::channel(64);
        macro_rules! press {
            ($tag:expr, $eid:expr) => {
                fire_interact_tag(PLAYER_EID, PLAYER_ID, $tag, $eid, &engine, &tx, &mut mgr).await
            };
        }

        // Chains 3028 + 3030: the elevator, and Carter becomes the contact.
        press!(BUTTON, BUTTON_EID);
        assert_eq!(mission_step(&mgr, 1562), Some(4625), "{who}: elevator");
        assert_eq!(drain(&mut rx), vec![Sent::Dialog(5366)], "{who}: elevator");
        assert!(carter_marked(&mgr), "{who}: chain 3030 marks Carter");

        // Chain 3032: reaching Carter opens her dialog on step 4626.
        press!(CARTER, CARTER_EID);
        assert_eq!(mission_step(&mgr, 1562), Some(4626), "{who}: chain 3032");
        assert_eq!(
            drain(&mut rx),
            vec![Sent::Dialog(5367)],
            "{who}: chain 3032"
        );

        // Chain 3034: a second press before the dialog closed shows it
        // again and grants nothing.
        press!(CARTER, CARTER_EID);
        assert_eq!(mission_step(&mgr, 1562), Some(4626), "{who}: chain 3034");
        assert_eq!(
            drain(&mut rx),
            vec![Sent::Dialog(5367)],
            "{who}: chain 3034"
        );

        // Chain 3033: closing 5367 opens the desk step.
        fire_dialog_choice(PLAYER_EID, PLAYER_ID, 5367, -1, &engine, &tx, &mut mgr).await;
        assert_eq!(mission_step(&mgr, 1562), Some(4627), "{who}: chain 3033");
        assert_eq!(drain(&mut rx), vec![], "{who}: chain 3033 sends no dialog");

        // Chain 3035: the SMG to the backpack, the pickup line, mission
        // complete, Carter unmarked.
        press!(CARTER, CARTER_EID);
        assert_eq!(
            drain(&mut rx),
            vec![Sent::Item(21, 1, 1), Sent::Dialog(5368)],
            "{who}: chain 3035 grants SMG 21 and shows 5368",
        );
        assert_eq!(mission_status(&mgr, 1562), COMPLETED, "{who}: chain 3035");
        assert!(!carter_marked(&mgr), "{who}: chain 3035 unmarks Carter");

        // No second SMG.
        press!(CARTER, CARTER_EID);
        assert_eq!(
            drain(&mut rx),
            vec![],
            "{who}: a second press grants nothing"
        );
    }
}

/// **Guard: the Asgard holding state on M1562 (OD-CS09).** For an Asgard
/// and every other non-Human the elevator still works (chain 3028,
/// unchanged), Carter is never marked, a press on her does nothing, and no
/// SMG is granted: M1562 stops at step 4625 as before CS-05. Drop the
/// `archetype lt 5` row from any of 3030-3035 and this fails.
#[tokio::test]
async fn live_db_m1562_still_stops_at_the_elevator_for_non_humans() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ROUTE).await;

    for archetype in NON_HUMANS {
        for step in [4624, 4625, 4626, 4627] {
            let mut mgr = on_step(Some(archetype), step);
            let (tx, mut rx) = mpsc::channel(64);

            if step == 4624 {
                fire_interact_tag(
                    PLAYER_EID, PLAYER_ID, BUTTON, BUTTON_EID, &engine, &tx, &mut mgr,
                )
                .await;
                assert_eq!(
                    drain(&mut rx),
                    vec![Sent::Dialog(5366)],
                    "archetype {archetype}: chain 3028 is unchanged",
                );
                assert_eq!(mission_step(&mgr, 1562), Some(4625));
            }
            let step_before = mission_step(&mgr, 1562);

            fire_interact_tag(
                PLAYER_EID, PLAYER_ID, CARTER, CARTER_EID, &engine, &tx, &mut mgr,
            )
            .await;
            fire_dialog_choice(PLAYER_EID, PLAYER_ID, 5367, -1, &engine, &tx, &mut mgr).await;

            assert_eq!(
                drain(&mut rx),
                vec![],
                "archetype {archetype} from step {step}: Carter gives nothing",
            );
            assert_eq!(mission_step(&mgr, 1562), step_before);
            assert_eq!(mission_status(&mgr, 1562), ACTIVE);
            assert!(
                !carter_marked(&mgr),
                "archetype {archetype} from step {step}: Carter is never marked",
            );
        }
    }
}

/// Chain 3031: a Human who relogs past the elevator finds Carter marked
/// again (interaction bits do not survive a relog). Not before the elevator,
/// not without the mission, not once it is complete, and not for the Asgard.
#[tokio::test]
async fn live_db_chain_3031_restores_carters_marker_on_relog_for_a_human_past_the_elevator() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[3031]).await;
    let human = Some(HUMANS[0]);

    for (archetype, step, expected) in [
        (human, Some(4624), false),
        (human, Some(4625), true),
        (human, Some(4626), true),
        (human, Some(4627), true),
        (human, None, false),
        (None, Some(4625), true),
        (Some(ASGARD), Some(4625), false),
    ] {
        let mut mgr = sgc_mgr(archetype);
        spawn_tagged(&mut mgr, CARTER_EID, CARTER);
        if let Some(step) = step {
            give_mission(&mut mgr, 1562, step);
        }
        let (tx, _rx) = mpsc::channel(64);

        fire_player_loaded(PLAYER_EID, PLAYER_ID, "SGC_W1", &engine, &tx, &mut mgr).await;

        assert_eq!(
            carter_marked(&mgr),
            expected,
            "chain 3031 for {} with M1562 step {step:?}",
            label(archetype),
        );
    }

    // A Human who finished M1562 and relogs: Carter stays unmarked. The gate
    // is `mission_status 1562 eq active`; a completed mission has no active
    // step, so the `step 4624 neq active` row alone would let it through.
    let mut mgr = sgc_mgr(human);
    spawn_tagged(&mut mgr, CARTER_EID, CARTER);
    give_mission(&mut mgr, 1562, 4627);
    mgr.get_entity_mut(PLAYER_EID)
        .unwrap()
        .missions
        .get_mission_mut(1562)
        .unwrap()
        .complete();
    assert_eq!(mission_status(&mgr, 1562), COMPLETED);
    let (tx, _rx) = mpsc::channel(64);

    fire_player_loaded(PLAYER_EID, PLAYER_ID, "SGC_W1", &engine, &tx, &mut mgr).await;

    assert!(
        !carter_marked(&mgr),
        "chain 3031 must not re-mark Carter once M1562 is complete",
    );
}
