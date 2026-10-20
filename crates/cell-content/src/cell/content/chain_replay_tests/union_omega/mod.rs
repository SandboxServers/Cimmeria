//! Union Omega (worlds 18 `Omega_Site` and 80 `Omega_Site_CmdCenter`):
//! chain-replay guards for chains 63800-63999
//! (`docs/analysis/union-zone-planning/omega-work-packets.md`).
//!
//! Resolve-only (TESTING.md type 6): every verb the Omega chains use has a
//! shipped executor arm. The risks are the gates (archetype, world, step)
//! and the bind pairing. Assertions on Omega's own actions filter to
//! [`OMEGA_CHAIN_RANGE`]; the two-dialogs-on-one-click count stays
//! unfiltered, because the collision a sibling could add is a chain outside
//! this range.
//!
//! - [`arrival`]: UO-04, the beam (63801) and Modi (63810, 63811).
//! - [`human`]: UO-07, Jackson's induction (63820-63822).
//! - [`jaffa`]: UO-08, Bra'tac's induction (63830-63832).
//! - [`reoffer`]: UO-09, the re-offer after an abandon (63840-63845).
//! - [`executed`]: UO-13, the same chains run through the executor, so a
//!   relog or ring round trip that stacks a second bind (F3) shows.
//! - [`handoff`]: UO-13, the departure fixtures Beta's tests start from.
//! - [`auctioneer`]: UO-32, the auctioneer chain (63850).
//! - [`medic`]: UO-33, the medic chain (63851).
//! - [`guides`]: UO-35, the four guide chains (63852-63855).
//! - [`hub_placeholders`]: UO-36, the inn and battlemaster chains (63856-63857).
//! - [`services_common`]: UO-26, the checks every hub service chain shares.

mod arrival;
mod auctioneer;
mod executed;
mod guides;
pub(super) mod handoff;
mod hub_placeholders;
mod human;
mod jaffa;
mod medic;
mod reoffer;
pub(super) mod services_common;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::mission_1324::{
    ctx_for, display_dialog_count, fire_dialog_choice, fire_player_loaded, with_mission, with_step,
};
use crate::cell::content::build_engine;
use crate::test_support::require_db_or_skip;

const WORLD_HUB: i32 = 18;
const WORLD_CMD: i32 = 80;
const HUB_NAME: &str = "Omega_Site";
const CMD_NAME: &str = "Omega_Site_CmdCenter";
/// The Command Center ring pad's region id, the `teleport_in` key.
const REGION_CMD_PAD: i32 = 17;

const M_1655: i32 = 1655;
/// "Beam to the Command Center": O5645/O5646 optional, O5647/O5644 required (F21).
const S_4940: i32 = 4940;
/// "Deliver Hammond's letter".
const S_4942: i32 = 4942;
/// "(Optional) Speak to Modi".
const O_5645: i32 = 5645;
const M_1423: i32 = 1423;
const M_1500: i32 = 1500;

const DLG_MODI: i32 = 6168;
const DLG_JACKSON: i32 = 4641;
const DLG_BRATAC: i32 = 5025;

const ARCH_HUMAN: i64 = 1;
const ARCH_ASGARD: i64 = 5;
const ARCH_JAFFA: i64 = 7;

/// Chain-id window owned by the four `union_omega_*_chains.sql` files.
const OMEGA_CHAIN_RANGE: std::ops::RangeInclusive<i64> = 63800..=63999;

/// Set `mission_<id>_obj_<objective>_status`, the key
/// `Condition::ObjectiveStatus` reads.
fn with_objective(
    mut ctx: ExecutionContext,
    id: i32,
    objective: i32,
    status: &str,
) -> ExecutionContext {
    ctx.set_param(
        format!("mission_{id}_obj_{objective}_status"),
        serde_json::json!(status),
    );
    ctx
}

/// Replay `teleport_in` for ring region `region_id`. `ExecutionContext` is
/// not `Clone`, so this rebuilds the world id and params, as
/// `mission_1324::derive` does.
fn fire_teleport_in(
    engine: &ChainEngine,
    ctx: &ExecutionContext,
    region_id: i32,
) -> ResolvedActions {
    let mut out = ExecutionContext::new();
    out.world_id = ctx.world_id;
    out.params = ctx.params.clone();
    out.set_param("region_id".to_string(), serde_json::json!(region_id));
    let event = TriggerEvent {
        trigger_type: TriggerType::TeleportIn,
        source_entity: None,
        target_entity: None,
        params: out.params.clone(),
    };
    engine.resolve_event(&event, &out)
}

/// Render Omega's resolved actions as `"<chain>:<verb>(<args>)"`. An action
/// this list does not name renders as `OTHER`, so a stray verb shows up as a
/// diff rather than passing silently.
fn labels(resolved: &ResolvedActions) -> Vec<String> {
    resolved
        .actions
        .iter()
        .filter(|(id, _)| OMEGA_CHAIN_RANGE.contains(id))
        .map(|(id, action)| {
            let body = match action {
                Action::AcceptMission { mission_id } => format!("accept_mission({mission_id})"),
                Action::CompleteMission { mission_id } => format!("complete_mission({mission_id})"),
                Action::AdvanceStep {
                    mission_id,
                    step_id,
                } => {
                    format!("advance_step({mission_id}->{step_id})")
                }
                Action::CompleteObjective {
                    mission_id,
                    objective_id,
                } => {
                    format!("complete_objective({mission_id}/{objective_id})")
                }
                Action::AddDialogSet {
                    dialog_set_id,
                    slot,
                    ..
                } => {
                    format!("add_dialog_set({dialog_set_id}@{slot})")
                }
                Action::RemoveDialogSet {
                    dialog_set_id,
                    slot,
                } => {
                    format!("remove_dialog_set({dialog_set_id}@{slot})")
                }
                Action::GrantStargateAddress { stargate_id } => {
                    format!("grant_stargate_address({stargate_id})")
                }
                Action::RemoveItem { item_id, count } => format!("remove_item({item_id}x{count})"),
                other => format!("OTHER({other:?})"),
            };
            format!("{id}:{body}")
        })
        .collect()
}

/// A player who never reached Omega on 1655 resolves nothing Omega owns, in
/// either world, for any archetype, on every event an Omega chain keys, and
/// no dialog at all on the three keyed dialogs. Each later chain packet must
/// keep this green; its revert proof is in UO-04.
#[tokio::test]
async fn live_db_a_visitor_with_no_1655_resolves_no_omega_chain() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for archetype in [ARCH_HUMAN, ARCH_ASGARD, ARCH_JAFFA] {
        for (world, name) in [(WORLD_HUB, HUB_NAME), (WORLD_CMD, CMD_NAME)] {
            let ctx = ctx_for(archetype, Some(world));
            let ctx = with_mission(ctx, M_1655, "not_active");
            let ctx = with_step(ctx, M_1655, S_4940, "not_active");
            let ctx = with_step(ctx, M_1655, S_4942, "not_active");
            let ctx = with_objective(ctx, M_1655, O_5645, "active");
            let ctx = with_mission(ctx, M_1423, "not_active");
            let ctx = with_mission(ctx, M_1500, "not_active");
            let mut seen = labels(&fire_player_loaded(&engine, &ctx, name));
            seen.extend(labels(&fire_teleport_in(&engine, &ctx, REGION_CMD_PAD)));
            for dialog in [DLG_MODI, DLG_JACKSON, DLG_BRATAC] {
                let resolved = fire_dialog_choice(&engine, &ctx, dialog);
                assert_eq!(
                    display_dialog_count(&resolved),
                    0,
                    "dialog {dialog} must open nothing for a visitor"
                );
                seen.extend(labels(&resolved));
            }
            assert!(
                seen.is_empty(),
                "archetype {archetype} in world {world} resolved Omega actions: {seen:?}"
            );
        }
    }
}
