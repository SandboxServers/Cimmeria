//! Black Market auctioneer chains
//! (`db/resources/Content/Seed/space_castle_cellblock_chains.sql`):
//! chain 5030 sets the `INT_Auction` interaction bit on player load, and
//! chain 5031 runs `open_black_market` when the player interacts with the
//! `BlackMarket_Auctioneer` tag.
//!
//! These load the seeded chains from the DB through the same pipeline the
//! cell service uses at startup, so they double as seed-integrity guards:
//! a typo in the action verb, the tag, or the interaction mask surfaces
//! here rather than only in-game.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Chain 5031 must resolve to exactly one `open_black_market` action when
/// the player interacts with the auctioneer tag. Reverting the seed (or
/// renaming the action verb / tag) breaks this guard.
#[tokio::test]
async fn chain_5031_fires_open_black_market_on_auctioneer_interact() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 5031)
        .await
        .expect("DB query for chain 5031 must succeed")
        .expect("chain 5031 (Black Market auctioneer open) must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("BlackMarket_Auctioneer"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let actions: Vec<_> = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 5031)
        .collect();

    assert_eq!(
        actions.len(),
        1,
        "chain 5031 must resolve to exactly one action; got {}",
        actions.len()
    );
    assert!(
        matches!(actions[0].1, Action::OpenBlackMarket),
        "chain 5031's action must be OpenBlackMarket, got {:?}",
        actions[0].1
    );
}

/// Chain 5031 must NOT fire on a different interact tag — pins that the
/// trigger key is the auctioneer tag, not a wildcard.
#[tokio::test]
async fn chain_5031_does_not_fire_on_other_tag() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 5031)
        .await
        .expect("DB query for chain 5031 must succeed")
        .expect("chain 5031 must exist");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Preparation_ColMarsh"),
    );
    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    assert!(
        resolved.actions.iter().all(|(id, _)| *id != 5031),
        "chain 5031 must only match the BlackMarket_Auctioneer tag"
    );
}

/// Chain 5030 must set the `INT_Auction` (mask = 4) interaction bit on the
/// auctioneer tag on player load. Without it the client renders the NPC as
/// scenery and never sends the interact click, so 5031 could never fire.
#[tokio::test]
async fn chain_5030_sets_auction_interaction_bit_on_player_load() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 5030)
        .await
        .expect("DB query for chain 5030 must succeed")
        .expect("chain 5030 (auctioneer interactable) must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: None,
        target_entity: None,
        params: ExecutionContext::new().params,
    };

    let resolved = engine.resolve_event(&event, &ExecutionContext::new());
    let action = resolved
        .actions
        .iter()
        .find(|(id, _)| *id == 5030)
        .map(|(_, a)| a)
        .expect("chain 5030 must resolve a set_interaction_type action on player load");

    match action {
        Action::SetInteractionType {
            entity_tag,
            operation,
            mask,
        } => {
            assert_eq!(entity_tag, "BlackMarket_Auctioneer");
            assert_eq!(operation, "|", "must OR the bit in, not overwrite");
            assert_eq!(
                *mask,
                cimmeria_entity::interaction_flags::INT_AUCTION,
                "mask must be INT_Auction (4) so the Black Market prompt shows"
            );
        }
        other => panic!("chain 5030 action must be SetInteractionType, got {other:?}"),
    }
}
