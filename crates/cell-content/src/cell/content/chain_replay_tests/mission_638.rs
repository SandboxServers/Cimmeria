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
async fn live_db_chain_1011_routes_non_jaffa_to_human_dialog_set() {
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
async fn live_db_chain_1012_routes_jaffa_to_jaffa_dialog_set() {
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
async fn live_db_chain_1011_does_not_match_jaffa_archetype() {
    const ARCHETYPE_JAFFA: i32 = 8;
    assert_region_enter_does_not_resolve(1011, ARCHETYPE_JAFFA).await;
}

/// Negative pin: chain 1012 (Jaffa branch) must NOT match when the
/// player is non-Jaffa. Mirror of the above, on the other side of the
/// archetype gate.
#[tokio::test]
async fn live_db_chain_1012_does_not_match_non_jaffa_archetype() {
    const ARCHETYPE_SOLDIER: i32 = 1;
    assert_region_enter_does_not_resolve(1012, ARCHETYPE_SOLDIER).await;
}

/// A relog recreates the cell entity with an empty available_interactions map.
/// The Region2 accept edge has already passed, so the active mission must
/// restore exactly its own archetype's Prisoner 329 dialog on player_loaded.
/// Completed mission 638 must not regain the topic.
#[tokio::test]
async fn live_db_mission_638_login_restores_only_active_archetype_dialog() {
    use cimmeria_content_engine::actions::Action;

    let pool = require_db_or_skip!();
    let mut engine = ChainEngine::new();
    for chain_id in [1026, 1027] {
        engine.register_chain(
            load_single_chain_for_test(&pool, chain_id)
                .await
                .unwrap_or_else(|e| panic!("DB query for chain {chain_id} failed: {e}"))
                .unwrap_or_else(|| panic!("login restore chain {chain_id} must exist")),
        );
    }

    for (archetype, status, expected) in [
        (1, "active", vec![(1026, 2794)]),
        (8, "active", vec![(1027, 5866)]),
        (1, "completed", vec![]),
        (8, "completed", vec![]),
        // Not yet accepted: Region2 entry (chains 1011/1012) binds it, not login.
        (1, "not_active", vec![]),
        (8, "not_active", vec![]),
    ] {
        let mut ctx = ExecutionContext::new();
        ctx.set_param(
            "world_name".to_string(),
            serde_json::json!("Castle_CellBlock"),
        );
        ctx.set_param("mission_638_status".to_string(), serde_json::json!(status));
        ctx.set_param("archetype".to_string(), serde_json::json!(archetype));
        let event = TriggerEvent {
            trigger_type: TriggerType::PlayerLoaded,
            source_entity: None,
            target_entity: None,
            params: ctx.params.clone(),
        };
        let resolved = engine.resolve_event(&event, &ctx);
        let bindings: Vec<_> = resolved
            .actions
            .iter()
            .filter_map(|(chain_id, action)| match action {
                Action::AddDialogSet {
                    dialog_set_id,
                    slot: 17,
                    ..
                } => Some((*chain_id, *dialog_set_id)),
                _ => None,
            })
            .collect();
        assert_eq!(
            bindings, expected,
            "archetype={archetype}, mission_638_status={status}"
        );
    }

    // The world key gates the restore: an active 638 loading into another
    // world must not get the Cellblock dialog.
    let mut ctx = ExecutionContext::new();
    ctx.set_param("world_name".to_string(), serde_json::json!("SGC_W1"));
    ctx.set_param(
        "mission_638_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param("archetype".to_string(), serde_json::json!(1));
    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    assert!(
        engine.resolve_event(&event, &ctx).actions.is_empty(),
        "login restore must not fire outside Castle_CellBlock"
    );
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
async fn live_db_jaffa_region2_entry_resolves_exactly_one_dialog_bind_and_accept() {
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
async fn live_db_human_region2_entry_resolves_only_human_dialog_bind() {
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

/// Chain 1028: the cell door button's Livewire bit (mask 256) is set only on
/// the 1014/1015 dialog edge and lives in cell memory. A relog at step 2115
/// must re-paint it; step 2114 and steps after 2115 must not.
#[tokio::test]
async fn live_db_mission_638_login_restores_door_button_bit_only_at_step_2115() {
    use cimmeria_content_engine::actions::Action;

    let pool = require_db_or_skip!();
    let mut engine = ChainEngine::new();
    engine.register_chain(
        load_single_chain_for_test(&pool, 1028)
            .await
            .expect("DB query for chain 1028 must succeed")
            .expect("door button restore chain 1028 must exist"),
    );

    for (params, expect_restore) in [
        (vec![("mission_638_step_2115_status", "active")], true),
        (vec![("mission_638_step_2114_status", "active")], false),
        (
            vec![
                ("mission_638_step_2115_status", "completed"),
                ("mission_638_step_2116_status", "active"),
            ],
            false,
        ),
    ] {
        let mut ctx = ExecutionContext::new();
        ctx.set_param(
            "world_name".to_string(),
            serde_json::json!("Castle_CellBlock"),
        );
        for (key, value) in &params {
            ctx.set_param((*key).to_string(), serde_json::json!(*value));
        }
        let event = TriggerEvent {
            trigger_type: TriggerType::PlayerLoaded,
            source_entity: None,
            target_entity: None,
            params: ctx.params.clone(),
        };
        let resolved = engine.resolve_event(&event, &ctx);
        let actions: Vec<&Action> = resolved
            .actions
            .iter()
            .filter(|(id, _)| *id == 1028)
            .map(|(_, action)| action)
            .collect();
        if expect_restore {
            assert!(
                matches!(
                    actions.as_slice(),
                    [Action::SetInteractionType { entity_tag, operation, mask: 256 }]
                    if entity_tag == "329_CellDoorButton" && operation == "|"
                ),
                "{params:?}: expected exactly the door button bit; got {actions:?}"
            );
        } else {
            assert!(
                actions.is_empty(),
                "{params:?}: chain 1028 must not fire; got {actions:?}"
            );
        }
    }
}
