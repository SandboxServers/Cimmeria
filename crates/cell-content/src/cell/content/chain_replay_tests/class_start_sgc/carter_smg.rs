//! M1562 "Locate Col. Carter" after CS-05: chains 3030-3037 take a Human
//! from the elevator (step 4625) through Carter's dialog 5367 (4626) to the
//! desk step (4627), where a press on the SMG lying on her desk grants the
//! SGHC 6 (item 21) once and completes the mission. The desk and the door
//! behaviour are read from the cooked map; the evidence is in the seed, on
//! spawn 81 and above chain 3030.
//!
//! For every other archetype M1562 stops at step 4625 with the lab doors
//! closed, Carter unmarked and the SMG unlit, as it did before CS-05 (the
//! Asgard holding state, OD-CS09).

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use cimmeria_content_engine::actions::Action;

use super::super::super::event_dispatch::{
    fire_dialog_choice, fire_interact_tag, fire_player_loaded,
};
use super::{
    drain, engine_with, give_mission, interaction_flags, label, mission_status, mission_step,
    sgc_mgr, spawn_tagged, Sent, ACTIVE, ASGARD, COMPLETED, HUMANS, MISSION_OBJECT, NON_HUMANS,
    PLAYER_EID, PLAYER_ID, STORY_ACTIVE,
};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::require_db_or_skip;

const CARTER_EID: u32 = 7102;
const BUTTON_EID: u32 = 7103;
const SMG_EID: u32 = 7104;
const CARTER: &str = "SGC_W1_SamCarter";
const BUTTON: &str = "SGC_W1_ElevatorButton2";
const SMG: &str = "SGC_W1_CarterDeskSMG";

/// The Kismet events of `CartersLabDoors`: 10009 closes, 10010 opens.
const OPEN_LAB_DOORS: i32 = 10010;

/// Every chain that listens on the M1562 route, old (3028) and new.
const ROUTE: [i32; 7] = [3028, 3030, 3032, 3033, 3034, 3035, 3036];

/// A player of `archetype` holding M1562 on `step`, with Carter, the desk
/// SMG and the second elevator button in the space.
fn on_step(archetype: Option<i32>, step: i32) -> SpaceManager {
    let mut mgr = sgc_mgr(archetype);
    spawn_tagged(&mut mgr, CARTER_EID, CARTER);
    spawn_tagged(&mut mgr, SMG_EID, SMG);
    spawn_tagged(&mut mgr, BUTTON_EID, BUTTON);
    give_mission(&mut mgr, 1562, step);
    mgr
}

fn carter_marked(mgr: &SpaceManager) -> bool {
    interaction_flags(mgr, CARTER_EID) & STORY_ACTIVE != 0
}

fn smg_lit(mgr: &SpaceManager) -> bool {
    interaction_flags(mgr, SMG_EID) & MISSION_OBJECT != 0
}

/// The delayed actions queued for the player and due within `within`, as
/// `(chain, action)`. Draining them stands in for the cell tick.
fn due_within(mgr: &mut SpaceManager, within: Duration) -> Vec<(i64, Action)> {
    mgr.take_ready_content_actions(Instant::now() + within)
        .into_iter()
        .map(|(_, pending)| (pending.chain_id, pending.action))
        .collect()
}

/// **Guard: a Human finishes M1562 at Carter's desk and receives SMG 21
/// exactly once.** Walks the whole route with the real dispatchers. Each
/// stage names the chain that owns it; remove that chain, or its step gate,
/// and the stage fails.
#[tokio::test]
async fn live_db_m1562_human_reaches_carter_and_takes_the_smg_from_her_desk_once() {
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

        // Chains 3028 + 3030: the elevator, Carter becomes the contact, and
        // the lab doors chain 3028 closes are re-opened once the 2 s close
        // has played: not at once, and within a few seconds.
        press!(BUTTON, BUTTON_EID);
        assert_eq!(mission_step(&mgr, 1562), Some(4625), "{who}: elevator");
        assert_eq!(drain(&mut rx), vec![Sent::Dialog(5366)], "{who}: elevator");
        assert!(carter_marked(&mgr), "{who}: chain 3030 marks Carter");
        assert_eq!(
            due_within(&mut mgr, Duration::from_millis(2_000)),
            vec![],
            "{who}: the doors are not re-opened while they are still closing",
        );
        assert_eq!(
            due_within(&mut mgr, Duration::from_secs(5)),
            vec![(
                3030,
                Action::PlaySequence {
                    sequence_id: OPEN_LAB_DOORS
                }
            )],
            "{who}: chain 3030 re-opens Carter's lab doors",
        );

        // Before the desk step the SMG is scenery: unlit, and a press on it
        // (the client would not send one) does nothing.
        assert!(!smg_lit(&mgr), "{who}: the SMG is unlit before step 4627");
        press!(SMG, SMG_EID);
        assert_eq!(drain(&mut rx), vec![], "{who}: no SMG before step 4627");

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

        // Chain 3033: closing 5367 opens the desk step and moves the marker
        // from Carter to the SMG on her desk.
        fire_dialog_choice(PLAYER_EID, PLAYER_ID, 5367, -1, &engine, &tx, &mut mgr).await;
        assert_eq!(mission_step(&mgr, 1562), Some(4627), "{who}: chain 3033");
        assert_eq!(drain(&mut rx), vec![], "{who}: chain 3033 sends no dialog");
        assert!(!carter_marked(&mgr), "{who}: chain 3033 unmarks Carter");
        assert!(smg_lit(&mgr), "{who}: chain 3033 lights the desk SMG");

        // Chain 3036: Carter on the desk step repeats her dialog, whose last
        // line points at the desk. No SMG, no step change.
        press!(CARTER, CARTER_EID);
        assert_eq!(
            drain(&mut rx),
            vec![Sent::Dialog(5367)],
            "{who}: chain 3036"
        );
        assert_eq!(mission_step(&mgr, 1562), Some(4627), "{who}: chain 3036");
        assert_eq!(mission_status(&mgr, 1562), ACTIVE, "{who}: chain 3036");
        // Closing that dialog must not run chain 3033 a second time.
        fire_dialog_choice(PLAYER_EID, PLAYER_ID, 5367, -1, &engine, &tx, &mut mgr).await;
        assert_eq!(mission_step(&mgr, 1562), Some(4627));
        assert!(smg_lit(&mgr), "{who}: the SMG stays lit");

        // Chain 3035: the SMG to the backpack, the pickup line, mission
        // complete, the SMG unlit.
        press!(SMG, SMG_EID);
        assert_eq!(
            drain(&mut rx),
            vec![Sent::Item(21, 1, 1), Sent::Dialog(5368)],
            "{who}: chain 3035 grants SMG 21 and shows 5368",
        );
        assert_eq!(mission_status(&mgr, 1562), COMPLETED, "{who}: chain 3035");
        assert!(!smg_lit(&mgr), "{who}: chain 3035 unlights the SMG");

        // No second SMG, from the desk or from Carter.
        press!(SMG, SMG_EID);
        press!(CARTER, CARTER_EID);
        assert_eq!(
            drain(&mut rx),
            vec![],
            "{who}: a second press grants nothing"
        );
        assert_eq!(due_within(&mut mgr, Duration::from_secs(60)), vec![]);
    }
}

/// **Guard: the Asgard holding state on M1562 (OD-CS09).** For an Asgard
/// and every other non-Human the elevator still works (chain 3028,
/// unchanged) and nothing else does: the lab doors are not re-opened,
/// Carter is never marked, the desk SMG is never lit, a press on either
/// does nothing, and no SMG is granted. M1562 stops at step 4625 as before
/// CS-05. Drop the `archetype lt 5` row from any of 3030-3036 and this
/// fails.
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

            for (tag, eid) in [(CARTER, CARTER_EID), (SMG, SMG_EID)] {
                fire_interact_tag(PLAYER_EID, PLAYER_ID, tag, eid, &engine, &tx, &mut mgr).await;
            }
            fire_dialog_choice(PLAYER_EID, PLAYER_ID, 5367, -1, &engine, &tx, &mut mgr).await;

            assert_eq!(
                drain(&mut rx),
                vec![],
                "archetype {archetype} from step {step}: neither Carter nor the SMG gives anything",
            );
            assert_eq!(mission_step(&mgr, 1562), step_before);
            assert_eq!(mission_status(&mgr, 1562), ACTIVE);
            assert!(
                !carter_marked(&mgr) && !smg_lit(&mgr),
                "archetype {archetype} from step {step}: nothing is marked",
            );
            assert_eq!(
                due_within(&mut mgr, Duration::from_secs(60)),
                vec![],
                "archetype {archetype} from step {step}: the lab doors are not re-opened",
            );
        }
    }
}

/// Chains 3031 and 3037: a Human who relogs past the elevator finds the
/// current step's marker again (interaction bits do not survive a relog):
/// Carter on steps 4625 and 4626, the desk SMG on 4627. Nothing before the
/// elevator, without the mission, once it is complete, or for the Asgard.
#[tokio::test]
async fn live_db_chains_3031_and_3037_restore_the_step_marker_on_relog_for_a_human() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[3031, 3037]).await;
    let human = Some(HUMANS[0]);
    let relog = |archetype: Option<i32>, step: Option<i32>, completed: bool| {
        let mut mgr = sgc_mgr(archetype);
        spawn_tagged(&mut mgr, CARTER_EID, CARTER);
        spawn_tagged(&mut mgr, SMG_EID, SMG);
        if let Some(step) = step {
            give_mission(&mut mgr, 1562, step);
        }
        if completed {
            mgr.get_entity_mut(PLAYER_EID)
                .unwrap()
                .missions
                .get_mission_mut(1562)
                .unwrap()
                .complete();
        }
        mgr
    };

    // (archetype, step, completed) -> (Carter marked, SMG lit).
    for (archetype, step, completed, expected) in [
        (human, Some(4624), false, (false, false)),
        (human, Some(4625), false, (true, false)),
        (human, Some(4626), false, (true, false)),
        (human, Some(4627), false, (false, true)),
        (human, None, false, (false, false)),
        // A completed mission has no active step: only chain 3031's
        // `mission_status 1562 eq active` row keeps Carter unmarked.
        (human, Some(4627), true, (false, false)),
        (None, Some(4625), false, (true, false)),
        (None, Some(4627), false, (false, true)),
        (Some(ASGARD), Some(4625), false, (false, false)),
        (Some(ASGARD), Some(4627), false, (false, false)),
    ] {
        let mut mgr = relog(archetype, step, completed);
        if completed {
            assert_eq!(mission_status(&mgr, 1562), COMPLETED);
        }
        let (tx, _rx) = mpsc::channel(64);

        fire_player_loaded(PLAYER_EID, PLAYER_ID, "SGC_W1", &engine, &tx, &mut mgr).await;

        assert_eq!(
            (carter_marked(&mgr), smg_lit(&mgr)),
            expected,
            "relog for {} with M1562 step {step:?}, completed {completed}: (Carter, SMG)",
            label(archetype),
        );
    }
}
