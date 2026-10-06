//! `grant_ability` rows (Class Start v6, CS-01a): conversion, and the
//! all-or-nothing rule that a bad grant refuses its whole chain.
//!
//! Bug shapes: a `gm` kind from content (a grant the next GM reset would
//! take away), a malformed row that drops only the grant and runs the rest
//! of the chain, and an unknown ability id that loads and then writes a
//! known-ability id the client cannot draw. No database.

use std::collections::HashSet;

use super::super::*;
use crate::actions::{AbilityGrant, AbilityGrantKind, Action};

const CHAIN: i32 = 7_301;

fn chain_row(chain_id: i32) -> DbChainRow {
    DbChainRow {
        chain_id,
        description: Some("CS-01a grant fixture".to_string()),
        scope_type: "global".to_string(),
        scope_id: None,
        enabled: true,
        priority: 0,
    }
}

fn row(chain_id: i32, action_type: &str, params: serde_json::Value, sort: i32) -> DbActionRow {
    DbActionRow {
        chain_id,
        action_type: action_type.to_string(),
        target_id: None,
        target_key: None,
        params,
        delay_ms: 0,
        sort_order: sort,
    }
}

/// One grant row plus a following `grant_xp`, built into chains.
fn build(params: serde_json::Value) -> Vec<crate::chain::Chain> {
    build_chains_from_rows(
        vec![chain_row(CHAIN)],
        vec![],
        vec![],
        vec![
            row(CHAIN, "grant_ability", params, 0),
            row(CHAIN, "grant_xp", serde_json::json!({"amount": 10}), 1),
        ],
    )
}

#[test]
fn a_valid_grant_row_converts_with_its_kind_and_source() {
    let chains = build(serde_json::json!({
        "ability_ids": [592, 594],
        "source_kind": "tutorial",
        "source_id": 1559
    }));
    assert_eq!(chains.len(), 1);
    assert_eq!(
        chains[0].actions[0],
        Action::GrantAbility(AbilityGrant {
            ability_ids: vec![592, 594],
            source_kind: AbilityGrantKind::Tutorial,
            source_id: Some(1559),
            archetypes: vec![],
        })
    );
    assert_eq!(
        chains[0].action_delays,
        vec![0, 0],
        "delays stay index-aligned"
    );
}

#[test]
fn source_id_is_optional_and_every_content_kind_loads() {
    for (raw, kind) in [
        ("racial_core", AbilityGrantKind::RacialCore),
        ("signature", AbilityGrantKind::Signature),
        ("mission", AbilityGrantKind::Mission),
    ] {
        let chains = build(serde_json::json!({
            "ability_ids": [598], "source_kind": raw, "archetypes": [1, 1]
        }));
        assert!(chains.is_empty(), "a duplicated archetype refuses {raw}");
        let chains = build(serde_json::json!({
            "ability_ids": [598], "source_kind": raw, "archetypes": [1, 8]
        }));
        assert_eq!(
            chains[0].actions[0],
            Action::GrantAbility(AbilityGrant {
                ability_ids: vec![598],
                source_kind: kind,
                source_id: None,
                archetypes: vec![1, 8],
            })
        );
    }
}

/// **Guard (review F2): a class or race grant must name its archetypes.**
/// Without the list a `signature` or `racial_core` chain gated only by its
/// trigger reaches every class wherever the trigger sets no `archetype`.
#[test]
fn signature_and_racial_core_grants_require_archetypes() {
    for raw in ["signature", "racial_core"] {
        let chains = build(serde_json::json!({"ability_ids": [598], "source_kind": raw}));
        assert!(chains.is_empty(), "{raw} without archetypes must refuse");
    }
    for raw in ["tutorial", "mission"] {
        let chains = build(serde_json::json!({"ability_ids": [598], "source_kind": raw}));
        assert_eq!(chains.len(), 1, "{raw} may omit archetypes");
    }
}

#[test]
fn bad_archetype_lists_refuse_the_chain() {
    for archetypes in [
        serde_json::json!([]),
        serde_json::json!([0]),
        serde_json::json!([9]),
        serde_json::json!(["1"]),
        serde_json::json!(1),
    ] {
        let chains = build(serde_json::json!({
            "ability_ids": [598], "source_kind": "tutorial", "archetypes": archetypes
        }));
        assert!(chains.is_empty(), "archetypes {archetypes} must refuse");
    }
}

/// **Guard: a malformed grant refuses the chain, not just the row.** Each
/// case would otherwise load the chain with only its `grant_xp`.
#[test]
fn a_bad_grant_row_refuses_the_whole_chain() {
    for params in [
        // A GM grant from content: removed by the next GM reset, no credit.
        serde_json::json!({"ability_ids": [598], "source_kind": "gm"}),
        serde_json::json!({"ability_ids": [598], "source_kind": "trained"}),
        serde_json::json!({"ability_ids": [598]}),
        serde_json::json!({"ability_ids": [], "source_kind": "tutorial"}),
        serde_json::json!({"source_kind": "tutorial"}),
        serde_json::json!({"ability_ids": 598, "source_kind": "tutorial"}),
        serde_json::json!({"ability_ids": [598, 598], "source_kind": "tutorial"}),
        serde_json::json!({"ability_ids": [-1], "source_kind": "tutorial"}),
        serde_json::json!({"ability_ids": ["598"], "source_kind": "tutorial"}),
        serde_json::json!({"ability_ids": [598], "source_kind": "tutorial", "source_id": "M1"}),
    ] {
        let chains = build(params.clone());
        assert!(chains.is_empty(), "{params} must refuse the chain");
    }
}

#[test]
fn a_target_id_on_a_grant_row_refuses_the_chain() {
    let mut bad = row(
        CHAIN,
        "grant_ability",
        serde_json::json!({"ability_ids": [598], "source_kind": "signature"}),
        0,
    );
    bad.target_id = Some(598);
    let chains = build_chains_from_rows(vec![chain_row(CHAIN)], vec![], vec![], vec![bad]);
    assert!(chains.is_empty());
}

/// A refused chain does not take its neighbours with it.
#[test]
fn a_refused_chain_leaves_other_chains_loaded() {
    let chains = build_chains_from_rows(
        vec![chain_row(CHAIN), chain_row(CHAIN + 1)],
        vec![],
        vec![],
        vec![
            row(
                CHAIN,
                "grant_ability",
                serde_json::json!({"ability_ids": [598], "source_kind": "gm"}),
                0,
            ),
            row(CHAIN + 1, "grant_xp", serde_json::json!({"amount": 5}), 0),
        ],
    );
    assert_eq!(
        chains.iter().map(|c| c.id).collect::<Vec<_>>(),
        vec![i64::from(CHAIN + 1)]
    );
}

/// **Guard: an id with no ability row refuses the chain**, every trigger
/// expansion of it, and nothing else.
#[test]
fn unknown_ability_ids_refuse_the_chain_at_load() {
    use crate::loader::refuse_chains_with_unknown_abilities;

    let triggers = vec![
        DbTriggerRow {
            chain_id: CHAIN,
            event_type: "player_loaded".to_string(),
            event_key: Some("SGC_W1".to_string()),
            scope: "player".to_string(),
            once: false,
            sort_order: 0,
        },
        DbTriggerRow {
            chain_id: CHAIN,
            event_type: "mission_completed".to_string(),
            event_key: Some("1559".to_string()),
            scope: "player".to_string(),
            once: false,
            sort_order: 1,
        },
    ];
    let chains = build_chains_from_rows(
        vec![chain_row(CHAIN), chain_row(CHAIN + 1)],
        triggers,
        vec![],
        vec![
            row(
                CHAIN,
                "grant_ability",
                serde_json::json!({"ability_ids": [592, 999_999], "source_kind": "tutorial"}),
                0,
            ),
            row(
                CHAIN + 1,
                "grant_ability",
                serde_json::json!({"ability_ids": [592], "source_kind": "tutorial"}),
                0,
            ),
        ],
    );
    assert_eq!(chains.len(), 3, "two expansions of CHAIN plus CHAIN + 1");

    let known = HashSet::from([592]);
    let unread = refuse_chains_with_unknown_abilities(chains.clone(), None);
    assert!(
        unread.is_empty(),
        "with the ability table unreadable every grant chain is refused"
    );
    let kept = refuse_chains_with_unknown_abilities(chains, Some(&known));
    assert_eq!(
        kept.iter().map(|c| c.id).collect::<Vec<_>>(),
        vec![i64::from(CHAIN + 1)]
    );
}
