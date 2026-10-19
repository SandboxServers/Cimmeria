//! Mission 1651 SGC half, C2 arrival and C2b (US-30, chains 3133-3142). Hammond
//! on arrival, Carter's generator in the Gate Room, Harriman's dial and the
//! three-second crossing to Dakara_E1, and the relog and recovery chains.
//!
//! The crossing destroys the player in the cell (`transport.rs`), so the
//! mission state is unreadable after it. Tests check the queue before the
//! crossing runs and the `Travel` row after it.

use std::collections::HashMap;

use sqlx::PgPool;

use crate::cell::spawner::{load_step_objectives, WorldRow};

use cimmeria_content_engine::actions::Action;

use super::rig::{Rig, Sent, ACTIVE, FREE_JAFFA, PLAYER, SOLDIER, STORY_ACTIVE, WORLD, WORLD_ID};
use crate::test_support::require_db_or_skip;

const M1559: i32 = 1559;
const M1651: i32 = 1651;
const S4928: i32 = 4928;
const S4929: i32 = 4929;
const S4931: i32 = 4931;
const S5171: i32 = 5171;
const O5956: i32 = 5956;
const HAMMOND: &str = "SGCW1_GenHammond";
const CARTER: &str = "SGC_W1_SamCarter";
const HARRIMAN: &str = "SGC_W1_Harriman";
/// Carter in the Gate Room (P08, sealed in `docs/analysis/union-sgc-continuation/placement.md`).
const P08: [f32; 3] = [164.5, -10.8, 18.0];
/// The gate 25 pin for C2b (`docs/analysis/union-sgc-continuation/crossings.md`, C2b row).
const DAKARA_PIN: [f32; 3] = [96.87, -16.75, 243.23];
const DAKARA: &str = "Dakara_E1";
const DIALOG_HAMMOND: i32 = 5844;
const DIALOG_CARTER: i32 = 5845;
const GENERATOR: i32 = 6784;
/// The dial opens at 0 ms (10047) and crosses at 2500 ms (10060).
const SEQ_OPEN: i32 = 10047;
const SEQ_CROSS: i32 = 10060;

/// Whether the rig has sent a `display_dialog` for `dialog_id` since the last drain.
fn showed(rig: &mut Rig, dialog_id: i32) -> bool {
    rig.sent().contains(&Sent::Dialog(dialog_id))
}

/// How many rows of `sent` equal `wanted`.
fn count(sent: &[Sent], wanted: &Sent) -> usize {
    sent.iter().filter(|s| *s == wanted).count()
}

/// Whether any row of `sent` is a cross-world teleport.
fn travelled(sent: &[Sent]) -> bool {
    sent.iter().any(|s| matches!(s, Sent::Travel(..)))
}

/// Whether `a` and `b` are within `metres` of each other.
fn within(a: [f32; 3], b: [f32; 3], metres: f32) -> bool {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
        <= metres
}

/// The player's queued deferred actions, as (cross sequence, advance, crossing)
/// flags: whether the queue is exactly the cross sequence 10060, the advance to
/// 5171 and the crossing, in that order.
fn queue_is_cross_advance_crossing(rig: &Rig) -> bool {
    matches!(
        rig.mgr.pending_content_actions.get(&PLAYER).map(Vec::as_slice),
        Some([a, b, c])
            if matches!(a.action, Action::PlaySequence { sequence_id: SEQ_CROSS })
                && matches!(b.action, Action::AdvanceStep { mission_id: M1651, step_id: S5171 })
                && matches!(c.action, Action::CrossWorldTeleport { .. })
    )
}

/// Make the dispatchers read the player's world as a world other than
/// SGC_W1 (id `WORLD_ID + 1`), as a client in another world would be read.
fn leave_sgc_w1(rig: &mut Rig) {
    rig.mgr.stamp_world_rows(&HashMap::from([(
        WORLD.to_string(),
        WorldRow::enforcing(WORLD_ID + 1),
    )]));
}

/// The rig after a relog, with the step-objective roster loaded again: the
/// relog builds a fresh `Rig::new`, which leaves the roster empty.
async fn relogged(rig: Rig, pool: &PgPool) -> Rig {
    let mut rig = rig.relog(pool).await;
    rig.mgr.step_objectives = load_step_objectives(pool)
        .await
        .expect("load the step objectives");
    rig
}

/// A player standing at SGC_W1 on 4928 with the step roster loaded. Mission
/// 1559 is finished first: chain 3001 otherwise accepts it and lights
/// Hammond's "!" on every load, which would hide chain 3133.
async fn on_4928(pool: &PgPool, archetype: i32) -> Rig {
    let mut rig = Rig::new(pool, Some(archetype)).await;
    rig.mgr.step_objectives = load_step_objectives(pool)
        .await
        .expect("load the step objectives");
    rig.accept(M1559).await;
    rig.complete(M1559).await;
    rig.accept(M1651).await;
    rig.advance(M1651, S4928).await;
    rig.loaded().await;
    rig.sent();
    rig
}

/// `on_4928`, then Hammond's briefing and Carter's generator: 4931 holds on
/// O6037 with O5956 done.
async fn holding(pool: &PgPool, archetype: i32) -> Rig {
    let mut rig = on_4928(pool, archetype).await;
    rig.press(HAMMOND).await;
    rig.close(DIALOG_HAMMOND).await;
    rig.press(CARTER).await;
    rig.choose(DIALOG_CARTER, 0).await;
    rig
}

/// **Revert proof:** delete chain 3133 and Hammond shows no "!" on arrival.
/// Both Human and Free Jaffa arrive on 4928 and get the marker.
#[tokio::test]
async fn live_db_m1651_arrival_lights_hammond() {
    let pool = require_db_or_skip!();
    for archetype in [SOLDIER, FREE_JAFFA] {
        let rig = on_4928(&pool, archetype).await;
        assert!(
            (rig.flags(HAMMOND) & STORY_ACTIVE) != 0,
            "archetype {archetype}: Hammond shows '!' on 4928"
        );
    }
}

/// **Revert proof:** delete chain 3134 and 5844 is never shown; delete 3135
/// and closing 5844 does not move to 4931, bring Carter to P08 or light her "!".
#[tokio::test]
async fn live_db_m1651_hammond_sends_you_to_carter_in_the_gate_room() {
    let pool = require_db_or_skip!();
    for archetype in [SOLDIER, FREE_JAFFA] {
        let mut rig = on_4928(&pool, archetype).await;
        rig.press(HAMMOND).await;
        assert!(
            showed(&mut rig, DIALOG_HAMMOND),
            "archetype {archetype}: 5844 is shown"
        );
        rig.close(DIALOG_HAMMOND).await;
        assert_eq!(
            rig.step(M1651),
            Some(S4931),
            "archetype {archetype}: closing 5844 moves to 4931"
        );
        assert_eq!(
            rig.flags(HAMMOND) & STORY_ACTIVE,
            0,
            "archetype {archetype}: Hammond's '!' clears"
        );
        assert!(
            (rig.flags(CARTER) & STORY_ACTIVE) != 0,
            "archetype {archetype}: Carter shows '!'"
        );
        assert!(
            within(rig.pos(CARTER), P08, 0.5),
            "archetype {archetype}: Carter stands at P08, got {:?}",
            rig.pos(CARTER)
        );
    }
}

/// **Revert proof:** delete chain 3137 and no generator is granted. Replace
/// its `complete_objective 1651 '5956'` with `'6037'` and the objective
/// assertion fails (O5956 stays open; O6037 is completed instead). Drop the
/// `neq completed` row of 3137 and a second 5845 choice grants a second item.
#[tokio::test]
async fn live_db_m1651_generator_granted_once_and_the_step_holds() {
    let pool = require_db_or_skip!();
    for archetype in [SOLDIER, FREE_JAFFA] {
        let mut rig = holding(&pool, archetype).await;
        let first = rig.sent();
        assert_eq!(
            count(&first, &Sent::Item(GENERATOR, 2, 1)),
            1,
            "archetype {archetype}: one generator is granted"
        );
        assert_eq!(
            count(&first, &Sent::Dialog(DIALOG_CARTER)),
            1,
            "archetype {archetype}: 5845 is shown once"
        );
        rig.choose(DIALOG_CARTER, 0).await;
        assert_eq!(
            count(&rig.sent(), &Sent::Item(GENERATOR, 2, 1)),
            0,
            "archetype {archetype}: a second 5845 choice grants nothing"
        );
        assert!(
            rig.objective_done(M1651, O5956),
            "archetype {archetype}: O5956 is done"
        );
        assert_eq!(
            rig.step(M1651),
            Some(S4931),
            "archetype {archetype}: O6037 holds the step on 4931"
        );
        assert_eq!(
            rig.status(M1651),
            ACTIVE,
            "archetype {archetype}: the mission is not completed (F13)"
        );
        assert!(
            (rig.flags(HARRIMAN) & STORY_ACTIVE) != 0,
            "archetype {archetype}: Harriman shows '!'"
        );
        assert_eq!(
            rig.flags(CARTER) & STORY_ACTIVE,
            0,
            "archetype {archetype}: Carter's '!' clears"
        );
    }
}

/// **Revert proof:** drop the `objective_status` row of chain 3138 and
/// Harriman dials before the generator is held: the step jumps to 4929 and
/// the sequence fires.
#[tokio::test]
async fn live_db_m1651_harriman_waits_for_the_generator() {
    let pool = require_db_or_skip!();
    for archetype in [SOLDIER, FREE_JAFFA] {
        let mut rig = on_4928(&pool, archetype).await;
        rig.press(HAMMOND).await;
        rig.close(DIALOG_HAMMOND).await;
        rig.press(HARRIMAN).await;
        let sent = rig.sent();
        assert!(
            !sent.contains(&Sent::Sequence(SEQ_OPEN)),
            "archetype {archetype}: no dial before the generator"
        );
        assert!(
            !travelled(&sent),
            "archetype {archetype}: no crossing before the generator"
        );
        assert_eq!(
            rig.step(M1651),
            Some(S4931),
            "archetype {archetype}: Harriman does not move the step"
        );
    }
}

/// **Revert proof:** delete chain 3138, or move its first `advance_step` to
/// delay 3000, and the first assertion fails (the step does not reach 4929
/// at once). The queue after the press holds the cross sequence, the 5171
/// advance and the crossing; dropping any of them fails a queue count.
#[tokio::test]
async fn live_db_m1651_harriman_dials_then_crosses_after_three_seconds() {
    let pool = require_db_or_skip!();
    for archetype in [SOLDIER, FREE_JAFFA] {
        let mut rig = holding(&pool, archetype).await;
        rig.sent();
        rig.press(HARRIMAN).await;
        let now = rig.sent();
        assert_eq!(
            rig.step(M1651),
            Some(S4929),
            "archetype {archetype}: the press moves to 4929 at once"
        );
        assert!(
            now.contains(&Sent::Sequence(SEQ_OPEN)),
            "archetype {archetype}: the dial opens at once"
        );
        assert!(
            !travelled(&now),
            "archetype {archetype}: no crossing at once"
        );
        assert_eq!(
            rig.pending(),
            3,
            "archetype {archetype}: the cross sequence, the 5171 advance and the crossing are queued"
        );
        assert!(
            queue_is_cross_advance_crossing(&rig),
            "archetype {archetype}: the queue is 10060, the advance to 5171, then the crossing"
        );

        rig.run_due(2500).await;
        let at_2500 = rig.sent();
        assert!(
            at_2500.contains(&Sent::Sequence(SEQ_CROSS)),
            "archetype {archetype}: the cross sequence at 2500 ms"
        );
        assert!(
            !travelled(&at_2500),
            "archetype {archetype}: no crossing at 2500 ms"
        );
        assert_eq!(
            rig.pending(),
            2,
            "archetype {archetype}: the advance and the crossing still wait"
        );

        // The advance to 5171 runs first, then the crossing destroys the
        // player; 5171 is therefore checked through the queue above.
        rig.run_due(3000).await;
        assert_eq!(
            count(&rig.sent(), &Sent::Travel(DAKARA.to_string(), DAKARA_PIN)),
            1,
            "archetype {archetype}: one crossing to Dakara_E1 at the pin"
        );
    }
}

/// **Revert proof:** delete chain 3139, 3140, 3141 or 3142 and the marker or
/// the recovery is gone after a relog on the step that owns it. Case (c)'s
/// second press is chain 3142: it sends the crossing at once.
#[tokio::test]
async fn live_db_m1651_relogs_restore_the_right_marker() {
    let pool = require_db_or_skip!();
    for archetype in [SOLDIER, FREE_JAFFA] {
        // (a) 4931 before the generator: Carter back at P08 with "!".
        let mut rig = on_4928(&pool, archetype).await;
        rig.press(HAMMOND).await;
        rig.close(DIALOG_HAMMOND).await;
        let rig = relogged(rig, &pool).await;
        assert_eq!(
            rig.step(M1651),
            Some(S4931),
            "archetype {archetype}: a relog keeps 4931"
        );
        assert!(
            (rig.flags(CARTER) & STORY_ACTIVE) != 0,
            "archetype {archetype}: Carter's '!' comes back"
        );
        assert!(
            within(rig.pos(CARTER), P08, 0.5),
            "archetype {archetype}: Carter is back at P08"
        );

        // (b) 4931 with the generator: Harriman "!", Carter's cleared.
        let rig = relogged(holding(&pool, archetype).await, &pool).await;
        assert!(
            (rig.flags(HARRIMAN) & STORY_ACTIVE) != 0,
            "archetype {archetype}: Harriman's '!' comes back"
        );
        assert_eq!(
            rig.flags(CARTER) & STORY_ACTIVE,
            0,
            "archetype {archetype}: Carter's '!' stays clear"
        );

        // (c) 4929 with the dial interrupted: the queued crossing is lost,
        // Harriman shows "!", and one press finishes the crossing.
        let mut rig = holding(&pool, archetype).await;
        rig.press(HARRIMAN).await;
        let mut rig = relogged(rig, &pool).await;
        assert_eq!(
            rig.pending(),
            0,
            "archetype {archetype}: the deferred crossing does not survive a relog"
        );
        assert_eq!(
            rig.step(M1651),
            Some(S4929),
            "archetype {archetype}: a relog keeps 4929"
        );
        assert!(
            (rig.flags(HARRIMAN) & STORY_ACTIVE) != 0,
            "archetype {archetype}: Harriman shows '!' on 4929"
        );
        rig.sent();
        rig.press(HARRIMAN).await;
        assert_eq!(
            count(&rig.sent(), &Sent::Travel(DAKARA.to_string(), DAKARA_PIN)),
            1,
            "archetype {archetype}: the recovery press crosses once"
        );
    }
}

/// **Characterisation (F4 window, no revert):** a second press of Harriman
/// inside the three-second window does not add a crossing. The first press
/// queues the crossing; the second matches chain 3142 on 4929, advances to
/// 5171 and crosses at once. That crossing destroys the player, which scrubs
/// the queued crossing, so the window sends exactly one `Travel`.
#[tokio::test]
async fn live_db_m1651_repress_inside_the_window() {
    let pool = require_db_or_skip!();
    let mut rig = holding(&pool, SOLDIER).await;
    rig.sent();
    rig.press(HARRIMAN).await;
    assert!(
        !travelled(&rig.sent()),
        "the first press crosses only at 3000 ms"
    );
    rig.press(HARRIMAN).await;
    assert_eq!(
        count(&rig.sent(), &Sent::Travel(DAKARA.to_string(), DAKARA_PIN)),
        1,
        "the second press fires recovery chain 3142 and crosses at once"
    );
    rig.run_due(3000).await;
    assert!(
        !travelled(&rig.sent()),
        "the first press queued crossing never runs (F4 design expected a second)"
    );
}

/// **Revert proof:** drop the `world 58` row of chain 3134, 3135, 3136, 3137,
/// 3138 or 3142. Every press and choice here is a client event, so a replay in
/// another world must do nothing: no briefing, no step move, no generator, no
/// dial and no recovery crossing.
#[tokio::test]
async fn live_db_m1651_chains_only_fire_in_sgc_w1() {
    let pool = require_db_or_skip!();

    // 3134 and 3135: Hammond's briefing and its close.
    let mut rig = on_4928(&pool, SOLDIER).await;
    leave_sgc_w1(&mut rig);
    rig.press(HAMMOND).await;
    assert!(
        !showed(&mut rig, DIALOG_HAMMOND),
        "3134: no 5844 outside SGC_W1"
    );
    rig.close(DIALOG_HAMMOND).await;
    assert_eq!(
        rig.step(M1651),
        Some(S4928),
        "3135: the close does not advance"
    );

    // 3136 and 3137: Carter's briefing and the generator.
    let mut rig = on_4928(&pool, SOLDIER).await;
    rig.press(HAMMOND).await;
    rig.close(DIALOG_HAMMOND).await;
    rig.sent();
    leave_sgc_w1(&mut rig);
    rig.press(CARTER).await;
    assert!(
        !showed(&mut rig, DIALOG_CARTER),
        "3136: no 5845 outside SGC_W1"
    );
    rig.choose(DIALOG_CARTER, 0).await;
    assert_eq!(
        count(&rig.sent(), &Sent::Item(GENERATOR, 2, 1)),
        0,
        "3137: no generator outside SGC_W1"
    );
    assert!(!rig.objective_done(M1651, O5956), "3137: O5956 stays open");

    // 3138: the dial.
    let mut rig = holding(&pool, SOLDIER).await;
    rig.sent();
    leave_sgc_w1(&mut rig);
    rig.press(HARRIMAN).await;
    assert!(
        rig.sent().is_empty(),
        "3138: nothing is sent outside SGC_W1"
    );
    assert_eq!(rig.step(M1651), Some(S4931), "3138: no dial outside SGC_W1");
    assert_eq!(rig.pending(), 0, "3138: nothing is queued outside SGC_W1");

    // 3142: the recovery crossing.
    let mut rig = holding(&pool, SOLDIER).await;
    rig.press(HARRIMAN).await;
    rig.sent();
    leave_sgc_w1(&mut rig);
    rig.press(HARRIMAN).await;
    assert!(
        !travelled(&rig.sent()),
        "3142: no recovery crossing outside SGC_W1"
    );
    assert_eq!(rig.step(M1651), Some(S4929), "3142: the step does not move");
}
