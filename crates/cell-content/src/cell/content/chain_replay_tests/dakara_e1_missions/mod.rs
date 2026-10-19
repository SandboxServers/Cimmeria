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
mod labels;
mod m1645;
mod m1646_drops;
mod m1646_offer;
mod m1648;
mod m1649_offer;
mod m1649_terminal;
mod m1650;
mod m1651;
mod m1651_gate;
mod m1652;
mod m1653;
mod m1654;
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
#[allow(dead_code)] // used by the wave-5 and wave-6 submodules (work-packets.md C7)
const M1648_CHAINS: RangeInclusive<i64> = 8180..=8199;
#[allow(dead_code)] // used by the wave-5 and wave-6 submodules (work-packets.md C7)
const M1649_OFFER_CHAINS: RangeInclusive<i64> = 8200..=8219;
#[allow(dead_code)] // used by the wave-5 and wave-6 submodules (work-packets.md C7)
const M1649_TERMINAL_CHAINS: RangeInclusive<i64> = 8220..=8249;
#[allow(dead_code)] // used by the wave-5 and wave-6 submodules (work-packets.md C7)
const M1650_CHAINS: RangeInclusive<i64> = 8250..=8289;
#[allow(dead_code)] // used by the wave-5 and wave-6 submodules (work-packets.md C7)
const M1651_CHAINS: RangeInclusive<i64> = 8290..=8309;
#[allow(dead_code)] // used by the wave-5 and wave-6 submodules (work-packets.md C7)
const M1651_GATE_CHAINS: RangeInclusive<i64> = 8310..=8319;
#[allow(dead_code)] // used by the wave-5 and wave-6 submodules (work-packets.md C7)
const M1652_CHAINS: RangeInclusive<i64> = 8320..=8349;
#[allow(dead_code)] // used by the wave-5 and wave-6 submodules (work-packets.md C7)
const M1653_CHAINS: RangeInclusive<i64> = 8350..=8389;
#[allow(dead_code)] // used by the wave-5 and wave-6 submodules (work-packets.md C7)
const M1654_CHAINS: RangeInclusive<i64> = 8430..=8449;

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

/// Fire `stargate_crossed` to `destination`, as the cell does when the
/// player steps through an open gate (`event_dispatch/stargate.rs`).
#[allow(dead_code)] // used by the wave-6c submodules (work-packets.md C7)
fn fire_stargate_crossed(
    engine: &ChainEngine,
    ctx: &ExecutionContext,
    destination: &str,
) -> ResolvedActions {
    let mut c = ExecutionContext::new();
    c.world_id = ctx.world_id;
    c.params = ctx.params.clone();
    c.set_param(
        "destination_world".to_string(),
        serde_json::json!(destination),
    );
    let event = TriggerEvent {
        trigger_type: TriggerType::StargateCrossed,
        source_entity: None,
        target_entity: None,
        params: c.params.clone(),
    };
    engine.resolve_event(&event, &c)
}

/// The actions a Livewire win runs: victory chain `id`, fired by id with no
/// condition check, as the minigame callback does (Castle 1357). Panics if
/// the chain is missing or carries a condition (a condition there is never read).
#[allow(dead_code)] // used by the wave-6c submodules (work-packets.md C7)
async fn fire_victory(pool: &sqlx::PgPool, id: i32) -> ResolvedActions {
    let chain = crate::cell::content::engine_loader::load_single_chain_for_test(pool, id)
        .await
        .expect("DB query for the victory chain must succeed")
        .unwrap_or_else(|| panic!("victory chain {id} is not seeded"));
    assert!(
        chain.conditions.is_empty(),
        "victory chain {id} has conditions, which are never evaluated: {:?}",
        chain.conditions
    );
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    let mut actions = Vec::new();
    let mut action_delays = Vec::new();
    for (action, delay) in engine.get_chain_actions(i64::from(id)) {
        actions.push((i64::from(id), action));
        action_delays.push(delay);
    }
    ResolvedActions {
        actions,
        action_delays,
        ..Default::default()
    }
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
                Action::DisplayRadioDialog { dialog_id } => {
                    format!("display_radio_dialog({dialog_id})")
                }
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
                Action::CrossWorldTeleport {
                    world_name,
                    position: p,
                } => format!(
                    "cross_world_teleport({world_name}@{:.2},{:.2},{:.2})",
                    p[0], p[1], p[2]
                ),
                Action::SpawnEntity {
                    template_id, tag, ..
                } => {
                    format!("spawn_entity({template_id}#{tag})")
                }
                Action::DespawnEntity { entity_tag } => format!("despawn_entity({entity_tag})"),
                Action::DestroyTaggedEntity { entity_tag } => {
                    format!("destroy_entity({entity_tag})")
                }
                Action::PlaySequence { sequence_id } => format!("play_sequence({sequence_id})"),
                Action::StartMinigame {
                    minigame_type,
                    difficulty,
                    on_victory_chains,
                } => format!(
                    "start_minigame({minigame_type},victory={on_victory_chains:?},d={difficulty})"
                ),
                Action::GrantStargateAddress { stargate_id } => {
                    format!("grant_stargate_address({stargate_id})")
                }
                other => format!("OTHER({other:?})"),
            };
            format!("{id}:{body}")
        })
        .collect()
}

/// The `delay_ms` of each action of `chain`, in order (`ResolvedActions`
/// keeps one delay per action, index for index).
#[allow(dead_code)] // used by the wave-5 and wave-6 submodules (work-packets.md C7)
fn delays_of(resolved: &ResolvedActions, chain: i64) -> Vec<i32> {
    resolved
        .actions
        .iter()
        .zip(resolved.action_delays.iter())
        .filter(|((id, _), _)| *id == chain)
        .map(|(_, d)| *d)
        .collect()
}

/// How many `display_radio_dialog` actions the whole engine resolved.
#[allow(dead_code)] // used by the wave-5 and wave-6 submodules (work-packets.md C7)
fn radio_count(resolved: &ResolvedActions) -> usize {
    resolved
        .actions
        .iter()
        .filter(|(_, a)| matches!(a, Action::DisplayRadioDialog { .. }))
        .count()
}
