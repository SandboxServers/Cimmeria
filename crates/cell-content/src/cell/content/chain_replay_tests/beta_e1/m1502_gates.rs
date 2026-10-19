//! Gate and key negatives for the 1502 chains (UB-21), split from `m1502.rs` to
//! keep both files small. Every 1502 chain is checked against the whole seed:
//! the mission-state gates of the offer chains, the step gates of the box
//! buttons, and the world, mission, region and tag keys of the triggers.

use std::collections::BTreeSet;

use super::m1502::{event_ctx, m1502_matched, stage_offer, stage_step};
use super::{fire, login_ctx, whole_seed, with_mission, BETA, FREE_JAFFA};
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::triggers::TriggerType;

/// Revert: drop the `mission_status 1502 not_active` row of 23240, 23241 or
/// 23242. Once 1502 is `active` or `completed`, Ras'nor's offer must not be
/// bound, shown or accepted again, from a login, the 1500 completion, a
/// click or the 5041 choice.
#[tokio::test]
async fn live_db_offer_is_closed_once_1502_is_accepted_or_done() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for status in ["active", "completed"] {
        let staged = |mut ctx: cimmeria_content_engine::context::ExecutionContext| {
            with_mission(&mut ctx, 1500, "completed");
            with_mission(&mut ctx, 1502, status);
            ctx
        };
        let cases = [
            (
                TriggerType::PlayerLoaded,
                staged(login_ctx(BETA, Some(FREE_JAFFA))),
            ),
            (
                TriggerType::MissionCompleted,
                staged(event_ctx(
                    "mission_id",
                    serde_json::json!(1500),
                    Some(FREE_JAFFA),
                )),
            ),
            (
                TriggerType::InteractTag,
                staged(event_ctx(
                    "entity_tag",
                    serde_json::json!("BetaE1_RasNor"),
                    Some(FREE_JAFFA),
                )),
            ),
            (
                TriggerType::DialogChoice,
                staged(event_ctx(
                    "dialog_id",
                    serde_json::json!(5041),
                    Some(FREE_JAFFA),
                )),
            ),
        ];
        for (trigger, ctx) in cases {
            let label = format!("{trigger:?}");
            assert!(
                m1502_matched(&fire(&whole, trigger, &ctx)).is_empty(),
                "{label} with 1502 {status} must match no 1502 chain",
            );
        }
    }
}

/// Revert: drop the `step_status` row of 23246 or 23248, or let either fire at
/// another step. The first box button (5042) answers only at S4377 and the
/// second (5043) only at S4378: any other step matches nothing.
#[tokio::test]
async fn live_db_box_buttons_answer_only_at_their_own_step() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for (dialog, steps, own) in [
        (5042, [4376, 4378, 4481], 4377),
        (5043, [4376, 4377, 4481], 4378),
    ] {
        for step in steps {
            let ctx = stage_step(
                event_ctx("dialog_id", serde_json::json!(dialog), Some(FREE_JAFFA)),
                step,
                "active",
            );
            assert!(
                m1502_matched(&fire(&whole, TriggerType::DialogChoice, &ctx)).is_empty(),
                "button {dialog} at S{step} (its own step is S{own}) must match no 1502 chain",
            );
        }
        let ctx = stage_step(
            event_ctx("dialog_id", serde_json::json!(dialog), Some(FREE_JAFFA)),
            own,
            "completed",
        );
        assert!(
            m1502_matched(&fire(&whole, TriggerType::DialogChoice, &ctx)).is_empty(),
            "button {dialog} with S{own} completed (not active) must match no 1502 chain",
        );
    }
}

/// Revert: widen or NULL a trigger key: 23240 or 23249's world, 23240's
/// completed mission (1500), 23243's region, or an NPC or box tag. A Free
/// Jaffa in the offer or return state must still match nothing when the event
/// is another world's login, another mission's completion, another region or
/// another NPC's click.
#[tokio::test]
async fn live_db_trigger_keys_are_exact() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    let matched = |trigger, ctx| m1502_matched(&fire(&whole, trigger, &ctx));
    let none = BTreeSet::new();

    assert_eq!(
        matched(
            TriggerType::PlayerLoaded,
            stage_offer(login_ctx("Omega_Site", Some(FREE_JAFFA))),
        ),
        none,
        "an offer-state login on another world must bind nothing",
    );
    assert_eq!(
        matched(
            TriggerType::PlayerLoaded,
            stage_step(login_ctx("Omega_Site", Some(FREE_JAFFA)), 4481, "active"),
        ),
        none,
        "a return-state login on another world must bind nothing",
    );
    assert_eq!(
        matched(
            TriggerType::MissionCompleted,
            stage_offer(event_ctx(
                "mission_id",
                serde_json::json!(1501),
                Some(FREE_JAFFA)
            )),
        ),
        none,
        "completing another mission must not bind the offer",
    );
    assert_eq!(
        matched(
            TriggerType::RegionEnter,
            stage_step(
                event_ctx(
                    "region_key",
                    serde_json::json!("Beta_Site_Evo_1.FireSite"),
                    Some(FREE_JAFFA),
                ),
                4376,
                "active",
            ),
        ),
        none,
        "entering another region at S4376 must not reach the camp",
    );
    assert_eq!(
        matched(
            TriggerType::InteractTag,
            stage_step(
                event_ctx(
                    "entity_tag",
                    serde_json::json!("BetaE1_Carter"),
                    Some(FREE_JAFFA)
                ),
                4481,
                "active",
            ),
        ),
        none,
        "clicking Carter at S4481 must match no 1502 chain",
    );
}
