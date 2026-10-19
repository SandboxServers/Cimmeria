//! Dakara_E1 mission chain replays (campaign `docs/analysis/dakara-e1-rebuild/`).
//!
//! One submodule per seed file, `db/resources/Content/Seed/dakara_e1_m*_chains.sql`
//! (and `dakara_e1_bridge_chains.sql`), each pinning that file's chain ID range
//! (work-packets.md, C3). A packet adds its own file and its own submodule; it
//! does not edit a sibling's file. The helpers here are shared by every
//! submodule (work-packets.md, C7).

use std::ops::RangeInclusive;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

mod bridge;
mod entry;
mod m1645;
mod m1646_drops;
mod m1646_offer;
mod report;
mod search;
mod tent;

/// ARCHETYPE_Sholva, the Free Jaffa (char_defs 8 and 18).
const FREE_JAFFA: i64 = 7;
/// ARCHETYPE_Jaffa, the Praxis Jaffa: gets the Jaffa lines (D-DK15), never the entry.
const PRAXIS_JAFFA: i64 = 8;
/// A Tau'ri archetype (Scientist), for the `lte 6` branch.
const TAURI: i64 = 3;
const DAKARA: i32 = 61;
const STORY_RM: i32 = 62;
const DAKARA_NAME: &str = "Dakara_E1";
#[allow(dead_code)] // used by the fallback-B tests only (work-packets.md, DK-10a)
const STORY_RM_NAME: &str = "Dakara_E1_StoryRm";

/// One range per seed file (work-packets.md C3).
#[allow(dead_code)] // used by the wave-3 submodules (work-packets.md C7)
const BRIDGE_CHAINS: RangeInclusive<i64> = 8075..=8079;
const ENTRY_CHAINS: RangeInclusive<i64> = 8040..=8044;
#[allow(dead_code)] // used by the wave-3 submodules (work-packets.md C7)
const TENT_CHAINS: RangeInclusive<i64> = 8045..=8059;
#[allow(dead_code)] // used by the wave-3 submodules (work-packets.md C7)
const SEARCH_CHAINS: RangeInclusive<i64> = 8060..=8074;
#[allow(dead_code)] // used by the wave-3 submodules (work-packets.md C7)
const REPORT_CHAINS: RangeInclusive<i64> = 8160..=8179;
#[allow(dead_code)] // used by the wave-3 submodules (work-packets.md C7)
const M1645_CHAINS: RangeInclusive<i64> = 8080..=8119;
#[allow(dead_code)] // used by the wave-3 submodules (work-packets.md C7)
const M1646_OFFER_CHAINS: RangeInclusive<i64> = 8120..=8139;
#[allow(dead_code)] // used by the wave-3 submodules (work-packets.md C7)
const M1646_DROP_CHAINS: RangeInclusive<i64> = 8140..=8159;

/// Set `mission_<m>_obj_<o>_status`, the key `Condition::ObjectiveStatus` reads.
#[allow(dead_code)] // used by the wave-3 submodules (work-packets.md C7)
fn with_objective(
    mut ctx: ExecutionContext,
    mission: i32,
    objective: i32,
    status: &str,
) -> ExecutionContext {
    ctx.set_param(
        format!("mission_{mission}_obj_{objective}_status"),
        serde_json::json!(status),
    );
    ctx
}

/// Set `counter_<name>`, the key `Condition::Counter` reads (a missing
/// counter reads 0). Used by DK-10c and DK-12c.
#[allow(dead_code)] // used by the wave-3 submodules (work-packets.md C7)
fn with_counter(mut ctx: ExecutionContext, name: &str, n: i64) -> ExecutionContext {
    ctx.set_param(format!("counter_{name}"), serde_json::json!(n));
    ctx
}

/// Fire `enter_region` with `region_key` = `key`, as the cell does for a
/// client-hinted point set (`event_dispatch/region.rs`). Used by DK-10c.
#[allow(dead_code)] // used by the wave-3 submodules (work-packets.md C7)
fn fire_enter_region(engine: &ChainEngine, ctx: &ExecutionContext, key: &str) -> ResolvedActions {
    let mut c = ExecutionContext::new();
    c.world_id = ctx.world_id;
    c.params = ctx.params.clone();
    c.set_param("region_key".to_string(), serde_json::json!(key));
    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: None,
        target_entity: None,
        params: c.params.clone(),
    };
    engine.resolve_event(&event, &c)
}

/// `"<chain>:<verb>(<args>)"` labels for the chains in `range`, as
/// `mission_1324::labels` plus the item and bark verbs.
fn labels_in(resolved: &ResolvedActions, range: RangeInclusive<i64>) -> Vec<String> {
    resolved
        .actions
        .iter()
        .filter(|(id, _)| range.contains(id))
        .map(|(id, action)| {
            let body = match action {
                Action::AcceptMission { mission_id } => format!("accept_mission({mission_id})"),
                Action::CompleteMission { mission_id } => format!("complete_mission({mission_id})"),
                Action::AdvanceStep {
                    mission_id,
                    step_id,
                } => format!("advance_step({mission_id}->{step_id})"),
                Action::CompleteObjective {
                    mission_id,
                    objective_id,
                } => format!("complete_objective({mission_id}/{objective_id})"),
                Action::DisplayDialog { dialog_id } => format!("display_dialog({dialog_id})"),
                Action::AddDialogSet {
                    dialog_set_id,
                    slot,
                    ..
                } => format!("add_dialog_set({dialog_set_id}@{slot})"),
                Action::RemoveDialogSet {
                    dialog_set_id,
                    slot,
                } => format!("remove_dialog_set({dialog_set_id}@{slot})"),
                Action::GrantItem { item_id, count, .. } => format!("add_item({item_id}x{count})"),
                Action::RemoveItem { item_id, count } => format!("remove_item({item_id}x{count})"),
                Action::NpcBark { screen_id, .. } => format!("npc_bark({screen_id})"),
                Action::IncrementCounter {
                    counter_name,
                    amount,
                } => format!("increment_counter({counter_name}+{amount})"),
                Action::ResetCounter { counter_name } => format!("reset_counter({counter_name})"),
                other => format!("OTHER({other:?})"),
            };
            format!("{id}:{body}")
        })
        .collect()
}
