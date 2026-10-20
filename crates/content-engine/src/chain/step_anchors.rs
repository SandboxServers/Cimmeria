//! What a mission step needs the player to reach (TG-MIS-03).
//!
//! When a step activates, the cell logs a `mission.step_anchor` row naming
//! the chains that can move that step forward and the world things those
//! chains wait on. Two stall shapes show up there before anyone stalls: a
//! step no chain advances (Frost's Letter step 4037, `advancers = 0`) and an
//! anchor whose entity was never spawned (#1341, a tag with no live entity).
//! This module is the engine half: a read-only scan of the registered chains.

use super::{Chain, ChainEngine};
use crate::actions::Action;
use crate::conditions::{ComparisonOp, Condition, StepStatusValue};
use crate::triggers::Trigger;

/// The chains that advance one mission step, and the world anchors their
/// triggers wait on. Every list is sorted and deduplicated.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct StepAnchors {
    /// Ids of the enabled chains that can move this step forward: gated on
    /// the step by a `step_status <mission>/<step> eq active` condition,
    /// **and** carrying an action that progresses the same mission
    /// (`advance_step`, `advance_mission`, `complete_objective` or
    /// `complete_mission` or `complete_mission_with_item`), **and** not held
    /// to another world by a `world`
    /// condition.
    pub advancers: Vec<i64>,
    /// Spawn tags the advancers' triggers name (`interact_tag`,
    /// `entity_dead_tag`, `entity_health_below`).
    pub tags: Vec<String>,
    /// NPC template names the advancers' triggers name (`interact_template`
    /// and the two flank triggers), matched against the entity's NPC name
    /// exactly as the interaction dispatch matches them.
    pub templates: Vec<String>,
    /// Dialog ids the advancers' triggers name (`dialog_open`,
    /// `dialog_choice`). Not entities, so the cell reports them unresolved.
    pub dialog_ids: Vec<i32>,
}

impl ChainEngine {
    /// Whether chain `chain_id` has at least one condition row. `false` for an unknown id.
    pub fn chain_has_conditions(&self, chain_id: i64) -> bool {
        self.chains_by_trigger
            .values()
            .flatten()
            .any(|c| c.id == chain_id && !c.conditions.is_empty())
    }

    /// The [`StepAnchors`] of `step_id` of `mission_id`, for a player in
    /// world `world_id` (`resources.worlds.world_id`; `None` when unknown).
    ///
    /// A scan of every registered chain, so call it on a step activation,
    /// never per tick. Only the `step_status` gate counts. A step that
    /// advances through `trigger_chain` from another chain, or through an
    /// objective-only gate (the castle cellblock chains 1141/1142), shows
    /// `advancers = 0` here although it can finish. `OnMissionStep` triggers
    /// are not counted either: the loader never builds one and nothing
    /// dispatches it.
    pub fn step_anchors(
        &self,
        mission_id: i32,
        step_id: i32,
        world_id: Option<i32>,
    ) -> StepAnchors {
        let mut out = StepAnchors::default();
        for chain in self.chains_by_trigger.values().flatten() {
            if !chain.enabled
                || !gates_on_step(chain, mission_id, step_id)
                || !progresses_mission(chain, mission_id)
                || !runs_in_world(chain, world_id)
            {
                continue;
            }
            out.advancers.push(chain.id);
            match &chain.trigger {
                Trigger::OnInteractTag { entity_tag }
                | Trigger::OnEntityHealthBelow { entity_tag, .. }
                | Trigger::OnEntityDeath {
                    entity_tag: Some(entity_tag),
                    ..
                } => out.tags.push(entity_tag.clone()),
                Trigger::OnInteractTemplate { template_name }
                | Trigger::OnNpcFlanked {
                    npc_template: Some(template_name),
                }
                | Trigger::OnPlayerFlankedNpc {
                    npc_template: Some(template_name),
                } => out.templates.push(template_name.clone()),
                Trigger::OnDialogOpen { dialog_id } | Trigger::OnDialogChoice { dialog_id } => {
                    out.dialog_ids.push(*dialog_id)
                }
                _ => {}
            }
        }
        for list in [&mut out.tags, &mut out.templates] {
            list.sort();
            list.dedup();
        }
        out.advancers.sort_unstable();
        out.advancers.dedup();
        out.dialog_ids.sort_unstable();
        out.dialog_ids.dedup();
        out
    }
}

/// Does `chain` run only while `step_id` of `mission_id` is the active step?
fn gates_on_step(chain: &Chain, mission_id: i32, step_id: i32) -> bool {
    chain.conditions.iter().any(|c| {
        matches!(
            c,
            Condition::StepStatus {
                mission_id: m,
                step_id: s,
                operator: ComparisonOp::Eq,
                expected_status: StepStatusValue::Active,
            } if *m == mission_id && *s == step_id
        )
    })
}

/// Does `chain` carry an action that moves `mission_id` forward?
fn progresses_mission(chain: &Chain, mission_id: i32) -> bool {
    chain.actions.iter().any(|a| match a {
        Action::AdvanceStep { mission_id: m, .. }
        | Action::AdvanceMission { mission_id: m }
        | Action::CompleteObjective { mission_id: m, .. }
        | Action::CompleteMission { mission_id: m }
        | Action::CompleteMissionWithItem { mission_id: m, .. } => *m == mission_id,
        _ => false,
    })
}

/// Can `chain` fire for a player in `world_id`? Every `world` condition must
/// hold, read as `Condition::World` reads it: fail-closed on an unknown world
/// and on any operator but `eq` / `neq`.
fn runs_in_world(chain: &Chain, world_id: Option<i32>) -> bool {
    chain.conditions.iter().all(|c| match c {
        Condition::World {
            operator,
            world_id: want,
        } => match (world_id, operator) {
            (Some(w), ComparisonOp::Eq) => w == *want,
            (Some(w), ComparisonOp::Neq) => w != *want,
            _ => false,
        },
        _ => true,
    })
}
