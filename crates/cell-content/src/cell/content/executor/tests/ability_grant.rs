//! `Action::GrantAbility` (Class Start v6, CS-01a): the non-GM content
//! grant forwards one `ContentGrantAbilities` to the base, ungated by
//! access level but gated by the grant's own archetype list.
//!
//! Revert proofs: drop the dispatch arm and
//! `a_player_grant_is_forwarded_with_its_provenance` sees no message; add a
//! GM gate and the access-level-0 player gets nothing; drop the archetype
//! check in `ability_grant::run` and
//! `a_grant_for_another_class_sends_nothing_and_no_line` sees the message.

use cimmeria_content_engine::actions::{AbilityGrant, AbilityGrantKind};

use super::*;
use crate::cell::messages::ContentGrantAbilities;
use crate::test_support::LogCapture;

const PLAYER: u32 = 31;
const PLAYER_ID: i32 = 4343;
const NPC: u32 = 100_031;
const CHAIN: i64 = 7_301;
/// Soldier.
const ARCHETYPE: i32 = 1;

fn world() -> SpaceManager {
    let mut mgr = make_space_mgr();
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    p.archetype_id = Some(ARCHETYPE);
    // An ordinary player: the grant is not GM-gated.
    p.access_level = 0;
    mgr.connect_entity(PLAYER);
    mgr.spawn_npc(NPC, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr
}

fn grant(archetypes: Vec<i32>) -> AbilityGrant {
    AbilityGrant {
        ability_ids: vec![592, 594],
        source_kind: AbilityGrantKind::Tutorial,
        source_id: Some(1559),
        archetypes,
    }
}

async fn fire(
    mgr: &mut SpaceManager,
    entity_id: u32,
    player_id: i32,
    grant: AbilityGrant,
) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(16);
    let resolved = ResolvedActions {
        action_delays: Vec::new(),
        params: Default::default(),
        actions: vec![(CHAIN, Action::GrantAbility(grant))],
    };
    execute_actions(
        resolved,
        entity_id,
        player_id,
        &tx,
        mgr,
        &ChainEngine::new(),
    )
    .await;
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn forwarded(msgs: &[CellToBaseMsg]) -> Vec<&ContentGrantAbilities> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::ContentGrantAbilities(g) => Some(g),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn a_player_grant_is_forwarded_with_its_provenance() {
    let mut mgr = world();
    let msgs = fire(&mut mgr, PLAYER, PLAYER_ID, grant(vec![ARCHETYPE, 7])).await;
    assert_eq!(
        forwarded(&msgs),
        vec![&ContentGrantAbilities {
            entity_id: PLAYER,
            player_id: PLAYER_ID,
            account_id: None,
            chain_id: CHAIN,
            ability_ids: vec![592, 594],
            source_kind: AbilityGrantKind::Tutorial,
            source_id: Some(1559),
            archetypes: vec![ARCHETYPE, 7],
        }]
    );
    // The cell writes nothing of its own: the known set waits for the
    // base's answer.
    assert!(!mgr.get_entity(PLAYER).unwrap().abilities.has_ability(592));
}

/// **Guard (review F2): another class gets nothing.** No base message, no
/// chat line, one `refused` row with `reason=archetype_mismatch`.
#[tokio::test]
async fn a_grant_for_another_class_sends_nothing_and_no_line() {
    let capture = LogCapture::install();
    let mut mgr = world();
    let msgs = fire(&mut mgr, PLAYER, PLAYER_ID, grant(vec![2, 3])).await;
    assert!(msgs.is_empty(), "no base message and no line");
    assert!(
        capture
            .find_event(
                tracing::Level::INFO,
                "grant_ability refused: the player's archetype",
                "archetype_mismatch",
            )
            .is_some(),
        "the refusal is one row with reason=archetype_mismatch"
    );
}

/// A player whose archetype is not loaded yet never matches a list.
#[tokio::test]
async fn a_player_with_no_archetype_never_matches_a_list() {
    let mut mgr = world();
    mgr.get_entity_mut(PLAYER).unwrap().archetype_id = None;
    let msgs = fire(&mut mgr, PLAYER, PLAYER_ID, grant(vec![ARCHETYPE])).await;
    assert!(forwarded(&msgs).is_empty());
}

#[tokio::test]
async fn a_grant_on_an_npc_sends_nothing_and_logs_the_refusal() {
    let capture = LogCapture::install();
    let mut mgr = world();
    let msgs = fire(&mut mgr, NPC, 0, grant(vec![])).await;
    assert!(
        forwarded(&msgs).is_empty(),
        "an NPC has no character to grant to"
    );
    assert!(
        capture
            .find_event(
                tracing::Level::WARN,
                "grant_ability fired for an entity that is not a loaded player",
                "not_a_player",
            )
            .is_some(),
        "the refusal is one WARN row with reason=not_a_player"
    );
}
