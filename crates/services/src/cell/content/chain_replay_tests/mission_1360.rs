//! Mission 1360 — Frost's Letter (C04,
//! docs/analysis/castle-cellblock-rebuild/work-packets.md#c04).
//!
//! D-CB03 (answered: accept). `ArmYourself.py` grants the letter (item
//! 3730) but never touched a mission for it — this is new content the spec
//! calls optional. Chain 1121 is a sibling to chain 1003 (identical
//! trigger + step gate: `dialog_open(3995)` while mission 622's step 2113
//! is active) that accepts mission 1360 alongside chain 1003's letter
//! grant, rather than folding the accept into chain 1003 itself (which
//! would force an unrelated `mission_status 1360` condition onto Frost's
//! item-grant/Guard-unlock/step-advance actions).
//!
//! - Chain 1121 (`dialog_open(3995)`, `step_status 622/2113 eq active` AND
//!   `mission_status 1360 eq not_active`): accepts mission 1360.
//!
//! Step 4037 ("Find a way to get Cpl. Frost's Letter to his family") stays
//! active for the rest of the zone; step 4038 ("Give Cpl. Frost's Letter
//! to Col. Marsh") is Castle-side and out of this packet's scope. Mission
//! persistence across the eventual Cellblock -> Castle hop is pinned by the
//! live-DB round-trip test `frosts_letter_accept_round_trips_cell_to_base_to_db`
//! in `crates/services/src/base/world_entry/methods/missions/tests.rs`, not
//! here — this file only pins chain *resolution*.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Fire `dialog_open(3995)` against chain 1121 with the given
/// `step_2113_status` (mission 622's opening step) and `mission_1360_status`,
/// and assert whether it resolves an `AcceptMission { mission_id: 1360 }`.
async fn assert_frost_dialog_resolves(
    step_2113_status: &str,
    mission_1360_status: &str,
    should_fire: bool,
) {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1121)
        .await
        .expect("DB query for chain 1121 must succeed")
        .expect("chain 1121 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("dialog_id".to_string(), serde_json::json!(3995));
    ctx.set_param(
        "mission_622_step_2113_status".to_string(),
        serde_json::json!(step_2113_status),
    );
    ctx.set_param(
        "mission_1360_status".to_string(),
        serde_json::json!(mission_1360_status),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::DialogOpen,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let accepts_1360 = resolved
        .actions
        .iter()
        .filter(|(id, a)| *id == 1121 && matches!(a, Action::AcceptMission { mission_id: 1360 }))
        .count();

    if should_fire {
        assert_eq!(
            accepts_1360, 1,
            "chain 1121 must resolve exactly one AcceptMission(1360) with \
             step_2113_status={step_2113_status:?}, mission_1360_status={mission_1360_status:?}; \
             got {accepts_1360} matches in {:?}",
            resolved.actions,
        );
    } else {
        assert_eq!(
            accepts_1360, 0,
            "chain 1121 must NOT resolve AcceptMission(1360) with \
             step_2113_status={step_2113_status:?}, mission_1360_status={mission_1360_status:?}; \
             got {accepts_1360} matches in {:?}",
            resolved.actions,
        );
    }
}

/// Positive: first Frost interaction (step 2113 active, mission 1360 never
/// accepted) — chain 1121 accepts 1360 exactly once. Pins T03's "1360
/// active after loot" acceptance criterion at the chain-resolution level.
#[tokio::test]
async fn chain_1121_accepts_1360_on_first_frost_loot() {
    assert_frost_dialog_resolves("active", "not_active", true).await;
}

/// Negative (re-loot guard): once step 2113 has advanced to 80623 (Frost
/// already searched — chain 1003 owns that advance), a second Frost
/// interaction must NOT re-accept 1360. Mirrors chain 1003's own
/// `chain_1003_does_not_fire_after_advancing_to_80623` guard — both chains
/// share the same step gate, so a second Frost dialog_open resolves neither.
#[tokio::test]
async fn chain_1121_does_not_refire_after_advancing_to_80623() {
    assert_frost_dialog_resolves("completed", "not_active", false).await;
}

/// Negative (mission offer guard): even if the step gate were somehow
/// still open, a second Frost interaction while 1360 is already ACTIVE
/// must not re-accept it. `cell::missions::lifecycle::accept_mission`'s
/// server-side offer guard would refuse this authoritatively regardless,
/// but the chain-level `mission_status 1360 eq not_active` condition
/// (content-chains.instructions.md "Mission grants must gate on
/// not_active") is the first line of defense pinned here.
#[tokio::test]
async fn chain_1121_does_not_refire_once_1360_already_active() {
    assert_frost_dialog_resolves("active", "active", false).await;
}

/// Negative: once mission 1360 is COMPLETED, chain 1121 must not re-fire
/// either — same offer-guard shape as the ACTIVE case above, covering the
/// other non-`not_active` status the mission lifecycle can be in.
#[tokio::test]
async fn chain_1121_does_not_refire_once_1360_completed() {
    assert_frost_dialog_resolves("active", "completed", false).await;
}
