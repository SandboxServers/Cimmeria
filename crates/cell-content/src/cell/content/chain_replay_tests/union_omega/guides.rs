//! UO-35: the eight crafting guides (template 63829, spawns 63810-63817)
//! point players to their vendors. Each click speaks one line naming the
//! vendors of that specialty (D-UO17 c, D-UO31); no reward (D-UO37).

use super::services_common::*;

const TEMPLATE: i32 = 63829;
/// UO-15's guide speaker string, verbatim.
const SPEAKER: &str = "SGC Scientist";
/// (spawn, tag, chain, screen)
const GUIDES: [(i32, &str, i32, i32); 8] = [
    (63810, "UnionOmega_Guide_Biology", 63852, 63901),
    (63811, "UnionOmega_Guide_Electronics", 63853, 63902),
    (63812, "UnionOmega_Guide_AdvElectronics", 63853, 63902),
    (63813, "UnionOmega_Guide_Engineering", 63854, 63903),
    (63814, "UnionOmega_Guide_AdvEngineering", 63854, 63903),
    (63815, "UnionOmega_Guide_Physics", 63855, 63904),
    (63816, "UnionOmega_Guide_AdvPhysics", 63855, 63904),
    (63817, "UnionOmega_Guide_AdvBiology", 63852, 63901),
];

#[cfg(test)]
mod tests {
    use super::super::super::super::engine_loader::load_chain_expansions_for_test;
    use super::*;
    use crate::test_support::require_db_or_skip;
    use cimmeria_content_engine::chain::ChainEngine;

    /// The four guide chains with every trigger expansion registered: each
    /// chain has a plain-spot and an Adv.-spot `interact_tag` trigger, and
    /// `load_single_chain_for_test` would keep only the first.
    async fn guide_engine(pool: &sqlx::PgPool) -> ChainEngine {
        let mut engine = ChainEngine::new();
        for chain_id in [63852, 63853, 63854, 63855] {
            let expansions = load_chain_expansions_for_test(pool, chain_id)
                .await
                .unwrap_or_else(|e| panic!("DB query for chain {chain_id}: {e}"));
            assert_eq!(
                expansions.len(),
                2,
                "chain {chain_id} must hold two interact_tag triggers (plain and Adv. guide)"
            );
            for chain in expansions {
                engine.register_chain(chain);
            }
        }
        engine
    }

    /// Every guide click is handled by its own chain and speaks that chain's
    /// screen, once, to the player.
    #[tokio::test]
    async fn live_db_each_omega_guide_speaks_its_specialty() {
        let pool = require_db_or_skip!();
        let spawns: Vec<i32> = GUIDES.iter().map(|g| g.0).collect();
        let (mut mgr, ids) = hub_with(&pool, &spawns, 63810).await;
        let engine = guide_engine(&pool).await;
        for (spawn, tag, _chain, screen) in GUIDES {
            let guide = *ids
                .get(&spawn)
                .unwrap_or_else(|| panic!("spawn {spawn} must map to an entity"));
            let (handled, sent) = click(&mut mgr, &engine, tag, guide).await;
            assert!(
                handled,
                "spawn {spawn} ({tag}): a click must be claimed by its guide chain"
            );
            let expected = bark_bytes(SPEAKER, &screen_text(&pool, screen).await);
            let talks: Vec<_> = sent
                .iter()
                .filter(|(entity, method, _)| {
                    *entity == PLAYER && *method == ON_PLAYER_COMMUNICATION
                })
                .collect();
            assert_eq!(
                talks.len(),
                1,
                "spawn {spawn} ({tag}) must send exactly one line to the player"
            );
            assert_eq!(
                talks[0].2, expected,
                "spawn {spawn} ({tag}) must speak screen {screen}"
            );
        }
    }

    /// Template 63829 is a trainer-bit NPC on moniker 7952 with no list, so
    /// the click reaches the chain, and its eight spawns carry the guide tags.
    #[tokio::test]
    async fn live_db_omega_guide_template_reaches_the_chain() {
        let pool = require_db_or_skip!();
        let row: (
            i64,
            Option<i32>,
            Option<i32>,
            Option<i32>,
            Option<i32>,
            Option<i32>,
            Option<i32>,
        ) = sqlx::query_as(
            "SELECT interaction_type, name_id, trainer_ability_list_id, buy_item_list, \
                 sell_item_list, repair_item_list, recharge_item_list \
                 FROM resources.entity_templates WHERE template_id = $1",
        )
        .bind(TEMPLATE)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("template {TEMPLATE} must be seeded: {e}"));
        assert_eq!(
            row,
            (128, Some(7952), None, None, None, None, None),
            "template {TEMPLATE} must be TRAINER (128) on name 7952 with no trainer or item list"
        );

        let spawned: Vec<(i32, String)> = sqlx::query_as(
            "SELECT spawn_id, tag FROM resources.spawnlist WHERE template_id = $1 ORDER BY spawn_id",
        )
        .bind(TEMPLATE)
        .fetch_all(&pool)
        .await
        .expect("spawnlist rows for template 63829");
        let expected: Vec<(i32, String)> = GUIDES
            .iter()
            .map(|(spawn, tag, _, _)| (*spawn, tag.to_string()))
            .collect();
        assert_eq!(
            spawned, expected,
            "the spawns of template {TEMPLATE} must be 63810-63817 with their guide tags"
        );
    }

    /// Screens 63901-63904 sit on dialog 63900 at index screen - 63901, speak
    /// as speaker 0, and carry no internal label in their text.
    #[tokio::test]
    async fn live_db_omega_guide_lines_are_screens_0_to_3() {
        let pool = require_db_or_skip!();
        for screen in 63901..=63904 {
            let (dialog_id, index, speaker_id): (i32, i32, Option<i32>) = sqlx::query_as(
                "SELECT dialog_id, index, speaker_id FROM resources.dialog_screens WHERE screen_id = $1",
            )
            .bind(screen)
            .fetch_one(&pool)
            .await
            .unwrap_or_else(|e| panic!("dialog screen {screen} must be seeded: {e}"));
            assert_eq!(
                (dialog_id, index, speaker_id),
                (63900, screen - 63901, Some(0)),
                "screen {screen} must be index {} on dialog 63900 with speaker 0",
                screen - 63901
            );
            let text = screen_text(&pool, screen).await.to_lowercase();
            for word in ["cimmeria", "authored", "reconstruction", "placeholder"] {
                assert!(
                    !text.contains(word),
                    "screen {screen} text must not contain '{word}'"
                );
            }
        }
    }
}
