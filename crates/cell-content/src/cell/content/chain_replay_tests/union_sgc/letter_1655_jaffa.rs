//! Mission 1655 Free Jaffa (US-33, chains 3166-3169). Woolsey's 5855 accepts
//! 1655 on the press, Hammond's 5875 sends a Free Jaffa to Harriman with no
//! letter, and the shared Harriman beat (US-32, 3160-3165) dials Omega Site.
//! The Free Jaffa archetype is `eq 7`; the Human gate is `gte 1` and `lte 4`.

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

use super::rig::{Rig, Sent, ACTIVE, FREE_JAFFA, HUMANS, OFFER, PLAYER, PLAYER_ID, STORY_ACTIVE};

pub(super) const M1654: i32 = 1654;
pub(super) const M1655: i32 = 1655;
pub(super) const S4943: i32 = 4943;
pub(super) const S4940: i32 = 4940;
pub(super) const LETTER: i32 = 6786;
pub(super) const WOOLSEY: &str = "SGC_W1_Woolsey";
pub(super) const HAMMOND: &str = "SGCW1_GenHammond";
pub(super) const HARRIMAN: &str = "SGC_W1_Harriman";
pub(super) const D5854: i32 = 5854;
pub(super) const D5855: i32 = 5855;
pub(super) const D5856: i32 = 5856;
pub(super) const D5857: i32 = 5857;
pub(super) const D5875: i32 = 5875;
pub(super) const OMEGA: &str = "Omega_Site";

const WOOLSEY_TEMPLATE: i32 = 47;
const P02: [f32; 3] = [197.5, 1.311, 43.6];

/// Whether the rig has sent a `display_dialog` for `dialog_id` since the last drain.
fn showed(rig: &mut Rig, dialog_id: i32) -> bool {
    rig.sent().contains(&Sent::Dialog(dialog_id))
}

/// Run the deferred actions due within `within_ms` as `Rig::run_due` does, and
/// return the step the player held just before each cross and the crosses
/// sent. A cross destroys the cell entity and with it the step, so the step can
/// only be read before it. The rig's sender is private, so this uses its own
/// channel and keeps only the `GateTravel` rows.
pub(super) async fn run_due_crossing(
    rig: &mut Rig,
    within_ms: u64,
) -> (Vec<Option<i32>>, Vec<Sent>) {
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

/// Spawn Woolsey at P02 with the "?" offer marker through the real dispatcher
/// (US-31 chain 3152 does the same). The rig's sender is private, so the spawn
/// gets its own channel; it sends nothing back.
pub(super) async fn spawn_woolsey(rig: &mut Rig) {
    let (tx, _rx) = mpsc::channel(16);
    let resolved = ResolvedActions {
        actions: vec![
            (
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
            ),
            (
                0,
                Action::SetInteractionType {
                    entity_tag: WOOLSEY.to_string(),
                    operation: "|".to_string(),
                    mask: OFFER,
                },
            ),
        ],
        action_delays: vec![0, 0],
        params: HashMap::new(),
    };
    execute_actions(resolved, PLAYER, PLAYER_ID, &tx, &mut rig.mgr, &rig.engine).await;
}

/// A fresh rig with the `step_objectives` cache loaded (production loads it at
/// startup; `Rig::new` leaves it empty).
pub(super) async fn fresh(pool: &PgPool, archetype: Option<i32>) -> Rig {
    let mut rig = Rig::new(pool, archetype).await;
    rig.mgr.step_objectives = load_step_objectives(pool)
        .await
        .expect("load the step objectives");
    rig
}

/// A player who has finished 1654 with Woolsey spawned at P02.
pub(super) async fn after_1654(pool: &PgPool, archetype: Option<i32>) -> Rig {
    let mut rig = fresh(pool, archetype).await;
    rig.accept(M1654).await;
    rig.complete(M1654).await;
    spawn_woolsey(&mut rig).await;
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

/// **Revert proof:** delete chain 3166 and Woolsey does not show 5855 to a
/// Free Jaffa. The press then leaves 1655 unaccepted, Woolsey's "?" stays and
/// Hammond gets no "!".
#[tokio::test]
async fn live_db_m1655_woolsey_accepts_for_a_free_jaffa_on_press() {
    let pool = require_db_or_skip!();
    let mut rig = after_1654(&pool, Some(FREE_JAFFA)).await;
    rig.press(WOOLSEY).await;
    assert!(showed(&mut rig, D5855), "the Free Jaffa is shown 5855");
    assert_eq!(rig.status(M1655), ACTIVE, "the press accepts 1655");
    assert_eq!(rig.step(M1655), Some(S4943), "accepted onto 4943");
    assert_eq!(
        rig.flags(WOOLSEY) & OFFER,
        0,
        "Woolsey's '?' is cleared by the press"
    );
    assert!(
        rig.flags(HAMMOND) & STORY_ACTIVE != 0,
        "Hammond shows '!' once the press accepts"
    );
}

/// **Revert proof:** delete chain 3167 or 3168 and the route stops. Without
/// 3167 no 5875 is shown; without 3168 closing 5875 never reaches 4944, so
/// Harriman's 5857 and the dial never happen. Either way the Free Jaffa does
/// not reach Omega. The route also gets no letter item and no 5854 or 5856.
#[tokio::test]
async fn live_db_m1655_jaffa_path_reaches_omega_without_a_letter() {
    let pool = require_db_or_skip!();
    let mut rig = after_1654(&pool, Some(FREE_JAFFA)).await;
    rig.press(WOOLSEY).await;
    rig.press(HAMMOND).await;
    rig.close(D5875).await;
    rig.press(HARRIMAN).await;
    rig.close(D5857).await;
    let (steps, crosses) = run_due_crossing(&mut rig, 3000).await;
    let sent = rig.sent();

    assert!(sent.contains(&Sent::Dialog(D5855)), "5855 is shown");
    assert!(sent.contains(&Sent::Dialog(D5875)), "5875 is shown");
    assert!(sent.contains(&Sent::Dialog(D5857)), "5857 is shown");
    assert!(
        !sent.contains(&Sent::Dialog(D5854)) && !sent.contains(&Sent::Dialog(D5856)),
        "a Free Jaffa is never offered 5854 or given 5856"
    );
    assert!(
        !sent.iter().any(|s| matches!(s, Sent::Item(LETTER, _, _))),
        "a Free Jaffa gets no letter item 6786"
    );
    assert_eq!(steps, vec![Some(S4940)], "the cross happens on 4940");
    assert!(
        matches!(&crosses[..], [Sent::Travel(w, _)] if w == OMEGA),
        "one cross to Omega_Site, got {crosses:?}"
    );
}

/// **Revert proof:** delete chain 3169 and a Free Jaffa who relogs on 4943 has
/// no Hammond "!". Before the relog the quiet accept leaves Hammond unmarked,
/// so the relog is the only thing that sets it.
#[tokio::test]
async fn live_db_m1655_jaffa_relog_on_4943_relights_hammond() {
    let pool = require_db_or_skip!();
    let mut rig = fresh(&pool, Some(FREE_JAFFA)).await;
    rig.accept(M1655).await;
    assert_eq!(
        rig.flags(HAMMOND) & STORY_ACTIVE,
        0,
        "the quiet accept leaves Hammond unmarked"
    );
    let mut rig = relogged(rig, &pool).await;
    rig.sent();
    assert!(
        rig.flags(HAMMOND) & STORY_ACTIVE != 0,
        "the relog on 4943 relights Hammond's '!'"
    );
}

/// **Revert proof:** drop the `archetype` row of chain 3166 (Humans then get
/// the Jaffa dismissal) or of chain 3167 (a Human at 4943 gets 5875 instead of
/// 5856). Each Human sees neither dialog.
#[tokio::test]
async fn live_db_m1655_jaffa_beats_ignore_humans() {
    let pool = require_db_or_skip!();
    for archetype in HUMANS {
        let mut rig = after_1654(&pool, Some(archetype)).await;
        rig.press(WOOLSEY).await;
        let sent = rig.sent();
        assert!(
            !sent.contains(&Sent::Dialog(D5855)),
            "archetype {archetype}: no Jaffa dismissal"
        );

        let mut rig = fresh(&pool, Some(archetype)).await;
        rig.accept(M1655).await;
        rig.loaded().await;
        rig.sent();
        rig.press(HAMMOND).await;
        let sent = rig.sent();
        assert!(
            !sent.contains(&Sent::Dialog(D5875)),
            "archetype {archetype}: no Jaffa message for Bra'tac"
        );
    }
}
