//! Mission 1655 Human and crossing C4 (US-32, chains 3156-3165). Woolsey offers
//! the closure to Humans, Hammond's 5856 grants the letter 6786, Harriman's 5857
//! dials Omega Site for both races, and the relog and recovery chains cover a
//! disconnect inside the window.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ResolvedActions;

use crate::cell::content::execute_actions;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::spawner::load_step_objectives;
use crate::test_support::require_db_or_skip;

use super::rig::{
    Rig, Sent, ACTIVE, FREE_JAFFA, HUMANS, NON_HUMANS, OFFER, PLAYER, PLAYER_ID, SOLDIER,
    STORY_ACTIVE, UNSTAMPED,
};

const M1654: i32 = 1654;
const M1655: i32 = 1655;
const S4943: i32 = 4943;
const S4944: i32 = 4944;
const S4941: i32 = 4941;
const S4940: i32 = 4940;
/// Letter to Daniel Jackson (D-US11 default), container 2.
const LETTER: i32 = 6786;
const WOOLSEY: &str = "SGC_W1_Woolsey";
const HAMMOND: &str = "SGCW1_GenHammond";
const HARRIMAN: &str = "SGC_W1_Harriman";
/// Woolsey's template (spawned by US-31 chain 3152; the tests spawn him here).
const WOOLSEY_TEMPLATE: i32 = 47;
/// Woolsey's arrival point at P02.
const P02: [f32; 3] = [197.5, 1.311, 43.6];
/// The C4 arrival pin UO-00 publishes (omega-work-packets.md, Receiving: C4).
/// The heading 3.1409 is not an action parameter, so it is not carried here.
const OMEGA: &str = "Omega_Site";
const OMEGA_PIN: [f32; 3] = [34.70, -51.28, -142.00];
/// Woolsey's closure offer (accept on the final screen).
const D5854: i32 = 5854;
/// Hammond's orders (zero buttons).
const D5856: i32 = 5856;
/// Harriman's dial orders (zero buttons).
const D5857: i32 = 5857;
/// The visible dial, played at once on the 5857 close (D-US4 default).
const SEQ_DIAL: i32 = 10047;
/// The arrival half of the dial, played at 2500 ms.
const SEQ_ARRIVE: i32 = 10060;

/// Whether the rig has sent a `display_dialog` for `dialog_id` since the last drain.
fn showed(rig: &mut Rig, dialog_id: i32) -> bool {
    rig.sent().contains(&Sent::Dialog(dialog_id))
}

/// Count the `cross_world_teleport` sends in `sent`.
fn travels(sent: &[Sent]) -> usize {
    sent.iter()
        .filter(|s| matches!(s, Sent::Travel(..)))
        .count()
}

/// Run the deferred actions due within `within_ms` as `Rig::run_due` does, and
/// return the step the player held just before each cross and the crosses
/// sent. A cross destroys the cell entity and with it the step, so the step can
/// only be read before it. The rig's sender is private, so this uses its own
/// channel and keeps only the `GateTravel` rows.
async fn run_due_crossing(rig: &mut Rig, within_ms: u64) -> (Vec<Option<i32>>, Vec<Sent>) {
    let (tx, mut rx) = mpsc::channel(64);
    let now = Instant::now() + Duration::from_millis(within_ms);
    let mut steps = Vec::new();
    let mut crosses = Vec::new();
    for (entity_id, pending) in rig.mgr.take_ready_content_actions(now) {
        if matches!(pending.action, Action::CrossWorldTeleport { .. }) {
            steps.push(rig.step(M1655));
        }
        let resolved = ResolvedActions {
            actions: vec![(pending.chain_id, pending.action)],
            action_delays: vec![0],
            params: pending.params,
        };
        execute_actions(
            resolved,
            entity_id,
            pending.player_id,
            &tx,
            &mut rig.mgr,
            &rig.engine,
        )
        .await;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::GateTravel {
                target_world_name,
                position,
                ..
            } = msg
            {
                crosses.push(Sent::Travel(target_world_name, position));
            }
        }
    }
    (steps, crosses)
}

/// Spawn Woolsey at P02 through the real dispatcher. The rig's sender is
/// private, so the spawn gets its own channel; the spawn sends nothing back.
async fn spawn_woolsey(rig: &mut Rig) {
    let (tx, _rx) = mpsc::channel(16);
    let resolved = ResolvedActions {
        actions: vec![(
            0,
            Action::SpawnEntity {
                template_id: WOOLSEY_TEMPLATE,
                position: P02,
                heading: 0.0,
                tag: WOOLSEY.to_string(),
                is_stationary: None,
                aggression: None,
                allow_shared: None,
            },
        )],
        action_delays: vec![0],
        params: HashMap::new(),
    };
    execute_actions(resolved, PLAYER, PLAYER_ID, &tx, &mut rig.mgr, &rig.engine).await;
}

/// A fresh rig with the `step_objectives` cache loaded (production loads it at
/// startup; `Rig::new` leaves it empty).
async fn fresh(pool: &PgPool, archetype: Option<i32>) -> Rig {
    let mut rig = Rig::new(pool, archetype).await;
    rig.mgr.step_objectives = load_step_objectives(pool)
        .await
        .expect("load the step objectives");
    rig
}

/// A player who has finished 1654 with Woolsey spawned at P02.
async fn after_1654(pool: &PgPool, archetype: Option<i32>) -> Rig {
    let mut rig = fresh(pool, archetype).await;
    rig.accept(M1654).await;
    rig.complete(M1654).await;
    spawn_woolsey(&mut rig).await;
    rig.loaded().await;
    rig.sent();
    rig
}

/// A player who has accepted 1655 and is on 4944, with the player loaded.
async fn on_4944(pool: &PgPool, archetype: Option<i32>) -> Rig {
    let mut rig = fresh(pool, archetype).await;
    rig.accept(M1655).await;
    rig.advance(M1655, S4944).await;
    rig.loaded().await;
    rig.sent();
    rig
}

/// Log out and back in, then reload the step-objective cache: the relog
/// rebuilds the rig, and `loaded()` runs before the cache is back.
async fn relogged(rig: Rig, pool: &PgPool) -> Rig {
    let mut rig = rig.relog(pool).await;
    rig.mgr.step_objectives = load_step_objectives(pool)
        .await
        .expect("load the step objectives");
    rig
}

/// **Revert proof:** delete 3156 or 3157 and Woolsey never offers or accepts
/// 1655 for a Human. Each Human (archetypes 1 to 4) sees the closure on 5854,
/// accepts it, and the mission lands on 4943 with Hammond's "!" set.
#[tokio::test]
async fn live_db_m1655_woolsey_offers_to_a_human() {
    let pool = require_db_or_skip!();
    for archetype in HUMANS {
        let mut rig = after_1654(&pool, Some(archetype)).await;
        rig.press(WOOLSEY).await;
        assert!(
            showed(&mut rig, D5854),
            "archetype {archetype}: 5854 offered after 1654"
        );
        rig.choose(D5854, 0).await;
        assert_eq!(rig.status(M1655), ACTIVE, "archetype {archetype}: accepted");
        assert_eq!(
            rig.step(M1655),
            Some(S4943),
            "archetype {archetype}: on 4943"
        );
        assert_eq!(
            rig.flags(WOOLSEY) & OFFER,
            0,
            "archetype {archetype}: Woolsey's '?' is cleared"
        );
        assert!(
            rig.flags(HAMMOND) & STORY_ACTIVE != 0,
            "archetype {archetype}: Hammond shows '!'"
        );
    }
}

/// **Revert proof:** delete 3158 or 3159, or drop 3159's `step_status` row
/// (then a second close grants a second letter). A Soldier presses Hammond on
/// 4943, closes 5856 twice, and gets exactly one letter, lands on 4944 and
/// Harriman shows "!".
#[tokio::test]
async fn live_db_m1655_hammond_hands_over_the_letter_once() {
    let pool = require_db_or_skip!();
    let mut rig = fresh(&pool, Some(SOLDIER)).await;
    rig.accept(M1655).await;
    rig.loaded().await;
    rig.sent();
    rig.press(HAMMOND).await;
    assert!(showed(&mut rig, D5856), "5856 is shown on 4943");
    rig.close(D5856).await;
    rig.close(D5856).await;
    let sent = rig.sent();
    let letters = sent
        .iter()
        .filter(|s| **s == Sent::Item(LETTER, 2, 1))
        .count();
    assert_eq!(letters, 1, "exactly one letter 6786 in container 2");
    assert_eq!(rig.step(M1655), Some(S4944), "the close advances to 4944");
    assert!(
        rig.flags(HAMMOND) & STORY_ACTIVE == 0,
        "Hammond's '!' is cleared"
    );
    assert!(
        rig.flags(HARRIMAN) & STORY_ACTIVE != 0,
        "Harriman shows '!'"
    );
}

/// **Revert proof:** delete 3161, or move its first `advance_step` to delay
/// 3000 (the gate then stays open for the whole window), or add an archetype
/// row to 3160 (case b then fails). Both races get the dial: the 5857 close
/// advances to 4941 at once and plays 10047, 10060 arrives at 2500 ms, and the
/// cross to Omega Site with step 4940 comes at 3000 ms, exactly once.
#[tokio::test]
async fn live_db_m1655_harriman_dials_omega_then_crosses_after_three_seconds() {
    let pool = require_db_or_skip!();
    for (archetype, label) in [(SOLDIER, "Soldier"), (FREE_JAFFA, "Free Jaffa")] {
        let mut rig = on_4944(&pool, Some(archetype)).await;
        rig.press(HARRIMAN).await;
        assert!(showed(&mut rig, D5857), "{label}: 5857 is shown on 4944");
        rig.close(D5857).await;
        assert_eq!(
            rig.step(M1655),
            Some(S4941),
            "{label}: the advance to 4941 is at once"
        );
        let now = rig.sent();
        assert!(
            now.contains(&Sent::Sequence(SEQ_DIAL)),
            "{label}: the dial 10047 plays at once"
        );
        assert_eq!(travels(&now), 0, "{label}: no cross before 3 s");
        rig.run_due(2500).await;
        assert!(
            rig.sent().contains(&Sent::Sequence(SEQ_ARRIVE)),
            "{label}: the arrival 10060 plays at 2500 ms"
        );
        let (steps, crosses) = run_due_crossing(&mut rig, 3000).await;
        assert_eq!(
            steps,
            vec![Some(S4940)],
            "{label}: the advance to 4940 runs before the cross"
        );
        assert_eq!(
            crosses,
            vec![Sent::Travel(OMEGA.to_string(), OMEGA_PIN)],
            "{label}: one cross to Omega Site at the pin"
        );
    }
}

/// **Revert proof:** delete 3162, 3163, 3164 or 3165 and the marker or the
/// recovery press is lost after the matching relog. Hammond's "!" comes back
/// on 4943, Harriman's on 4944, and on 4941 a relog keeps the step, restores
/// Harriman's "!" and a press sends the player through without a second dial.
#[tokio::test]
async fn live_db_m1655_relogs_restore_the_right_marker() {
    let pool = require_db_or_skip!();

    let mut rig = fresh(&pool, Some(SOLDIER)).await;
    rig.accept(M1655).await;
    let rig = relogged(rig, &pool).await;
    assert!(
        rig.flags(HAMMOND) & STORY_ACTIVE != 0,
        "(a) a relog on 4943 restores Hammond's '!'"
    );

    let rig = on_4944(&pool, Some(SOLDIER)).await;
    let rig = relogged(rig, &pool).await;
    assert!(
        rig.flags(HARRIMAN) & STORY_ACTIVE != 0,
        "(b) a relog on 4944 restores Harriman's '!'"
    );

    let mut rig = on_4944(&pool, Some(SOLDIER)).await;
    rig.press(HARRIMAN).await;
    rig.close(D5857).await;
    let mut rig = relogged(rig, &pool).await;
    assert_eq!(
        rig.pending(),
        0,
        "(c) the deferred dial does not survive a relog"
    );
    assert_eq!(rig.step(M1655), Some(S4941), "(c) a relog keeps 4941");
    assert!(
        rig.flags(HARRIMAN) & STORY_ACTIVE != 0,
        "(c) a relog on 4941 restores Harriman's '!'"
    );
    rig.press(HARRIMAN).await;
    // The cross destroys the cell entity, so the step is not readable after it;
    // the entity being gone and one GateTravel show the recovery crossed.
    assert!(
        rig.mgr.get_entity(PLAYER).is_none(),
        "(c) the recovery press crosses the player out of SGC_W1"
    );
    assert_eq!(travels(&rig.sent()), 1, "(c) exactly one cross");
    assert!(
        !showed(&mut rig, D5857),
        "(c) no second 5857 dial on the recovery press"
    );
}

/// **Characterisation of the F4 re-press risk, not a regression guard.** A press
/// on Harriman inside the dial window matches recovery chain 3165 (step 4941),
/// which crosses at once. That cross destroys the cell entity, and
/// `destroy_entity` drops its deferred queue, so 3161's 10060, advance and
/// cross at 2500 and 3000 ms never run. The observed count is one `GateTravel`
/// (not two as the packet first guessed): the re-press cannot double-cross, but
/// the 10060 arrival sequence is lost. The count is asserted so a change to it
/// is seen.
#[tokio::test]
async fn live_db_m1655_repress_inside_the_window() {
    let pool = require_db_or_skip!();
    let mut rig = on_4944(&pool, Some(SOLDIER)).await;
    rig.press(HARRIMAN).await;
    rig.close(D5857).await;
    rig.press(HARRIMAN).await;
    rig.run_due(3000).await;
    assert_eq!(
        travels(&rig.sent()),
        1,
        "the re-press inside the window: 3165's cross only, 3161's deferred cross is dropped"
    );
}

/// **Revert proof:** drop the `archetype gte '1'` row of 3156 or 3158. A
/// non-Human (Asgard, Goa'uld, Free Jaffa, Loyalist Jaffa) or an unstamped
/// player gets no 5854 offer and no 5856 orders, and so no letter.
#[tokio::test]
async fn live_db_m1655_human_beats_ignore_other_archetypes() {
    let pool = require_db_or_skip!();
    let archetypes: Vec<Option<i32>> = NON_HUMANS
        .into_iter()
        .map(Some)
        .chain(std::iter::once(UNSTAMPED))
        .collect();
    for archetype in archetypes {
        let mut rig = after_1654(&pool, archetype).await;
        rig.press(WOOLSEY).await;
        assert!(!showed(&mut rig, D5854), "{archetype:?}: no closure offer");

        let mut rig = fresh(&pool, archetype).await;
        rig.accept(M1655).await;
        rig.loaded().await;
        rig.sent();
        rig.press(HAMMOND).await;
        let sent = rig.sent();
        assert!(
            !sent.contains(&Sent::Dialog(D5856)),
            "{archetype:?}: no Hammond orders"
        );
        assert!(
            !sent.contains(&Sent::Item(LETTER, 2, 1)),
            "{archetype:?}: no letter"
        );
    }
}
