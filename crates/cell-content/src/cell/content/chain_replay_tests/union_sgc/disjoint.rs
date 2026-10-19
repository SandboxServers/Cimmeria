//! Human / Free Jaffa disjointness suite (US-33). Whole-range reads of the
//! seeded chains 3051-3199 that the campaign's guards rely on: world guards,
//! the Human and Free Jaffa archetype gates, the F13 mission-end rule, no
//! Human tutorial credit for a Free Jaffa, and two distinguishable letter
//! recipients for Omega Site.

use std::collections::BTreeMap;

use sqlx::PgPool;

use crate::cell::spawner::load_step_objectives;
use crate::test_support::require_db_or_skip;

use super::letter_1655_jaffa::{after_1654, run_due_crossing, LETTER, OMEGA};
use super::rig::{Rig, Sent, FREE_JAFFA, SOLDIER, WORLD_ID};

const FIRST: i32 = 3051;
const LAST: i32 = 3199;
const M1655: i32 = 1655;
const M1651: i32 = 1651;
const M1562: i32 = 1562;
const M1559: i32 = 1559;
const S4940: i32 = 4940;
const CARTER: &str = "SGC_W1_SamCarter";
const HAMMOND: &str = "SGCW1_GenHammond";
const TEALC: &str = "SGC_W1_Tealc";
const HARRIMAN: &str = "SGC_W1_Harriman";
const D5375: i32 = 5375;
const D5378: i32 = 5378;
const D5854: i32 = 5854;
const D5856: i32 = 5856;
const D5875: i32 = 5875;
const D5857: i32 = 5857;
const WOOLSEY: &str = "SGC_W1_Woolsey";

/// The Human-only Union tutorial missions (1563 to 1571).
const TUTORIAL: std::ops::RangeInclusive<i32> = 1563..=1571;

/// Every archetype row of the seeded chains in range, grouped by chain and
/// in `sort_order`, as `(operator, value)` pairs.
async fn archetype_pairs(pool: &PgPool) -> BTreeMap<i32, Vec<(String, Option<String>)>> {
    let rows: Vec<(i32, String, Option<String>)> = sqlx::query_as(
        "SELECT chain_id, operator, value FROM resources.content_conditions \
         WHERE condition_type = 'archetype' AND chain_id BETWEEN $1 AND $2 \
         ORDER BY chain_id, sort_order",
    )
    .bind(FIRST)
    .bind(LAST)
    .fetch_all(pool)
    .await
    .expect("archetype rows read");
    let mut by_chain: BTreeMap<i32, Vec<(String, Option<String>)>> = BTreeMap::new();
    for (chain, op, value) in rows {
        by_chain.entry(chain).or_default().push((op, value));
    }
    by_chain
}

/// The `(operator, value)` pair the Union Human gate is written with.
fn human_pair() -> Vec<(String, Option<String>)> {
    vec![
        ("gte".to_string(), Some("1".to_string())),
        ("lte".to_string(), Some("4".to_string())),
    ]
}

/// The `(operator, value)` pair the Free Jaffa gate is written with.
fn jaffa_pair() -> Vec<(String, Option<String>)> {
    vec![("eq".to_string(), Some("7".to_string()))]
}

/// **Revert proof:** drop the `world` row of 3133 (a `player_loaded` chain)
/// and this lists 3133. The same holds for any `player_loaded` or
/// `enter_region` chain in range.
#[tokio::test]
async fn live_db_sgc_union_every_loaded_or_region_chain_has_world_58() {
    let pool = require_db_or_skip!();
    let loaded: Vec<(i32,)> = sqlx::query_as(
        "SELECT DISTINCT chain_id FROM resources.content_triggers \
         WHERE event_type IN ('player_loaded', 'enter_region') \
           AND chain_id BETWEEN $1 AND $2 ORDER BY chain_id",
    )
    .bind(FIRST)
    .bind(LAST)
    .fetch_all(&pool)
    .await
    .expect("loaded and region triggers read");
    let worlds: Vec<(i32,)> = sqlx::query_as(
        "SELECT DISTINCT chain_id FROM resources.content_conditions \
         WHERE condition_type = 'world' AND target_id = $1 AND operator = 'eq' \
           AND chain_id BETWEEN $2 AND $3",
    )
    .bind(WORLD_ID)
    .bind(FIRST)
    .bind(LAST)
    .fetch_all(&pool)
    .await
    .expect("world rows read");
    let missing: Vec<i32> = loaded
        .iter()
        .map(|r| r.0)
        .filter(|c| !worlds.iter().any(|w| w.0 == *c))
        .collect();
    assert!(
        missing.is_empty(),
        "player_loaded or enter_region chains without a world 58 row: {missing:?}"
    );
}

/// **Revert proof:** drop the `archetype gte '1'` row of chain 3110 and this
/// lists 3110 as a tutorial chain whose gate is not exactly the Human pair.
#[tokio::test]
async fn live_db_sgc_union_tutorial_chains_are_human_only() {
    let pool = require_db_or_skip!();
    let scoped: Vec<(i32, Option<i32>)> = sqlx::query_as(
        "SELECT chain_id, scope_id FROM resources.content_chains \
         WHERE chain_id BETWEEN $1 AND $2 ORDER BY chain_id",
    )
    .bind(FIRST)
    .bind(LAST)
    .fetch_all(&pool)
    .await
    .expect("chain scopes read");
    let pairs = archetype_pairs(&pool).await;
    let mut bad = Vec::new();
    for (chain, scope) in &scoped {
        let Some(scope) = scope.filter(|s| TUTORIAL.contains(s)) else {
            continue;
        };
        let got = pairs.get(chain).cloned().unwrap_or_default();
        if got != human_pair() {
            bad.push((*chain, scope, got));
        }
    }
    assert!(
        bad.is_empty(),
        "tutorial chains not gated to exactly the Human pair: {bad:?}"
    );
}

/// **Revert proof:** add the `archetype gte '1'` and `lte '4'` pair to chain
/// 3160 and this lists 3160 as a chain that must have no archetype row; a
/// Free Jaffa would then be stranded at 4944. Dropping the Human pair from
/// 3149 or the Jaffa `eq '7'` from 3166 fails the same test.
#[tokio::test]
async fn live_db_sgc_union_race_rows_never_conflict() {
    let pool = require_db_or_skip!();
    let pairs = archetype_pairs(&pool).await;
    let mut conflicts = Vec::new();
    for (chain, rows) in &pairs {
        let has_jaffa = rows.iter().any(|(op, _)| op == "eq");
        let has_human = rows.iter().any(|(op, _)| op == "gte" || op == "lte");
        if rows.len() > 2 || (has_jaffa && has_human) {
            conflicts.push(*chain);
        }
    }
    assert!(
        conflicts.is_empty(),
        "chains with more than the Human pair or mixing both races: {conflicts:?}"
    );
    for chain in [3149, 3156, 3157, 3158, 3159, 3162] {
        assert_eq!(
            pairs.get(&chain).cloned().unwrap_or_default(),
            human_pair(),
            "chain {chain} is the Human race"
        );
    }
    for chain in [3150, 3166, 3167, 3168, 3169] {
        assert_eq!(
            pairs.get(&chain).cloned().unwrap_or_default(),
            jaffa_pair(),
            "chain {chain} is the Free Jaffa race"
        );
    }
    for chain in (3133..=3142)
        .chain([3148, 3151, 3152, 3160, 3161])
        .chain(3163..=3165)
    {
        assert!(
            !pairs.contains_key(&chain),
            "chain {chain} is shared by both races and has no archetype row"
        );
    }
}

/// **Revert proof:** add `(…, 'complete_objective', 1651, '6037', …)` to a
/// scratch copy of 3138 and the mission-1651 list names it; a
/// `complete_objective` on 1655 anywhere, or one on 5645 or 5646 at step 4940,
/// is listed too (F13 and F21).
#[tokio::test]
async fn live_db_sgc_union_no_mission_ending_completion_mid_mission() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, i32, Option<String>)> = sqlx::query_as(
        "SELECT chain_id, target_id, target_key FROM resources.content_actions \
         WHERE action_type = 'complete_objective' AND chain_id BETWEEN $1 AND $2 \
         ORDER BY chain_id",
    )
    .bind(FIRST)
    .bind(LAST)
    .fetch_all(&pool)
    .await
    .expect("complete_objective rows read");
    let mut bad = Vec::new();
    for (chain, mission, key) in &rows {
        if *mission == M1655 || (*mission == M1651 && key.as_deref() != Some("5956")) {
            bad.push((*chain, *mission, key.clone()));
        }
    }
    assert!(
        bad.is_empty(),
        "complete_objective on 1655, or on 1651 other than 5956: {bad:?}"
    );

    let at_4940: Vec<(i32,)> = sqlx::query_as(
        "SELECT a.chain_id FROM resources.content_actions a \
         WHERE a.action_type = 'complete_objective' AND a.target_key IN ('5645', '5646') \
           AND a.chain_id BETWEEN $1 AND $2 \
           AND EXISTS (SELECT 1 FROM resources.content_conditions c \
                       WHERE c.chain_id = a.chain_id AND c.condition_type = 'step_status' \
                         AND c.target_key = '4940')",
    )
    .bind(FIRST)
    .bind(LAST)
    .fetch_all(&pool)
    .await
    .expect("F21 rows read");
    assert!(
        at_4940.is_empty(),
        "no chain completes 5645 or 5646 on step 4940: {at_4940:?}"
    );
}

/// **Revert proof:** drop both `archetype` rows of 3051 (US-12's 1563 offer)
/// and a Free Jaffa who finishes 1562 and 1559 is offered 1563 by Carter's
/// press, so `status(1563)` is set and this fails.
#[tokio::test]
async fn live_db_sgc_union_free_jaffa_gains_no_human_tutorial_credit() {
    let pool = require_db_or_skip!();
    let mut rig = Rig::new(&pool, Some(FREE_JAFFA)).await;
    rig.mgr.step_objectives = load_step_objectives(&pool)
        .await
        .expect("load the step objectives");
    rig.finish(M1562).await;
    rig.finish(M1559).await;
    rig.loaded().await;
    for tag in [CARTER, HAMMOND, TEALC, HARRIMAN] {
        rig.press(tag).await;
    }
    rig.close(D5375).await;
    rig.close(D5378).await;
    rig.sent();
    for mission in TUTORIAL {
        assert_eq!(
            rig.status(mission),
            None,
            "a Free Jaffa has no credit for tutorial mission {mission}"
        );
    }
}

/// **Revert proof:** delete the `add_item` row of 3159 and the Soldier gets no
/// letter, so the first assertion fails. Adding the letter to the Jaffa path
/// (3168) fails the second.
#[tokio::test]
async fn live_db_sgc_union_letter_recipients_are_distinguishable() {
    let pool = require_db_or_skip!();

    let mut soldier = after_1654(&pool, Some(SOLDIER)).await;
    soldier.press(WOOLSEY).await;
    soldier.choose(D5854, 0).await;
    soldier.press(HAMMOND).await;
    soldier.close(D5856).await;
    soldier.press(HARRIMAN).await;
    soldier.close(D5857).await;
    let (soldier_steps, soldier_crosses) = run_due_crossing(&mut soldier, 3000).await;
    let soldier_sent = soldier.sent();
    let letters = soldier_sent
        .iter()
        .filter(|s| matches!(s, Sent::Item(LETTER, 2, 1)))
        .count();
    assert_eq!(letters, 1, "the Soldier gets exactly one letter 6786");

    let mut jaffa = after_1654(&pool, Some(FREE_JAFFA)).await;
    jaffa.press(WOOLSEY).await;
    jaffa.press(HAMMOND).await;
    jaffa.close(D5875).await;
    jaffa.press(HARRIMAN).await;
    jaffa.close(D5857).await;
    let (jaffa_steps, jaffa_crosses) = run_due_crossing(&mut jaffa, 3000).await;
    let jaffa_sent = jaffa.sent();
    assert!(
        !jaffa_sent
            .iter()
            .any(|s| matches!(s, Sent::Item(LETTER, _, _))),
        "the Free Jaffa gets no letter 6786"
    );

    for (who, steps, crosses) in [
        ("Soldier", soldier_steps, soldier_crosses),
        ("Free Jaffa", jaffa_steps, jaffa_crosses),
    ] {
        assert_eq!(steps, vec![Some(S4940)], "{who}: the cross happens on 4940");
        assert!(
            matches!(&crosses[..], [Sent::Travel(w, _)] if w == OMEGA),
            "{who}: one cross to Omega_Site, got {crosses:?}"
        );
    }
}
