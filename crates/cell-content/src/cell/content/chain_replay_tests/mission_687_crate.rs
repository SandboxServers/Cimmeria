//! Mission 687 — Aftermath, the crate's class rewards (Class Start v6,
//! CS-04; OD-CS02, OD-CS06, OD-CS08, OD-CS12).
//!
//! Searching `Cellblock_WoodenCrate` while step 2354 is active is a five-way
//! class split: chains 1192 (Soldier), 1098 (Commando), 1193 (Scientist),
//! 1194 (Archaeologist) and 1099 (Loyalist Jaffa) each show their own
//! dialog, open their own loot table and grant one signature ability. Chain
//! 1195 is every other archetype: the Goa'uld holding state, which keeps the
//! reward the old non-Jaffa chain gave and gets no signature.
//!
//! Resolver-level, against the full seeded engine (`build_engine`, which
//! also runs the loader's unknown-ability and unknown-tutorial refusals), so
//! a refused, missing or duplicate chain on the crate shows up as a wrong
//! action list. The loot window itself is `executor/tests/open_loot.rs`'s;
//! the base's half of the grant is `content_grant_write`'s.

use cimmeria_content_engine::ability_grant::AbilityGrant;
use cimmeria_content_engine::actions::{AbilityGrantKind, Action};
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

const CRATE: &str = "Cellblock_WoodenCrate";
const HOLDING_CHAIN: i64 = 1195;
const COMMANDO_TABLE: i32 = 10;
const COMMANDO_DIALOG: i32 = 3942;

/// One row of the campaign's Gear and Abilities matrices for M687.
struct ClassReward {
    class: &'static str,
    /// `EArchetype` ordinal (`entities/defs/enumerations.xml`).
    archetype: i32,
    chain: i64,
    dialog: i32,
    table: i32,
    items: &'static [i32],
    signature: i32,
}

const CLASS_REWARDS: [ClassReward; 5] = [
    ClassReward {
        class: "Soldier",
        archetype: 1,
        chain: 1192,
        dialog: 2517,
        table: 12,
        items: &[3260, 7373],
        signature: 598,
    },
    ClassReward {
        class: "Commando",
        archetype: 2,
        chain: 1098,
        dialog: COMMANDO_DIALOG,
        table: COMMANDO_TABLE,
        items: &[3347, 3359, 3372, 3387, 3401, 3325],
        signature: 646,
    },
    ClassReward {
        class: "Scientist",
        archetype: 3,
        chain: 1193,
        dialog: 4408,
        table: 13,
        items: &[4444, 7373],
        signature: 948,
    },
    ClassReward {
        class: "Archaeologist",
        archetype: 4,
        chain: 1194,
        dialog: 4409,
        table: 14,
        items: &[6843, 7373],
        signature: 802,
    },
    ClassReward {
        class: "Loyalist Jaffa",
        archetype: 8,
        chain: 1099,
        dialog: 3943,
        table: 11,
        items: &[4342, 2797],
        signature: 1984,
    },
];

/// Every action a crate press resolves while step 2354 is active, across
/// the whole seeded engine. `archetype = None` leaves the param out, the
/// shape a trigger with no archetype has (the condition then reads -1).
fn search_crate(engine: &ChainEngine, archetype: Option<i32>) -> Vec<(i64, Action)> {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(CRATE));
    ctx.set_param(
        "mission_687_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_687_step_2354_status".to_string(),
        serde_json::json!("active"),
    );
    if let Some(archetype) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(archetype));
    }
    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx).actions
}

/// What every crate chain does before any grant: the dialog, one
/// once-per-character loot window, the step advance and the highlight clear.
fn crate_search(chain: i64, dialog: i32, table: i32) -> Vec<(i64, Action)> {
    vec![
        (chain, Action::DisplayDialog { dialog_id: dialog }),
        (
            chain,
            Action::OpenLoot {
                loot_table_id: Some(table),
                once_per_character: true,
                container_key: Some(CRATE.to_string()),
            },
        ),
        (
            chain,
            Action::AdvanceStep {
                mission_id: 687,
                step_id: 2355,
            },
        ),
        (
            chain,
            Action::SetInteractionType {
                entity_tag: CRATE.to_string(),
                operation: "~".to_string(),
                mask: cimmeria_entity::interaction_flags::INT_MISSION_WORLD_OBJECT,
            },
        ),
    ]
}

/// **Guard: the crate is a five-way class split, one window and one
/// signature per class (OD-CS02, OD-CS06).** For each class the press
/// resolves exactly its own chain: its dialog, its loot table once per
/// character, the advance to step 2355, the highlight clear, then one
/// `signature` grant of its ability, source 687, gated on its own archetype
/// on the grant row itself. The whole list is compared, so a second chain
/// matching the same class (two windows, two kits), a missing or refused
/// class chain (a dead crate), a wrong table or dialog, a grant with no
/// `archetypes` gate and a grant before the loot window all fail it.
/// Restore the Human/Jaffa pair and every class but Commando and Jaffa
/// fails on the chain id, and those two on the missing grant.
#[tokio::test]
async fn live_db_crate_gives_each_class_its_own_table_and_signature() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    for reward in &CLASS_REWARDS {
        let mut expected = crate_search(reward.chain, reward.dialog, reward.table);
        expected.push((
            reward.chain,
            Action::GrantAbility(AbilityGrant {
                ability_ids: vec![reward.signature],
                source_kind: AbilityGrantKind::Signature,
                source_id: Some(687),
                archetypes: vec![reward.archetype],
            }),
        ));
        assert_eq!(
            search_crate(&engine, Some(reward.archetype)),
            expected,
            "{} (archetype {}) searching the crate",
            reward.class,
            reward.archetype
        );
    }
}

/// **Guard: the Goa'uld holding state keeps the old reward and gets no
/// signature (OD-CS08).** A Goa'uld (archetype 6, char_defs 10/19) searching
/// the crate resolves exactly what the non-Jaffa chain resolved before
/// CS-04: dialog 3942, loot table 10, the advance and the highlight clear,
/// and nothing else. The same holds for every other value outside the five
/// classes, a missing archetype included, so the crate is never a dead end.
/// Give the holding chain a grant, or let a class chain's gate reach
/// archetype 6, and the list is no longer equal; drop the holding chain and
/// it is empty.
#[tokio::test]
async fn live_db_crate_holding_state_keeps_table_10_and_grants_nothing() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let legacy = crate_search(HOLDING_CHAIN, COMMANDO_DIALOG, COMMANDO_TABLE);

    assert_eq!(
        search_crate(&engine, Some(6)),
        legacy,
        "a Goa'uld gets the pre-CS-04 crate and no grant"
    );
    for archetype in [None, Some(0), Some(5), Some(7), Some(9)] {
        assert_eq!(
            search_crate(&engine, archetype),
            legacy,
            "archetype {archetype:?} has no class row and takes the holding chain"
        );
    }
}

/// **Guard: the class tables hold the Gear matrix, every row certain, and
/// no gun in them arrives loaded (OD-CS12, OD-CS13).** Each table's item
/// rows are exactly its class's set at probability 1 and quantity 1, each
/// item exists, and none has a seeded `charges` above 0 (a looted weapon's
/// rounds start at `charges`). A swapped or dropped item, a chance row, an
/// item id with no `resources.items` row and a pre-loaded gun all fail it.
#[tokio::test]
async fn live_db_class_reward_tables_hold_the_gear_matrix() {
    let pool = require_db_or_skip!();

    for reward in &CLASS_REWARDS {
        let rows: Vec<(Option<i32>, i32, i32, f32)> = sqlx::query_as(
            "SELECT design_id, min_quantity, max_quantity, probability::real \
             FROM resources.loot WHERE loot_table_id = $1 ORDER BY loot_id",
        )
        .bind(reward.table)
        .fetch_all(&pool)
        .await
        .expect("loot rows");
        let expected: Vec<(Option<i32>, i32, i32, f32)> = reward
            .items
            .iter()
            .map(|&item| (Some(item), 1, 1, 1.0))
            .collect();
        assert_eq!(rows, expected, "table {} ({})", reward.table, reward.class);

        let seeded: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT item_id, charges FROM resources.items \
             WHERE item_id = ANY($1) ORDER BY item_id",
        )
        .bind(reward.items)
        .fetch_all(&pool)
        .await
        .expect("item rows");
        assert_eq!(
            seeded.len(),
            reward.items.len(),
            "every {} reward item is seeded",
            reward.class
        );
        assert!(
            seeded.iter().all(|&(_, charges)| charges == 0),
            "no {} reward arrives with rounds or charges: {seeded:?}",
            reward.class
        );
    }
}
