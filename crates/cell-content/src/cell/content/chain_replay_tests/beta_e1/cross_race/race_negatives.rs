//! Beta Site E1 cross-race guard: no walk state moves a player of another race
//! (UB-45a). Every trigger key, fired at every state with every other
//! archetype and with the archetype unset, matches only race-neutral chains.

use std::collections::BTreeSet;

use super::super::*;
use super::walk::{apply, archetypes, beta_trigger_keys, walk, Race, TriggerKeys};
use crate::test_support::require_db_or_skip;

/// The chains any race may match: the arrival grants and the ungated beam
/// pads. A new entry needs a reason here, and the reviewer checks it.
const RACE_NEUTRAL: &[(i64, &str)] = &[
    (23001, "Human arrival grant of the Omega Site address; moves no mission or race state (UB-01b)"),
    (23002, "Asgard arrival grant of the Omega Site address; moves no mission or race state (UB-01b)"),
    (23003, "Free Jaffa arrival grant of the Omega Site address; moves no mission or race state (UB-01b)"),
    (23314, "Ground ring pad, beam up to the Asgard deck: ungated transport, no dialog (UB-30)"),
    (23315, "Deck ring pad, beam down to the ground base: ungated transport, no dialog (UB-30)"),
];

/// Every trigger the Beta chains answer to, stamped with `archetype` (`None`
/// leaves it unset): a login, an interact click, a dialog choice, an item use,
/// a mission completion and a region entry, one per key.
fn events(keys: &TriggerKeys, archetype: Option<i64>) -> Vec<(TriggerType, ExecutionContext)> {
    let stamp = |mut ctx: ExecutionContext| {
        if let Some(a) = archetype {
            ctx.set_param("archetype".to_string(), serde_json::json!(a));
        }
        ctx
    };
    let mut out = vec![(TriggerType::PlayerLoaded, login_ctx(BETA, archetype))];
    for tag in &keys.tags {
        let mut ctx = ExecutionContext::new();
        ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
        out.push((TriggerType::InteractTag, stamp(ctx)));
    }
    for &dialog in &keys.dialogs {
        let mut ctx = ExecutionContext::new();
        ctx.set_param("dialog_id".to_string(), serde_json::json!(dialog));
        out.push((TriggerType::DialogChoice, stamp(ctx)));
    }
    for &item in &keys.items {
        let mut ctx = ExecutionContext::new();
        ctx.set_param("item_id".to_string(), serde_json::json!(item));
        out.push((TriggerType::ItemUse, stamp(ctx)));
    }
    for &mission in &keys.missions {
        let mut ctx = ExecutionContext::new();
        ctx.set_param("mission_id".to_string(), serde_json::json!(mission));
        out.push((TriggerType::MissionCompleted, stamp(ctx)));
    }
    for region in &keys.regions {
        let mut ctx = ExecutionContext::new();
        ctx.set_param("region_key".to_string(), serde_json::json!(region));
        out.push((TriggerType::RegionEnter, stamp(ctx)));
    }
    out
}

/// For every walk state of every race, each archetype outside the race's own
/// (1-8 minus its archetypes) and an unset archetype, fire every trigger key.
/// The matched chains must be a subset of [`RACE_NEUTRAL`].
/// Revert: drop the `ASG` (`archetype eq 5`) row from chain 23317. The Asgard
/// 481 S1762 state, fired with a Human archetype, then matches 23317 on a
/// Mimir click, and this test must fail.
#[tokio::test]
async fn no_race_state_moves_another_race() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let keys = beta_trigger_keys(&pool).await;
    let neutral: BTreeSet<i64> = RACE_NEUTRAL.iter().map(|(id, _)| *id).collect();
    for race in Race::ALL {
        for state in walk(&pool, race).await {
            let own = archetypes(&state);
            let others = (1..=8)
                .filter(|a| !own.contains(a))
                .map(Some)
                .chain(std::iter::once(None));
            for other in others {
                for (trigger, ctx) in events(&keys, other) {
                    let ctx = apply(ctx, &state);
                    let matched = chains_matched(&fire(&engine, trigger.clone(), &ctx));
                    let foreign: Vec<i64> = matched.difference(&neutral).copied().collect();
                    assert!(
                        foreign.is_empty(),
                        "{} with archetype {other:?}: {trigger:?} matches chains {foreign:?}, which are not race-neutral",
                        state.label
                    );
                }
            }
        }
    }
}
