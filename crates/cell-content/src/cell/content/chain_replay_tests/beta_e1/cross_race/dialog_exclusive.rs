//! Beta Site E1 cross-race guard: one dialog per click and one offer per
//! choice, on every walk state of every race (UB-45a).

use super::super::*;
use super::walk::{apply, archetypes, beta_trigger_keys, walk, Race};
use crate::test_support::require_db_or_skip;

/// The matched chains whose rendered actions include an action starting with
/// `prefix`.
fn chains_with(resolved: &ResolvedActions, prefix: &str) -> Vec<i64> {
    chains_matched(resolved)
        .into_iter()
        .filter(|&chain| {
            summarized(resolved, chain)
                .iter()
                .any(|action| action.starts_with(prefix))
        })
        .collect()
}

/// On every walk state, for every race archetype (all four Human ones on a
/// Human state) and every interact tag, at most one matched chain shows a
/// dialog. Two chains that show on one click would stack two dialogs, so
/// this is the guard against a sibling chain on a shared NPC.
/// Revert: in a scratch copy of the seed, delete the `mission_status 481
/// completed` row of chain 23341 (420's offer show). At the 481 S1762 state a
/// Mimir click then shows 1916 and 1953, and this test must fail.
#[tokio::test]
async fn every_walk_state_shows_at_most_one_dialog_per_click() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let keys = beta_trigger_keys(&pool).await;
    for race in Race::ALL {
        for state in walk(&pool, race).await {
            for archetype in archetypes(&state) {
                for tag in &keys.tags {
                    let ctx = apply(interact_ctx(tag, archetype), &state);
                    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
                    let shown = chains_with(&resolved, "display_dialog(");
                    assert!(
                        shown.len() <= 1,
                        "{} archetype {archetype}, tag {tag}: chains {shown:?} each show a dialog",
                        state.label
                    );
                }
            }
        }
    }
}

/// On every walk state, for every race archetype and every dialog choice key,
/// at most one matched chain accepts a mission. A second accept on the same
/// choice would enrol the player in two missions from one offer.
#[tokio::test]
async fn every_offer_choice_resolves_to_one_chain() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let keys = beta_trigger_keys(&pool).await;
    for race in Race::ALL {
        for state in walk(&pool, race).await {
            for archetype in archetypes(&state) {
                for &dialog in &keys.dialogs {
                    let ctx = apply(choice_ctx(dialog, archetype), &state);
                    let resolved = fire(&engine, TriggerType::DialogChoice, &ctx);
                    let accepted = chains_with(&resolved, "accept_mission(");
                    assert!(
                        accepted.len() <= 1,
                        "{} archetype {archetype}, dialog {dialog}: chains {accepted:?} each accept a mission",
                        state.label
                    );
                }
            }
        }
    }
}
