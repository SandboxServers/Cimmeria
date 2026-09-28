//! The `open_loot` containers seeded on 2026-09-28 (Decision (@Cadacious,
//! 2026-09-28)): the Castle pre-Romney chest (chains 1274-1276) and the
//! Cellblock crate's fallback press (chain 1191). Resolver-level, against the
//! full seeded engine, so a duplicate or missing chain on either tag shows up
//! in the action count.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

fn press(tag: &str, params: &[(&str, serde_json::Value)]) -> (TriggerEvent, ExecutionContext) {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    for (k, v) in params {
        ctx.set_param((*k).to_string(), v.clone());
    }
    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    (event, ctx)
}

fn open(table: Option<i32>, once: bool, key: &str) -> Action {
    Action::OpenLoot {
        loot_table_id: table,
        once_per_character: once,
        container_key: Some(key.to_string()),
    }
}

/// While 703 is active the chest opens the archetype's table once per
/// character (8 for everyone but Jaffa, 9 for Jaffa); without 703 it only
/// reopens pending loot or says why it is empty. Exactly one action per
/// press, so a player never sees two windows or none.
#[tokio::test]
async fn live_db_castle_chest_opens_the_archetype_table_while_703_is_active() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    const KEY: &str = "Castle_PreRomneyChest";

    for (archetype, table) in [(1, 8), (3, 8), (8, 9)] {
        let (event, ctx) = press(
            KEY,
            &[
                ("mission_703_status", serde_json::json!("active")),
                ("archetype", serde_json::json!(archetype)),
            ],
        );
        let actions = engine.resolve_event(&event, &ctx).actions;
        assert_eq!(
            actions,
            vec![(
                if table == 8 { 1274 } else { 1275 },
                open(Some(table), true, KEY)
            )],
            "archetype {archetype} with 703 active opens table {table} once"
        );
    }

    for status in ["not_active", "completed"] {
        let (event, ctx) = press(
            KEY,
            &[
                ("mission_703_status", serde_json::json!(status)),
                ("archetype", serde_json::json!(1)),
            ],
        );
        let actions = engine.resolve_event(&event, &ctx).actions;
        assert_eq!(
            actions,
            vec![(1276, open(None, false, KEY))],
            "703 {status}: the press reopens pending loot or says empty, never rolls"
        );
    }
}

/// The chest tables: the archetype weapon, 2-3 Slappacks, two Focus Heals and
/// 25-75 naquadah, every row certain.
#[tokio::test]
async fn live_db_castle_chest_tables_hold_the_decided_rewards() {
    let pool = require_db_or_skip!();
    for (table, weapon) in [(8, 3127), (9, 3472)] {
        let rows: Vec<(Option<i32>, i32, i32, f32)> = sqlx::query_as(
            "SELECT design_id, min_quantity, max_quantity, probability::real \
             FROM resources.loot WHERE loot_table_id = $1 ORDER BY loot_id",
        )
        .bind(table)
        .fetch_all(&pool)
        .await
        .expect("loot rows");
        assert_eq!(
            rows,
            vec![
                (Some(weapon), 1, 1, 1.0),
                (Some(2893), 2, 3, 1.0),
                (Some(6106), 2, 2, 1.0),
                (None, 25, 75, 1.0),
            ],
            "table {table}"
        );
    }
}

/// Every Cellblock crate press outside step 2354 (before 687, or after the
/// first search) reaches chain 1191's reopen-only `open_loot`, so it is
/// never silent and never rolls.
#[tokio::test]
async fn live_db_cellblock_crate_other_presses_reopen_only() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    const KEY: &str = "Cellblock_WoodenCrate";
    for step in ["not_active", "completed"] {
        let (event, ctx) = press(
            KEY,
            &[
                ("mission_687_status", serde_json::json!("active")),
                ("mission_687_step_2354_status", serde_json::json!(step)),
                ("archetype", serde_json::json!(1)),
            ],
        );
        let actions = engine.resolve_event(&event, &ctx).actions;
        assert_eq!(
            actions,
            vec![(1191, open(None, false, KEY))],
            "step 2354 {step}: reopen-only"
        );
    }
}
