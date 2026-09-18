//! Mission 638 (Speak to Prisoner 329) — archetype-gated dialog routing.
//!
//! Chains 1011 (non-Jaffa) and 1012 (Jaffa) had their `add_dialog_set`
//! IDs swapped relative to the archetype condition (issue #216), so a
//! Tau'ri / Human player saw the Jaffa "My symbiote will cure me"
//! dialog and vice versa.
//!
//! The shared helper [`assert_region_enter_resolves_dialog_set`] is the
//! reusable shape for any future archetype-gated dialog regression
//! guard: pass a chain id, an archetype value, and the dialog_set_map id
//! you expect the chain to add. Every other archetype-routed dialog
//! pair in the seed (1056/1057, future siblings) can be regression-
//! guarded by adding two calls.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::{build_engine, load_single_chain_for_test};
use crate::test_support::require_db_or_skip;

/// Load `chain_id`, fire a `RegionEnter('Castle_Cellblock.Region2')`
/// event with the supplied `archetype` (and `mission_638_status =
/// 'not_active'` so the mission-gate condition passes), and assert
/// that the resolved actions contain exactly one `AddDialogSet` whose
/// `dialog_set_id` matches `expected_dialog_set_map`.
///
/// `expected_dialog_set_map` is the `dialog_set_maps.dialog_set_map_id`
/// (i.e., the `target_id` column on the `add_dialog_set` action row),
/// not the `dialog_set_id` it groups under. The Rust field is named
/// `dialog_set_id` for historical reasons; the value stored is the
/// per-archetype map row.
async fn assert_region_enter_resolves_dialog_set(
    chain_id: i32,
    archetype: i32,
    expected_dialog_set_map: i32,
) {
    use cimmeria_content_engine::actions::Action;

    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "region_key".to_string(),
        serde_json::json!("Castle_Cellblock.Region2"),
    );
    ctx.set_param(
        "mission_638_status".to_string(),
        serde_json::json!("not_active"),
    );
    ctx.set_param("archetype".to_string(), serde_json::json!(archetype));

    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);

    let actual: Vec<i32> = resolved
        .actions
        .iter()
        .filter_map(|(_, action)| match action {
            Action::AddDialogSet { dialog_set_id, .. } => Some(*dialog_set_id),
            _ => None,
        })
        .collect();

    assert_eq!(
        actual,
        vec![expected_dialog_set_map],
        "chain {chain_id} with archetype={archetype} must add exactly \
         dialog_set_map {expected_dialog_set_map}; got {actual:?}"
    );
}

/// Load `chain_id` and fire `RegionEnter('Castle_Cellblock.Region2')`
/// with the supplied archetype. Assert that NO `AddDialogSet` actions
/// resolve — used as the negative pin (the wrong archetype must not
/// match the chain's gate).
async fn assert_region_enter_does_not_resolve(chain_id: i32, archetype: i32) {
    use cimmeria_content_engine::actions::Action;

    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "region_key".to_string(),
        serde_json::json!("Castle_Cellblock.Region2"),
    );
    ctx.set_param(
        "mission_638_status".to_string(),
        serde_json::json!("not_active"),
    );
    ctx.set_param("archetype".to_string(), serde_json::json!(archetype));

    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let dialog_set_actions: Vec<i32> = resolved
        .actions
        .iter()
        .filter_map(|(_, action)| match action {
            Action::AddDialogSet { dialog_set_id, .. } => Some(*dialog_set_id),
            _ => None,
        })
        .collect();

    assert!(
        dialog_set_actions.is_empty(),
        "chain {chain_id} must NOT match for archetype={archetype} — \
         the archetype condition is what gates branch selection. \
         Got AddDialogSet ids: {dialog_set_actions:?}"
    );
}

/// Tau'ri / Human player (archetype Soldier=1) entering Region2 must get
/// the Human "Free Prisoner 329" dialog set (dialog_set_map 2794 →
/// dialog 2300), not the Jaffa one. Pinned shape: chain 1011's
/// archetype-neq-8 condition routes here.
#[tokio::test]
async fn chain_1011_routes_non_jaffa_to_human_dialog_set() {
    const ARCHETYPE_SOLDIER: i32 = 1;
    const HUMAN_FREE_PRISONER_DIALOG_SET_MAP: i32 = 2794;
    assert_region_enter_resolves_dialog_set(
        1011,
        ARCHETYPE_SOLDIER,
        HUMAN_FREE_PRISONER_DIALOG_SET_MAP,
    )
    .await;
}

/// Jaffa player (archetype 8) entering Region2 must get the Jaffa
/// "Free Prisoner 329" dialog set (dialog_set_map 5866 → dialog 5021,
/// the symbiote dialog), not the Human one. Pinned shape: chain 1012's
/// archetype-eq-8 condition routes here.
#[tokio::test]
async fn chain_1012_routes_jaffa_to_jaffa_dialog_set() {
    const ARCHETYPE_JAFFA: i32 = 8;
    const JAFFA_FREE_PRISONER_DIALOG_SET_MAP: i32 = 5866;
    assert_region_enter_resolves_dialog_set(
        1012,
        ARCHETYPE_JAFFA,
        JAFFA_FREE_PRISONER_DIALOG_SET_MAP,
    )
    .await;
}

/// Negative pin: chain 1011 (non-Jaffa branch) must NOT match when the
/// player is Jaffa. Without the archetype condition both chains would
/// fire and the prisoner would offer both dialog sets at once.
#[tokio::test]
async fn chain_1011_does_not_match_jaffa_archetype() {
    const ARCHETYPE_JAFFA: i32 = 8;
    assert_region_enter_does_not_resolve(1011, ARCHETYPE_JAFFA).await;
}

/// Negative pin: chain 1012 (Jaffa branch) must NOT match when the
/// player is non-Jaffa. Mirror of the above, on the other side of the
/// archetype gate.
#[tokio::test]
async fn chain_1012_does_not_match_non_jaffa_archetype() {
    const ARCHETYPE_SOLDIER: i32 = 1;
    assert_region_enter_does_not_resolve(1012, ARCHETYPE_SOLDIER).await;
}

/// Full-seed regression guard for audit.md defect B1 (the purged
/// `space_castle_cellblock_chains.sql` auto-export). Unlike the helpers
/// above, which load ONE named chain and so can never observe a second,
/// unrelated chain double-firing on the same trigger, this loads the
/// *entire* seeded content engine via [`build_engine`] — the same
/// assembly path the live server uses — so a duplicate chain anywhere in
/// the DB shows up in the resolved action count.
///
/// Before the purge, a Jaffa entering Region2 matched curated chain 1012
/// (`AddDialogSet 5866` x1, `AcceptMission 638` x1) *and* the auto-export's
/// chains 5004 (`AddDialog 5866` x1, `AcceptMission 638` x1) and 5005
/// (`AddDialog 5866` x4, `AddDialog 2794` x1, `AcceptMission 638` x2) —
/// six dialog-bind actions and four accepts total. This test asserts
/// exactly one of each, which fails on the pre-purge tree and passes
/// once `space_castle_cellblock_chains.sql` is deleted.
#[tokio::test]
async fn jaffa_region2_entry_resolves_exactly_one_dialog_bind_and_accept() {
    use cimmeria_content_engine::actions::Action;

    const ARCHETYPE_JAFFA: i32 = 8;
    const JAFFA_DIALOG_SET_MAP: i32 = 5866;

    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "region_key".to_string(),
        serde_json::json!("Castle_Cellblock.Region2"),
    );
    ctx.set_param(
        "mission_638_status".to_string(),
        serde_json::json!("not_active"),
    );
    ctx.set_param("archetype".to_string(), serde_json::json!(ARCHETYPE_JAFFA));

    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);

    // Count both `AddDialogSet` (curated seed's action verb) and
    // `AddDialog` (the auto-export's action verb) for dialog_set_id
    // 5866 — the purged duplicate used the other verb, so a count that
    // only checked one variant would miss the double-fire.
    let dialog_binds: Vec<_> = resolved
        .actions
        .iter()
        .filter(|(_, action)| match action {
            Action::AddDialogSet { dialog_set_id, .. } => *dialog_set_id == JAFFA_DIALOG_SET_MAP,
            Action::AddDialog { dialog_set_id, .. } => *dialog_set_id == JAFFA_DIALOG_SET_MAP,
            _ => false,
        })
        .collect();
    assert_eq!(
        dialog_binds.len(),
        1,
        "Jaffa entering Region2 must resolve exactly one dialog-set bind for \
         5866 (AddDialogSet or AddDialog); got {} — duplicate chains from the \
         purged auto-export are still in the seed. Actions: {:?}",
        dialog_binds.len(),
        resolved.actions,
    );

    let accepts: Vec<_> = resolved
        .actions
        .iter()
        .filter(|(_, action)| matches!(action, Action::AcceptMission { mission_id: 638 }))
        .collect();
    assert_eq!(
        accepts.len(),
        1,
        "Jaffa entering Region2 must resolve exactly one AcceptMission(638); \
         got {} — duplicate chains from the purged auto-export are still in \
         the seed. Actions: {:?}",
        accepts.len(),
        resolved.actions,
    );
}

/// Sibling of the Jaffa guard above: a Human (non-Jaffa) player entering
/// Region2 must resolve only the Human dialog-set bind (2794), never the
/// Jaffa one (5866). The purged auto-export never had a non-Jaffa branch
/// at all (chains 5004/5005 both gated `archetype eq 8`), so this side
/// of the bug was never double-firing — this test pins that it stays
/// correct now that the duplicate file is gone.
#[tokio::test]
async fn human_region2_entry_resolves_only_human_dialog_bind() {
    use cimmeria_content_engine::actions::Action;

    const ARCHETYPE_SOLDIER: i32 = 1;
    const HUMAN_DIALOG_SET_MAP: i32 = 2794;
    const JAFFA_DIALOG_SET_MAP: i32 = 5866;

    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "region_key".to_string(),
        serde_json::json!("Castle_Cellblock.Region2"),
    );
    ctx.set_param(
        "mission_638_status".to_string(),
        serde_json::json!("not_active"),
    );
    ctx.set_param(
        "archetype".to_string(),
        serde_json::json!(ARCHETYPE_SOLDIER),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let dialog_set_ids: Vec<i32> = resolved
        .actions
        .iter()
        .filter_map(|(_, action)| match action {
            Action::AddDialogSet { dialog_set_id, .. } => Some(*dialog_set_id),
            Action::AddDialog { dialog_set_id, .. } => Some(*dialog_set_id),
            _ => None,
        })
        .collect();
    assert_eq!(
        dialog_set_ids,
        vec![HUMAN_DIALOG_SET_MAP],
        "Human entering Region2 must resolve exactly one dialog-set bind, \
         2794, and never the Jaffa one ({JAFFA_DIALOG_SET_MAP}); got {dialog_set_ids:?}",
    );
}
