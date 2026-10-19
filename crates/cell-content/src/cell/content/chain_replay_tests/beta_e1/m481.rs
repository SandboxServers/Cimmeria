//! Beta Site E1 From on High (UB-30, mission 481): the Asgard receiver
//! (23310, 23311, 23313), the Carter hand-in (23312), the beam pads (23314,
//! 23315), the teleporter objective (23316) and the Mimir turn-in (23317).
//! Seeded in `db/resources/Content/Seed/beta_e1_m481_chains.sql`.
//!
//! Omega UO-16, the Asgard producer, is HELD (HELD-B16), so no live player
//! reaches the receiver yet. These tests replay archetype 5 (D-UB13).

use std::collections::BTreeSet;

use super::super::assert_no_deferred_actions;
use super::{
    chains_matched, fire, login_ctx, summarized, whole_seed, with_mission, with_obj, with_step,
    ALL_ARCHETYPES, ASGARD, BETA,
};
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::ResolvedActions;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;

const MISSION: i32 = 481;
const RECEIVER: i64 = 23310;
const RECEIVER_RELOG_TALK: i64 = 23311;
const CARTER_HANDIN: i64 = 23312;
const MIMIR_RELOG_TURNIN: i64 = 23313;
const BEAM_UP: i64 = 23314;
const BEAM_DOWN: i64 = 23315;
const TELEPORTER_OBJECTIVE: i64 = 23316;
const MIMIR_TURNIN: i64 = 23317;
const M481_CHAINS: std::ops::RangeInclusive<i64> = 23310..=23317;

/// A world other than Beta, for the `player_loaded` world-key negatives.
const OTHER_WORLD: &str = "Castle_Cellblock";

const GROUND_PAD: &str = "BetaSite_00000007";
const DECK_PAD: &str = "BetaSite_Beam_000007_Ship";

/// Sealed beam destinations (placement-seal.md § beam, UB-00b Q3). UB-02c's
/// spawner test holds the same strings; `beam_pads_move_between_the_seal_points`
/// also checks them against the seeded `move_entity` rows.
const BEAM_DECK_DEST: &str = "698.98,157.93,70.36";
const BEAM_GROUND_DEST: &str = "745.05,-19.60,68.75";

/// The rendered `move_entity` lines (`summarize` prints each coordinate).
const BEAM_DECK_MOVE: &str = "move_entity(698.98, 157.93, 70.36)";
const BEAM_GROUND_MOVE: &str = "move_entity(745.05, -19.6, 68.75)";

/// The mission 481 chains that `resolved` matched, for the exclusivity checks.
fn m481_matched(resolved: &ResolvedActions) -> BTreeSet<i64> {
    chains_matched(resolved)
        .into_iter()
        .filter(|id| M481_CHAINS.contains(id))
        .collect()
}

/// Mark mission 481 active at `step` (the way `populate_mission_context` does).
fn at_step(mut ctx: ExecutionContext, step: i32) -> ExecutionContext {
    with_mission(&mut ctx, MISSION, "active");
    with_step(&mut ctx, MISSION, step, "active");
    ctx
}

/// Context for an `interact_tag` click on `tag`. `None` leaves the archetype unset.
fn tag_ctx(tag: &str, archetype: Option<i64>) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    if let Some(a) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(a));
    }
    ctx
}

/// Every race that is not Asgard: `None` and archetypes 0-4, 6-8.
fn non_asgard() -> Vec<Option<i64>> {
    std::iter::once(None)
        .chain(
            ALL_ARCHETYPES
                .iter()
                .copied()
                .filter(|a| *a != ASGARD)
                .map(Some),
        )
        .collect()
}

/// Revert: drop 23310's `advance_step` or its `add_dialog_set` row, or move
/// its step gate off 1760. The exact list then differs.
#[tokio::test]
async fn receiver_asgard_arrival_with_s1760_active_moves_to_carter() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let ctx = at_step(login_ctx(BETA, Some(ASGARD)), 1760);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
    assert_eq!(
        summarized(&resolved, RECEIVER),
        vec![
            "advance_step(481, 1761)".to_string(),
            "add_dialog_set(2465, slot=2300)".to_string(),
        ],
    );
    assert_no_deferred_actions(&resolved, RECEIVER);
    assert_eq!(m481_matched(&resolved), BTreeSet::from([RECEIVER]));
}

/// Revert: remove 23310's archetype or step gate. Then a non-Asgard or a
/// player past S1760 would also move to Carter.
#[tokio::test]
async fn receiver_ignores_other_races_and_states() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for archetype in non_asgard() {
        let ctx = at_step(login_ctx(BETA, archetype), 1760);
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
        assert!(
            m481_matched(&resolved).is_empty(),
            "archetype {archetype:?} at S1760 must not match the receiver",
        );
    }
    for step in [1761, 1762] {
        let ctx = at_step(login_ctx(BETA, Some(ASGARD)), step);
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
        assert!(
            !m481_matched(&resolved).contains(&RECEIVER),
            "the receiver must not fire at S{step}",
        );
    }
    let mut completed = login_ctx(BETA, Some(ASGARD));
    with_mission(&mut completed, MISSION, "completed");
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &completed);
    assert!(
        m481_matched(&resolved).is_empty(),
        "a completed 481 must not re-offer"
    );
}

/// Revert: drop the `Beta_Site_Evo_1` event key from 23310, 23311 or 23313 and
/// an Asgard who logs into another world at S1760-S1762 matches it.
#[tokio::test]
async fn login_chains_ignore_other_worlds() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for step in [1760, 1761, 1762] {
        let ctx = at_step(login_ctx(OTHER_WORLD, Some(ASGARD)), step);
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
        assert!(
            m481_matched(&resolved).is_empty(),
            "S{step} in {OTHER_WORLD} must match no 481 chain"
        );
    }
}

/// Revert: delete 23311 or 23313. A relogging Asgard then has no bind on
/// Carter's or Mimir's dialog.
#[tokio::test]
async fn receiver_relog_restores_the_bind_at_s1761_and_s1762() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let talk = fire(
        &engine,
        TriggerType::PlayerLoaded,
        &at_step(login_ctx(BETA, Some(ASGARD)), 1761),
    );
    assert_eq!(
        summarized(&talk, RECEIVER_RELOG_TALK),
        vec!["add_dialog_set(2465, slot=2300)".to_string()],
    );
    assert_eq!(m481_matched(&talk), BTreeSet::from([RECEIVER_RELOG_TALK]));
    let turnin = fire(
        &engine,
        TriggerType::PlayerLoaded,
        &at_step(login_ctx(BETA, Some(ASGARD)), 1762),
    );
    assert_eq!(
        summarized(&turnin, MIMIR_RELOG_TURNIN),
        vec!["add_dialog_set(2467, slot=2305)".to_string()],
    );
    assert_eq!(m481_matched(&turnin), BTreeSet::from([MIMIR_RELOG_TURNIN]));
    for archetype in non_asgard() {
        for step in [1761, 1762] {
            let resolved = fire(
                &engine,
                TriggerType::PlayerLoaded,
                &at_step(login_ctx(BETA, archetype), step),
            );
            assert!(
                m481_matched(&resolved).is_empty(),
                "archetype {archetype:?} at S{step}"
            );
        }
    }
}

/// Revert: drop the `remove_item` row, or the `display_dialog`. The list
/// then loses `remove_item(3399, 1)` (HELD-B1b) or the 1915 line.
#[tokio::test]
async fn carter_handin_takes_the_communique_and_sends_asgard_to_mimir() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let ctx = at_step(tag_ctx("BetaE1_Carter", Some(ASGARD)), 1761);
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_eq!(
        summarized(&resolved, CARTER_HANDIN),
        vec![
            "display_dialog(1915)".to_string(),
            "remove_dialog_set(2465, slot=2300)".to_string(),
            "remove_item(3399, 1)".to_string(),
            "advance_step(481, 1762)".to_string(),
            "add_dialog_set(2467, slot=2305)".to_string(),
        ],
    );
    assert_no_deferred_actions(&resolved, CARTER_HANDIN);
    assert_eq!(m481_matched(&resolved), BTreeSet::from([CARTER_HANDIN]));
    for archetype in non_asgard() {
        let ctx = at_step(tag_ctx("BetaE1_Carter", archetype), 1761);
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        assert!(
            m481_matched(&resolved).is_empty(),
            "archetype {archetype:?} at S1761"
        );
    }
    for step in [1760, 1762] {
        let ctx = at_step(tag_ctx("BetaE1_Carter", Some(ASGARD)), step);
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        assert!(
            m481_matched(&resolved).is_empty(),
            "Carter at S{step} must not hand in"
        );
    }
    let ctx = at_step(tag_ctx("BetaE1_Mimir", Some(ASGARD)), 1761);
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert!(!m481_matched(&resolved).contains(&CARTER_HANDIN));
}

/// Revert: change either pad's destination in the seed. The exact list and
/// the DB check both fail. The beam is ungated, so every race rides it.
#[tokio::test]
async fn beam_pads_move_between_the_seal_points() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for archetype in ALL_ARCHETYPES.iter().map(|a| Some(*a)).chain([None]) {
        let up = fire(
            &engine,
            TriggerType::InteractTag,
            &tag_ctx(GROUND_PAD, archetype),
        );
        assert_eq!(summarized(&up, BEAM_UP), vec![BEAM_DECK_MOVE.to_string()]);
        assert_no_deferred_actions(&up, BEAM_UP);
        assert_eq!(m481_matched(&up), BTreeSet::from([BEAM_UP]));
        let down = fire(
            &engine,
            TriggerType::InteractTag,
            &tag_ctx(DECK_PAD, archetype),
        );
        assert_eq!(
            summarized(&down, BEAM_DOWN),
            vec![BEAM_GROUND_MOVE.to_string()]
        );
        assert_no_deferred_actions(&down, BEAM_DOWN);
        assert_eq!(m481_matched(&down), BTreeSet::from([BEAM_DOWN]));
    }
    for (chain, expected) in [(BEAM_UP, BEAM_DECK_DEST), (BEAM_DOWN, BEAM_GROUND_DEST)] {
        let seeded: String = sqlx::query_scalar(
            "SELECT params->>'destination' FROM resources.content_actions \
             WHERE chain_id = $1 AND action_type = 'move_entity'",
        )
        .bind(chain as i32)
        .fetch_one(&pool)
        .await
        .expect("the beam chain must have one move_entity row");
        assert_eq!(
            seeded, expected,
            "chain {chain} destination must match the seal"
        );
    }
}

/// Revert: add `complete_objective 481 4015` to 23316, or drop O2080's gate.
/// The exact list and the DB count both catch it: O4015 is only closed by
/// the Mimir turn-in (23317), so a pad click can never finish the mission.
#[tokio::test]
async fn teleporter_objective_never_completes_481() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut open = at_step(tag_ctx(GROUND_PAD, Some(ASGARD)), 1762);
    with_obj(&mut open, MISSION, 2080, "active");
    let resolved = fire(&engine, TriggerType::InteractTag, &open);
    assert_eq!(
        summarized(&resolved, TELEPORTER_OBJECTIVE),
        vec!["complete_objective(481, 2080)".to_string()],
    );
    assert_eq!(
        m481_matched(&resolved),
        BTreeSet::from([BEAM_UP, TELEPORTER_OBJECTIVE]),
    );
    let mut done = at_step(tag_ctx(GROUND_PAD, Some(ASGARD)), 1762);
    with_obj(&mut done, MISSION, 2080, "completed");
    let resolved = fire(&engine, TriggerType::InteractTag, &done);
    assert_eq!(m481_matched(&resolved), BTreeSet::from([BEAM_UP]));
    // O2080 is active in every negative below, so only the archetype or the
    // step gate can keep 23316 from matching (a closed objective alone would).
    for archetype in non_asgard() {
        let mut ctx = at_step(tag_ctx(GROUND_PAD, archetype), 1762);
        with_obj(&mut ctx, MISSION, 2080, "active");
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        assert_eq!(
            m481_matched(&resolved),
            BTreeSet::from([BEAM_UP]),
            "archetype {archetype:?} must not close O2080"
        );
    }
    for step in [1760, 1761] {
        let mut ctx = at_step(tag_ctx(GROUND_PAD, Some(ASGARD)), step);
        with_obj(&mut ctx, MISSION, 2080, "active");
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        assert_eq!(
            m481_matched(&resolved),
            BTreeSet::from([BEAM_UP]),
            "S{step} must not close O2080"
        );
    }
    let completes_4015: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resources.content_actions \
         WHERE action_type = 'complete_objective' AND target_id = 481 AND target_key = '4015'",
    )
    .fetch_one(&pool)
    .await
    .expect("count complete_objective 481 4015 rows");
    assert_eq!(
        completes_4015, 0,
        "no chain may complete O4015 (Converse with Mimir)"
    );
}

/// Revert: drop 23317's `complete_mission` or its `display_dialog`, or
/// widen the step gate. The exact list then differs.
#[tokio::test]
async fn mimir_turnin_completes_481() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let ctx = at_step(tag_ctx("BetaE1_Mimir", Some(ASGARD)), 1762);
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_eq!(
        summarized(&resolved, MIMIR_TURNIN),
        vec![
            "display_dialog(1916)".to_string(),
            "remove_dialog_set(2467, slot=2305)".to_string(),
            "complete_mission(481)".to_string(),
        ],
    );
    assert_no_deferred_actions(&resolved, MIMIR_TURNIN);
    assert_eq!(m481_matched(&resolved), BTreeSet::from([MIMIR_TURNIN]));
    for archetype in non_asgard() {
        let ctx = at_step(tag_ctx("BetaE1_Mimir", archetype), 1762);
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        assert!(
            m481_matched(&resolved).is_empty(),
            "archetype {archetype:?} at S1762"
        );
    }
    let ctx = at_step(tag_ctx("BetaE1_Mimir", Some(ASGARD)), 1761);
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert!(
        m481_matched(&resolved).is_empty(),
        "Mimir must not close 481 before S1762"
    );
}

/// Revert: any gate change that lets a second chain fire on one event shows
/// here as an extra id. Each positive event matches exactly its packet chains.
#[tokio::test]
async fn whole_seed_fires_exactly_the_packet_chains() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let cases: Vec<(TriggerType, ExecutionContext, BTreeSet<i64>)> = vec![
        (
            TriggerType::PlayerLoaded,
            at_step(login_ctx(BETA, Some(ASGARD)), 1760),
            BTreeSet::from([RECEIVER]),
        ),
        (
            TriggerType::PlayerLoaded,
            at_step(login_ctx(BETA, Some(ASGARD)), 1761),
            BTreeSet::from([RECEIVER_RELOG_TALK]),
        ),
        (
            TriggerType::PlayerLoaded,
            at_step(login_ctx(BETA, Some(ASGARD)), 1762),
            BTreeSet::from([MIMIR_RELOG_TURNIN]),
        ),
        (
            TriggerType::InteractTag,
            at_step(tag_ctx("BetaE1_Carter", Some(ASGARD)), 1761),
            BTreeSet::from([CARTER_HANDIN]),
        ),
        (
            TriggerType::InteractTag,
            at_step(tag_ctx(DECK_PAD, Some(ASGARD)), 1762),
            BTreeSet::from([BEAM_DOWN]),
        ),
        (
            TriggerType::InteractTag,
            at_step(tag_ctx("BetaE1_Mimir", Some(ASGARD)), 1762),
            BTreeSet::from([MIMIR_TURNIN]),
        ),
    ];
    for (trigger, ctx, expected) in cases {
        let label = format!("{trigger:?} with {:?}", ctx.params.get("entity_tag"));
        let resolved = fire(&engine, trigger, &ctx);
        assert_eq!(m481_matched(&resolved), expected, "{label}");
    }
    let mut pad = at_step(tag_ctx(GROUND_PAD, Some(ASGARD)), 1762);
    with_obj(&mut pad, MISSION, 2080, "active");
    let resolved = fire(&engine, TriggerType::InteractTag, &pad);
    assert_eq!(
        m481_matched(&resolved),
        BTreeSet::from([BEAM_UP, TELEPORTER_OBJECTIVE]),
    );
}
