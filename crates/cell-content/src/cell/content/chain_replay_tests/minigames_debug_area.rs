//! Debug Area minigame terminals (MG-74, D-MG23), seeded in
//! [`db/resources/Content/Seed/debug_area_minigames_chains.sql`] (launchers
//! 24050-24055, win line 24056, loss line 24057), with their templates
//! 1600-1605 in `entity_templates_debug_area_minigames.sql` and their spawns
//! 14100-14105 in `spawnlist_debug_area_minigames.sql`.
//!
//! Six reusable GM test fixtures: no mission, no step gate, so each launcher
//! fires on its tag alone. The harness is [`super::livewire_pairs`]'s, reused
//! for both the resolve and the executor halves.

use cimmeria_content_engine::actions::Action;
use sqlx::PgPool;

use super::super::engine_loader::load_single_chain_for_test;
use super::livewire_pairs::{execute_and_drain_starts, resolve_interact};
use crate::test_support::require_db_or_skip;

/// (launcher chain, game, tag), one row per terminal in spawn order.
const TERMINALS: [(i32, &str, &str); 6] = [
    (24050, "CrystalGame", "DebugArea_Minigame_CrystalGame"),
    (24051, "Alignment", "DebugArea_Minigame_Alignment"),
    (24052, "GoauldCrystals", "DebugArea_Minigame_GoauldCrystals"),
    (
        24053,
        "ConverseBasicHumanoid",
        "DebugArea_Minigame_ConverseBasicHumanoid",
    ),
    (24054, "Livewire", "DebugArea_Minigame_Livewire"),
    (24055, "Hack", "DebugArea_Minigame_Hack"),
];
const WIN_CHAIN: i64 = 24056;
const LOSS_CHAIN: i64 = 24057;
/// The services plaza's Livewire terminal (chains 13004-13005), which no
/// debug-area minigame launcher may answer.
const PLAZA_TAG: &str = "DebugArea_LivewireTerminal";
/// Entity id of the player firing each chain (the same id
/// `livewire_pairs` stages), which `execute_and_drain_starts` sends with.
const PLAYER_EID: u32 = 7301;

/// The actions chain `chain_id` holds in the seed.
async fn chain_actions(pool: &PgPool, chain_id: i32) -> Vec<Action> {
    load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"))
        .actions
}

/// Clicking each terminal starts its own game, difficulty 1, with the win
/// line 24056 and the loss line 24057 as its results.
#[tokio::test]
async fn live_db_each_debug_area_terminal_starts_its_game() {
    let pool = require_db_or_skip!();
    for (chain, game, tag) in TERMINALS {
        let actions = resolve_interact(&pool, chain, tag, &[]).await;
        let starts: Vec<_> = actions
            .iter()
            .filter_map(|(_, action)| match action {
                Action::StartMinigame {
                    minigame_type,
                    difficulty,
                    on_victory_chains,
                    on_failure_chains,
                } => Some((
                    minigame_type.clone(),
                    *difficulty,
                    on_victory_chains.clone(),
                    on_failure_chains.clone(),
                )),
                _ => None,
            })
            .collect();
        assert_eq!(
            starts,
            vec![(game.to_string(), 1, vec![WIN_CHAIN], vec![LOSS_CHAIN])],
            "chain {chain} on tag {tag} must resolve exactly one \
             StartMinigame({game}, difficulty 1, victory [{WIN_CHAIN}], failure [{LOSS_CHAIN}]); \
             got {actions:?}",
        );
    }
}

/// Each launcher resolves nothing for the plaza's Livewire tag or for the
/// next terminal's tag, so no click is shared between two terminals.
#[tokio::test]
async fn live_db_debug_area_terminal_ignores_other_tags() {
    let pool = require_db_or_skip!();
    for (i, (chain, _, _)) in TERMINALS.iter().enumerate() {
        let next_tag = TERMINALS[(i + 1) % TERMINALS.len()].2;
        for other in [PLAZA_TAG, next_tag] {
            let actions = resolve_interact(&pool, *chain, other, &[]).await;
            assert!(
                actions.is_empty(),
                "chain {chain} must resolve nothing for tag {other}; got {actions:?}",
            );
        }
    }
}

/// Each launcher's resolved start reaches the base as exactly one
/// `StartMinigame` for its game, difficulty 1, victory chain 24056.
#[tokio::test]
async fn live_db_debug_area_terminal_reaches_base() {
    let pool = require_db_or_skip!();
    for (chain, game, tag) in TERMINALS {
        let actions = resolve_interact(&pool, chain, tag, &[]).await;
        let sends = execute_and_drain_starts(actions).await;
        assert_eq!(
            sends,
            vec![(PLAYER_EID, game.to_string(), 1, vec![WIN_CHAIN])],
            "chain {chain} must emit exactly one CellToBaseMsg::StartMinigame for {game}; \
             got {sends:?}",
        );
    }
}

/// The result chains each bark their own line through `npc_bark`, the win
/// line on screen 200020 and the loss line on screen 200021.
#[tokio::test]
async fn live_db_debug_area_result_chains_bark() {
    let pool = require_db_or_skip!();
    let win = chain_actions(&pool, WIN_CHAIN as i32).await;
    assert_eq!(
        win,
        vec![Action::NpcBark {
            screen_id: 200020,
            speaker: "Terminal".into(),
            channel: 0,
        }],
        "chain {WIN_CHAIN} must bark the win line (screen 200020) and nothing else",
    );
    let loss = chain_actions(&pool, LOSS_CHAIN as i32).await;
    assert_eq!(
        loss,
        vec![Action::NpcBark {
            screen_id: 200021,
            speaker: "Terminal".into(),
            channel: 0,
        }],
        "chain {LOSS_CHAIN} must bark the loss line (screen 200021) and nothing else",
    );
}

/// The six terminal spawns, templates and dialog screens are seeded as
/// authored: spawns 14100-14105 on world 1300 with their tags and templates,
/// each template a Livewire-cursor minigame terminal with its display name,
/// and screens 200020 and 200021 holding the two result lines.
#[tokio::test]
async fn live_db_debug_area_terminals_are_seeded() {
    let pool = require_db_or_skip!();

    let spawns: Vec<(i32, Option<String>, i32, i32, bool, i64, Option<String>)> = sqlx::query_as(
        "SELECT s.spawn_id, s.tag, s.template_id, s.world_id, s.is_stationary, \
                    t.interaction_type, t.display_name \
             FROM resources.spawnlist s \
             JOIN resources.entity_templates t ON t.template_id = s.template_id \
             WHERE s.spawn_id BETWEEN 14100 AND 14109 \
             ORDER BY s.spawn_id",
    )
    .fetch_all(&pool)
    .await
    .expect("spawnlist and entity_templates must be queryable");
    let expected_spawns = [
        (
            14100,
            "DebugArea_Minigame_CrystalGame",
            1600,
            "Minigame: CrystalGame",
        ),
        (
            14101,
            "DebugArea_Minigame_Alignment",
            1601,
            "Minigame: Alignment",
        ),
        (
            14102,
            "DebugArea_Minigame_GoauldCrystals",
            1602,
            "Minigame: GoauldCrystals",
        ),
        (
            14103,
            "DebugArea_Minigame_ConverseBasicHumanoid",
            1603,
            "Minigame: ConverseBasicHumanoid",
        ),
        (
            14104,
            "DebugArea_Minigame_Livewire",
            1604,
            "Minigame: Livewire",
        ),
        (
            14105,
            "DebugArea_Minigame_Hack",
            1605,
            "Minigame: Hack (shell)",
        ),
    ];
    let got: Vec<(i32, Option<&str>, i32, bool, i64, Option<&str>)> = spawns
        .iter()
        .map(
            |(spawn, tag, template, world, stationary, cursor, display)| {
                (
                    *spawn,
                    tag.as_deref(),
                    *template,
                    *world == 1300 && *stationary,
                    *cursor,
                    display.as_deref(),
                )
            },
        )
        .collect();
    let want: Vec<(i32, Option<&str>, i32, bool, i64, Option<&str>)> = expected_spawns
        .iter()
        .map(|(spawn, tag, template, display)| {
            (*spawn, Some(*tag), *template, true, 256, Some(*display))
        })
        .collect();
    assert_eq!(
        got, want,
        "spawns 14100-14109 must be the six world-1300 stationary terminals, each on \
         interaction_type 256 with its display name",
    );

    let screens: Vec<(i32, i32, String)> = sqlx::query_as(
        "SELECT screen_id, dialog_id, text FROM resources.dialog_screens \
         WHERE screen_id IN (200020, 200021) ORDER BY screen_id",
    )
    .fetch_all(&pool)
    .await
    .expect("dialog_screens must be queryable");
    assert_eq!(
        screens,
        vec![
            (
                200020,
                60120,
                "Minigame round trip complete: the server recorded your win.".to_string(),
            ),
            (
                200021,
                60120,
                "Minigame round trip complete: the server recorded your loss.".to_string(),
            ),
        ],
        "screens 200020 and 200021 must hold the win and loss lines on dialog 60120",
    );
}
