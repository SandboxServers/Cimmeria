//! Beta Site E1 1426 Amateur Photography (UB-15): once 1425 reaches S4244 (or is
//! completed), Carter offers 1426 (dialog 4658, the blurb bind 5460). The accept
//! takes 1426 and grants the camera 4832 (HELD-B9). The player photographs the
//! Four Pieces case (S4245), uploads the photo at the hub computer (S4246, dialog
//! 4659, button 458), which takes the camera back, and reports to Carter (S4247,
//! dialog 4660). Chains 23160-23171, seeded in
//! `db/resources/Content/Seed/beta_e1_m1426_chains.sql`. Other races and unstamped
//! players must match none of them.

use std::collections::BTreeSet;

use super::super::assert_no_deferred_actions;
use super::{
    chains_matched, fire, login_ctx, summarized, whole_seed, with_mission, with_step,
    ALL_ARCHETYPES, BETA, HUMANS,
};
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;

const OFFER_LOGIN_S4244: i64 = 23160;
const OFFER_LOGIN_DONE: i64 = 23161;
const OFFER_IN_SESSION: i64 = 23162;
const CARTER_SHOW_S4244: i64 = 23163;
const CARTER_SHOW_DONE: i64 = 23164;
const ACCEPT_S4244: i64 = 23165;
const ACCEPT_DONE: i64 = 23166;
const CASE_PHOTO: i64 = 23167;
const HUB_CLICK: i64 = 23168;
const HUB_UPLOAD: i64 = 23169;
const RESTORE_TURN_IN: i64 = 23170;
const TURN_IN: i64 = 23171;
/// The 1425 science login hack (dialog 5497, button 457), which also fires 23136.
const SCIENCE_BUTTON_1425: i64 = 23136;
/// The 1425 hub computer click (S4242), which shares tag `BetaE1_HubComputer`.
const HUB_CLICK_1425: i64 = 23133;

const M1425: i32 = 1425;
const M1426: i32 = 1426;
const CARTER: &str = "BetaE1_Carter";
const CASE: &str = "BetaE1_FourPiecesCase";
const HUB: &str = "BetaE1_HubComputer";
const OFFER_DIALOG: i64 = 4658;
const UPLOAD_DIALOG: i64 = 4659;

/// The 1426 chains (23160-23171) that `resolved` matched.
fn matched_1426(resolved: &ResolvedActions) -> BTreeSet<i64> {
    chains_matched(resolved)
        .into_iter()
        .filter(|id| (OFFER_LOGIN_S4244..=TURN_IN).contains(id))
        .collect()
}

/// Every chain of 1425 and 1426 (23130-23171) that `resolved` matched.
fn matched_all(resolved: &ResolvedActions) -> BTreeSet<i64> {
    chains_matched(resolved)
        .into_iter()
        .filter(|id| (23130..=TURN_IN).contains(id))
        .collect()
}

/// The state of both missions a chain reads: each one's status and, when given,
/// its active step.
#[derive(Clone, Copy)]
struct Missions {
    m1425: (&'static str, Option<i32>),
    m1426: (&'static str, Option<i32>),
}

/// 1425 at S4242 (the hub step), 1426 not started.
const HUB_1425: Missions = Missions {
    m1425: ("active", Some(4242)),
    m1426: ("not_active", None),
};
/// 1425 at S4243 (the science step), 1426 not started.
const SCIENCE_1425: Missions = Missions {
    m1425: ("active", Some(4243)),
    m1426: ("not_active", None),
};
/// 1425 at S4244 (Jackson), 1426 not started: the offer is open.
const DELIVER_1425: Missions = Missions {
    m1425: ("active", Some(4244)),
    m1426: ("not_active", None),
};
/// 1425 completed (D-UB5 B or C), 1426 not started: the offer is open.
const DONE_1425: Missions = Missions {
    m1425: ("completed", None),
    m1426: ("not_active", None),
};
/// Neither mission started.
const NOT_ACTIVE: Missions = Missions {
    m1425: ("not_active", None),
    m1426: ("not_active", None),
};
/// 1426 completed, 1425 completed.
const FINISHED: Missions = Missions {
    m1425: ("completed", None),
    m1426: ("completed", None),
};

/// 1426 at `step`, with 1425 still at S4244 (the offer was accepted there).
fn photo_stage(step: i32) -> Missions {
    Missions {
        m1425: ("active", Some(4244)),
        m1426: ("active", Some(step)),
    }
}

/// Set both missions the way `populate_mission_context` does.
fn with_missions(mut ctx: ExecutionContext, m: Missions) -> ExecutionContext {
    with_mission(&mut ctx, M1425, m.m1425.0);
    if let Some(step) = m.m1425.1 {
        with_step(&mut ctx, M1425, step, "active");
    }
    with_mission(&mut ctx, M1426, m.m1426.0);
    if let Some(step) = m.m1426.1 {
        with_step(&mut ctx, M1426, step, "active");
    }
    ctx
}

/// A context with `param` set to `value`, the archetype stamped when given, and
/// both missions in state `m`.
fn event(
    param: &str,
    value: serde_json::Value,
    archetype: Option<i64>,
    m: Missions,
) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param(param.to_string(), value);
    if let Some(a) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(a));
    }
    with_missions(ctx, m)
}

/// A `player_loaded` into Beta, the archetype stamped when given, in state `m`.
fn login(archetype: Option<i64>, m: Missions) -> ExecutionContext {
    with_missions(login_ctx(BETA, archetype), m)
}

/// Every archetype outside the Human gate, plus an unstamped player (`None`).
fn non_human_or_unstamped() -> Vec<Option<i64>> {
    ALL_ARCHETYPES
        .iter()
        .copied()
        .filter(|a| !HUMANS.contains(a))
        .map(Some)
        .chain(std::iter::once(None))
        .collect()
}

/// Pass the context to `fire` and assert that `chain` matched alone among the
/// 1426 chains, with exactly `list`.
fn assert_chain(
    engine: &ChainEngine,
    trigger: TriggerType,
    ctx: &ExecutionContext,
    chain: i64,
    list: &[&str],
) {
    let resolved = fire(engine, trigger, ctx);
    assert_eq!(matched_1426(&resolved), BTreeSet::from([chain]));
    assert_eq!(
        summarized(&resolved, chain),
        list.iter().map(|s| s.to_string()).collect::<Vec<_>>()
    );
    assert_no_deferred_actions(&resolved, chain);
}

/// Revert: drop a `mission_completed` trigger row of 23161 (the bind then waits
/// for the next login), or the `add_dialog_set` row of 23160 (no blurb). A Human
/// at 1425 S4244 or with 1425 done binds Carter's blurb 5460 on login, the done
/// path also on the finish of 1425, and the science hack (S4243) binds it in
/// session. Other races, unstamped players, another world and the photo stage
/// match none of these.
#[tokio::test]
async fn offer_binds_cover_s4244_completion_and_in_session() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    let bind = ["add_dialog_set(5460, slot=2300)"];
    for archetype in HUMANS {
        let a = Some(archetype);
        assert_chain(
            &whole,
            TriggerType::PlayerLoaded,
            &login(a, DELIVER_1425),
            OFFER_LOGIN_S4244,
            &bind,
        );
        assert_chain(
            &whole,
            TriggerType::PlayerLoaded,
            &login(a, DONE_1425),
            OFFER_LOGIN_DONE,
            &bind,
        );
        // 23161 has two trigger rows, so the whole seed holds both paths.
        let finished = event("mission_id", serde_json::json!(M1425), a, DONE_1425);
        assert_chain(
            &whole,
            TriggerType::MissionCompleted,
            &finished,
            OFFER_LOGIN_DONE,
            &bind,
        );
        // The science hack also fires 23136 (1425 to S4244), so check the set.
        let hack = event("dialog_id", serde_json::json!(5497), a, SCIENCE_1425);
        let resolved = fire(&whole, TriggerType::DialogChoice, &hack);
        assert_eq!(
            matched_all(&resolved),
            BTreeSet::from([SCIENCE_BUTTON_1425, OFFER_IN_SESSION])
        );
        assert_eq!(summarized(&resolved, OFFER_IN_SESSION), bind);
    }
    for archetype in non_human_or_unstamped() {
        let logins = [
            login(archetype, DELIVER_1425),
            login(archetype, DONE_1425),
            // The wrong world must match nothing either.
            with_missions(login_ctx("Some_Other_World", archetype), DELIVER_1425),
            // A login at 1425 S4244 with 1426 already at the photo step matches nothing.
            login(archetype, photo_stage(4245)),
        ];
        for ctx in &logins {
            let resolved = fire(&whole, TriggerType::PlayerLoaded, ctx);
            assert!(
                matched_1426(&resolved).is_empty(),
                "archetype {archetype:?} must match no 1426 offer bind on login"
            );
        }
        let finished = event("mission_id", serde_json::json!(M1425), archetype, DONE_1425);
        let resolved = fire(&whole, TriggerType::MissionCompleted, &finished);
        assert!(matched_1426(&resolved).is_empty());
        let hack = event(
            "dialog_id",
            serde_json::json!(5497),
            archetype,
            SCIENCE_1425,
        );
        let resolved = fire(&whole, TriggerType::DialogChoice, &hack);
        assert!(matched_1426(&resolved).is_empty());
    }
}

/// Revert: drop the `step_status 1425 4244` row of 23163 (the offer then shows
/// at S4242) or the `mission_status 1426 not_active` row (the offer then shows
/// after the accept). 23163 and 23164 show Carter's offer 4658, exactly one at a
/// time, and the accepts (23165 at S4244, 23166 on the 1425 done path) take 1426,
/// remove the blurb bind and grant the camera 4832 as the last action.
#[tokio::test]
async fn carter_offer_shows_and_accept_takes_1426() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    let accept = [
        "accept_mission(1426)",
        "remove_dialog_set(5460, slot=2300)",
        "add_item(4832, 1)",
    ];
    for archetype in HUMANS {
        let a = Some(archetype);
        let show = ["display_dialog(4658)"];
        let click = event("entity_tag", serde_json::json!(CARTER), a, DELIVER_1425);
        assert_chain(
            &whole,
            TriggerType::InteractTag,
            &click,
            CARTER_SHOW_S4244,
            &show,
        );
        let click = event("entity_tag", serde_json::json!(CARTER), a, DONE_1425);
        assert_chain(
            &whole,
            TriggerType::InteractTag,
            &click,
            CARTER_SHOW_DONE,
            &show,
        );
        let choice = event(
            "dialog_id",
            serde_json::json!(OFFER_DIALOG),
            a,
            DELIVER_1425,
        );
        assert_chain(
            &whole,
            TriggerType::DialogChoice,
            &choice,
            ACCEPT_S4244,
            &accept,
        );
        let choice = event("dialog_id", serde_json::json!(OFFER_DIALOG), a, DONE_1425);
        assert_chain(
            &whole,
            TriggerType::DialogChoice,
            &choice,
            ACCEPT_DONE,
            &accept,
        );
    }
    for archetype in non_human_or_unstamped() {
        for m in [DELIVER_1425, DONE_1425] {
            let click = event("entity_tag", serde_json::json!(CARTER), archetype, m);
            let resolved = fire(&whole, TriggerType::InteractTag, &click);
            assert!(matched_1426(&resolved).is_empty());
            let choice = event("dialog_id", serde_json::json!(OFFER_DIALOG), archetype, m);
            let resolved = fire(&whole, TriggerType::DialogChoice, &choice);
            assert!(matched_1426(&resolved).is_empty());
        }
    }
}

/// Revert: remove the `step_status 1425 4244` row of 23165 (an accept at S4242 or
/// S4243 would then take 1426), or the `mission_status 1425 completed` row of
/// 23166 (an accept at S4242 or S4243 would then take 1426 too). A `pick:4658`
/// with 1425 at S4242 or S4243, or with neither mission started, matches neither
/// accept.
#[tokio::test]
async fn accept_needs_1425_at_s4244_or_done() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        for m in [HUB_1425, SCIENCE_1425, NOT_ACTIVE] {
            let choice = event(
                "dialog_id",
                serde_json::json!(OFFER_DIALOG),
                Some(archetype),
                m,
            );
            let resolved = fire(&whole, TriggerType::DialogChoice, &choice);
            assert!(
                matched_1426(&resolved).is_empty(),
                "archetype {archetype} at {:?} must not accept 1426",
                m.m1425
            );
        }
    }
    for archetype in non_human_or_unstamped() {
        let choice = event(
            "dialog_id",
            serde_json::json!(OFFER_DIALOG),
            archetype,
            DELIVER_1425,
        );
        let resolved = fire(&whole, TriggerType::DialogChoice, &choice);
        assert!(
            matched_1426(&resolved).is_empty(),
            "archetype {archetype:?} must not accept 1426"
        );
    }
}

/// Revert: drop the `step_status 1426 4246` row of 23168 (the hub then shows the
/// upload at S4245), or the `step_status 1425 4242` row of 23133 (the hub then
/// shows the search at S4246). With 1425 at S4242, the hub shows only 23133. At
/// the upload step (1426 S4246), it shows only 23168. Non-Humans match neither.
#[tokio::test]
async fn hub_computer_one_chain_per_step() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let a = Some(archetype);
        let click = event("entity_tag", serde_json::json!(HUB), a, HUB_1425);
        let resolved = fire(&whole, TriggerType::InteractTag, &click);
        assert_eq!(matched_all(&resolved), BTreeSet::from([HUB_CLICK_1425]));
        let click = event("entity_tag", serde_json::json!(HUB), a, photo_stage(4246));
        let resolved = fire(&whole, TriggerType::InteractTag, &click);
        assert_eq!(matched_all(&resolved), BTreeSet::from([HUB_CLICK]));
        assert_eq!(summarized(&resolved, HUB_CLICK), ["display_dialog(4659)"]);
        // The case step is not the hub: no hub chain matches at S4245.
        let click = event("entity_tag", serde_json::json!(HUB), a, photo_stage(4245));
        let resolved = fire(&whole, TriggerType::InteractTag, &click);
        assert!(matched_all(&resolved).is_empty());
    }
    for archetype in non_human_or_unstamped() {
        for m in [HUB_1425, photo_stage(4246)] {
            let click = event("entity_tag", serde_json::json!(HUB), archetype, m);
            let resolved = fire(&whole, TriggerType::InteractTag, &click);
            assert!(
                matched_all(&resolved).is_empty(),
                "archetype {archetype:?} must match no hub chain"
            );
        }
    }
}

/// Revert: drop the `step_status 1426 4245` row of 23167 (the case then advances
/// from any step), or the `advance_step` target (the photo then skips the upload).
/// The case click at S4245 advances 1426 to S4246 and nothing else. Before the
/// photo, or after it, the case matches nothing.
#[tokio::test]
async fn case_photo_advances_one_step() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let click = event(
            "entity_tag",
            serde_json::json!(CASE),
            Some(archetype),
            photo_stage(4245),
        );
        let resolved = fire(&whole, TriggerType::InteractTag, &click);
        assert_eq!(matched_all(&resolved), BTreeSet::from([CASE_PHOTO]));
        assert_eq!(
            summarized(&resolved, CASE_PHOTO),
            ["advance_step(1426, 4246)"]
        );
        assert_no_deferred_actions(&resolved, CASE_PHOTO);
        for m in [photo_stage(4246), photo_stage(4247), DELIVER_1425] {
            let click = event("entity_tag", serde_json::json!(CASE), Some(archetype), m);
            let resolved = fire(&whole, TriggerType::InteractTag, &click);
            assert!(matched_1426(&resolved).is_empty());
        }
    }
    for archetype in non_human_or_unstamped() {
        let click = event(
            "entity_tag",
            serde_json::json!(CASE),
            archetype,
            photo_stage(4245),
        );
        let resolved = fire(&whole, TriggerType::InteractTag, &click);
        assert!(matched_1426(&resolved).is_empty());
    }
}

/// Revert: drop the `remove_item` row of 23169 (the camera then stays in the
/// bag), or the `add_dialog_set` row of 23170 (no turn-in bind after a relog).
/// The upload (dialog 4659, S4246) advances to S4247, binds Carter's turn-in
/// 5462 and takes the camera back. A relog at S4247 restores the bind, and the
/// upload matches nothing at any other step.
#[tokio::test]
async fn upload_takes_camera_and_restore_binds_turn_in() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    let upload = [
        "advance_step(1426, 4247)",
        "add_dialog_set(5462, slot=2300)",
        "remove_item(4832, 1)",
    ];
    let restore = ["add_dialog_set(5462, slot=2300)"];
    for archetype in HUMANS {
        let a = Some(archetype);
        let choice = event(
            "dialog_id",
            serde_json::json!(UPLOAD_DIALOG),
            a,
            photo_stage(4246),
        );
        assert_chain(
            &whole,
            TriggerType::DialogChoice,
            &choice,
            HUB_UPLOAD,
            &upload,
        );
        assert_chain(
            &whole,
            TriggerType::PlayerLoaded,
            &login(a, photo_stage(4247)),
            RESTORE_TURN_IN,
            &restore,
        );
        for m in [photo_stage(4245), photo_stage(4247)] {
            let choice = event("dialog_id", serde_json::json!(UPLOAD_DIALOG), a, m);
            let resolved = fire(&whole, TriggerType::DialogChoice, &choice);
            assert!(matched_1426(&resolved).is_empty());
        }
    }
    for archetype in non_human_or_unstamped() {
        let choice = event(
            "dialog_id",
            serde_json::json!(UPLOAD_DIALOG),
            archetype,
            photo_stage(4246),
        );
        let resolved = fire(&whole, TriggerType::DialogChoice, &choice);
        assert!(matched_1426(&resolved).is_empty());
        let login_ctx = login(archetype, photo_stage(4247));
        let resolved = fire(&whole, TriggerType::PlayerLoaded, &login_ctx);
        assert!(matched_1426(&resolved).is_empty());
    }
}

/// Revert: drop the `complete_mission 1426` row of 23171 (the turn-in then only
/// shows the done dialog), or the `step_status 1426 4247` row (the turn-in then
/// runs from any step). At S4247, Carter's click shows the done dialog 4660,
/// unbinds 5462 and completes 1426. Carter's clicks, in every state of the two
/// missions, match at most one dialog-showing chain, and only the expected one.
#[tokio::test]
async fn carter_one_dialog_chain_per_state() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    // (state, the chains of 1425 and 1426 that must match Carter's click there).
    let states: [(Missions, &[i64]); 7] = [
        (HUB_1425, &[]),
        (DELIVER_1425, &[CARTER_SHOW_S4244]),
        (DONE_1425, &[CARTER_SHOW_DONE]),
        (photo_stage(4245), &[]),
        (photo_stage(4246), &[]),
        (photo_stage(4247), &[TURN_IN]),
        (FINISHED, &[]),
    ];
    for (m, expected) in states {
        for archetype in HUMANS {
            let click = event("entity_tag", serde_json::json!(CARTER), Some(archetype), m);
            let resolved = fire(&whole, TriggerType::InteractTag, &click);
            assert_eq!(
                matched_all(&resolved),
                expected.iter().copied().collect::<BTreeSet<i64>>(),
                "archetype {archetype} at 1425 {:?}, 1426 {:?}",
                m.m1425,
                m.m1426
            );
        }
        for archetype in non_human_or_unstamped() {
            let click = event("entity_tag", serde_json::json!(CARTER), archetype, m);
            let resolved = fire(&whole, TriggerType::InteractTag, &click);
            assert!(matched_all(&resolved).is_empty());
        }
    }
    let turn_in = [
        "display_dialog(4660)",
        "remove_dialog_set(5462, slot=2300)",
        "complete_mission(1426)",
    ];
    let click = event(
        "entity_tag",
        serde_json::json!(CARTER),
        Some(HUMANS[0]),
        photo_stage(4247),
    );
    let resolved = fire(&whole, TriggerType::InteractTag, &click);
    assert_eq!(summarized(&resolved, TURN_IN), turn_in);
}

/// Revert: add a `camera 4832` grant to any other chain, or drop the `add_item`
/// from 23165 or 23166 (the grant then goes missing on that path). Only 23165 and
/// 23166 grant item 4832, and only 23169 takes it back. The grants and removal
/// read the item id and the `add_item` / `remove_item` verbs from the live seed.
#[tokio::test]
async fn camera_granted_on_accept_and_taken_on_upload() {
    let pool = require_db_or_skip!();
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT chain_id::text, action_type::text FROM resources.content_actions \
         WHERE target_id = 4832 AND chain_id BETWEEN 23160 AND 23189 \
         ORDER BY chain_id, sort_order",
    )
    .fetch_all(&pool)
    .await
    .expect("read the camera chain actions");
    assert_eq!(
        rows,
        vec![
            ("23165".to_string(), "add_item".to_string()),
            ("23166".to_string(), "add_item".to_string()),
            ("23169".to_string(), "remove_item".to_string()),
        ],
        "only the accepts grant the camera and only the upload takes it back"
    );
    let whole = whole_seed(&pool).await;
    let accept = event(
        "dialog_id",
        serde_json::json!(OFFER_DIALOG),
        Some(HUMANS[0]),
        DELIVER_1425,
    );
    let resolved = fire(&whole, TriggerType::DialogChoice, &accept);
    assert_eq!(
        summarized(&resolved, ACCEPT_S4244)
            .last()
            .map(String::as_str),
        Some("add_item(4832, 1)")
    );
    let upload = event(
        "dialog_id",
        serde_json::json!(UPLOAD_DIALOG),
        Some(HUMANS[0]),
        photo_stage(4246),
    );
    let resolved = fire(&whole, TriggerType::DialogChoice, &upload);
    assert_eq!(
        summarized(&resolved, HUB_UPLOAD).last().map(String::as_str),
        Some("remove_item(4832, 1)")
    );
}
